use std::{
    collections::{BTreeSet, HashMap, HashSet, VecDeque},
    io::Read,
    rc::Rc,
};

use crate::{
    atom::Atom,
    error::Error,
    grammar::ListsParser,
    ir::{
        Arguments, Backquote, FunctionDefinition, Location, Operator, Quote, Statement, Statements,
        TopLevelStatement, Value,
    },
    opcodes::{Arity, Immediate, OpCode, OpCodes},
    syscalls,
    vm::VirtualMachine,
};

//
// Context.
//

#[derive(Clone, Debug, Default)]
pub(crate) struct Context {
    arity: Arity,
    locals: HashMap<Box<str>, Vec<usize>>,
    stackn: usize,
    stream: Stream,
}

impl Context {
    fn new(arity: Arity) -> Self {
        Self {
            arity,
            locals: HashMap::default(),
            stackn: 0,
            stream: Stream::default(),
        }
    }

    fn track_arguments(&mut self, args: &Arguments) {
        self.track_arguments_and_closure(args, &BTreeSet::new());
    }

    fn track_arguments_and_closure(&mut self, args: &Arguments, closure: &BTreeSet<Box<str>>) {
        //
        // Track the arguments.
        //
        args.iter().rev().for_each(|v| {
            self.locals.entry(v.clone()).or_default().push(self.stackn);
            self.stackn += 1;
        });
        //
        // Track the closure.
        //
        closure.iter().for_each(|v| {
            self.locals.entry(v.clone()).or_default().push(self.stackn);
            self.stackn += 1;
        });
    }

    #[cfg(test)]
    pub(crate) fn stream(&self) -> &Stream {
        &self.stream
    }
}

//
// OpCode or reference.
//

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LabelOrOpCode {
    Branch(Box<str>),
    BranchIfNot(Box<str>),
    Funcall(Box<str>),
    Get(Box<str>),
    OpCode(OpCode),
}

impl From<OpCode> for LabelOrOpCode {
    fn from(value: OpCode) -> Self {
        Self::OpCode(value)
    }
}

//
// Stream.
//

type Stream = VecDeque<LabelOrOpCode>;

//
// Symbols and OpCodes.
//

pub type SymbolsAndOpCodes = (Vec<(Box<str>, usize, Arity)>, OpCodes);

//
// Compiler trait.
//

pub trait CompilerTrait: Clone {
    fn eval(self, atom: Rc<Atom>) -> Result<Rc<Atom>, Error>;
    fn load(&mut self, atom: Rc<Atom>) -> Result<(), Error>;
    fn compile(self) -> Result<SymbolsAndOpCodes, Error>;
}

//
// Compiler.
//

#[derive(Clone, Default)]
pub struct Compiler {
    blocks: Vec<(Box<str>, Context)>,
    defuns: HashMap<Box<str>, Arity>,
    macros: HashSet<Box<str>>,
    labels: HashMap<Box<str>, usize>,
    lcount: usize,
}

impl Compiler {
    fn label(&mut self, prefix: &str) -> Box<str> {
        let label = format!("{prefix}_{:04}", self.lcount).into_boxed_str();
        self.lcount += 1;
        label
    }
}

//
// Compiler trait.
//

impl CompilerTrait for Compiler {
    fn eval(mut self, atom: Rc<Atom>) -> Result<Rc<Atom>, Error> {
        //
        // Convert the atom into a statement.
        //
        let stmt = Statement::from_atom(atom.clone(), &self.macros)?;
        //
        // Wrap the statement in a main function definition.
        //
        let stmts = Statements::new(vec![stmt]);
        let fdef = FunctionDefinition::new("__eval__".into(), Arguments::None, stmts);
        let topl = TopLevelStatement::Function(atom, fdef);
        //
        // Load the top-level statement.
        //
        self.load_statement(topl)?;
        //
        // Generate the bytecode.
        //
        let (syms, ops) = self.compile()?;
        //
        // Build the virtual machine.
        //
        let mut vm = VirtualMachine::new("__eval__", 1024, false);
        //
        // Run.
        //
        let result = vm.run(syms, ops)?;
        //
        // Convert the result into an atom.
        //
        result.try_into()
    }

    fn load(&mut self, atom: Rc<Atom>) -> Result<(), Error> {
        let stmt = TopLevelStatement::from_atom(atom, &self.macros)?;
        self.load_statement(stmt)
    }

    fn compile(self) -> Result<SymbolsAndOpCodes, Error> {
        //
        // Collect the live defuns.
        //
        let live_defuns = self.collect_live_defuns()?;
        //
        // Serialize the live streams.
        //
        let (index, stream) = self
            .blocks
            .into_iter()
            .filter(|(k, _)| {
                live_defuns
                    .as_ref()
                    .map(|v| v.contains(k.as_ref()))
                    .unwrap_or(true)
            })
            .fold(
                (Vec::new(), Vec::new()),
                |(mut offsets, mut opcodes), (name, ctxt)| {
                    offsets.push((name, opcodes.len(), ctxt.arity));
                    opcodes.extend(ctxt.stream);
                    (offsets, opcodes)
                },
            );
        //
        // Convert the stream to opcodes.
        //
        let opcodes = stream
            .into_iter()
            .map(|v| match v {
                LabelOrOpCode::Branch(v) => {
                    let delta = self.labels.get(&v).copied().ok_or(Error::InvalidLabel(v))?;
                    Ok(OpCode::Br(delta as isize))
                }
                LabelOrOpCode::BranchIfNot(v) => {
                    let delta = self.labels.get(&v).copied().ok_or(Error::InvalidLabel(v))?;
                    Ok(OpCode::Brn(delta as isize))
                }
                LabelOrOpCode::Funcall(sym) => {
                    //
                    // Get the address of the symbol.
                    //
                    let (addr, argcnt) = index
                        .iter()
                        .find_map(|(k, a, n)| (k == &sym).then_some((*a, *n)))
                        .ok_or(Error::InvalidSymbol(sym))?;
                    //
                    // Push the funcall.
                    //
                    Ok(OpCode::Psh(Immediate::funcall(addr, argcnt)))
                }
                LabelOrOpCode::Get(sym) => {
                    //
                    // Get the address of the symbol.
                    //
                    let (addr, argcnt) = index
                        .iter()
                        .find_map(|(k, a, n)| (k == &sym).then_some((*a, *n)))
                        .ok_or(Error::UnresolvedSymbol(sym))?;
                    //
                    // Push the funcall.
                    //
                    Ok(OpCode::Psh(Immediate::funcall(addr, argcnt)))
                }
                LabelOrOpCode::OpCode(v) => Ok(v),
            })
            .collect::<Result<_, Error>>()?;
        //
        // Done.
        //
        Ok((index, opcodes))
    }
}

//
// Live defuns.
//

impl Compiler {
    fn collect_live_defuns_for_context(
        &self,
        name: &str,
        ctxt: &Context,
        index: &mut HashSet<String>,
    ) -> Result<(), Error> {
        //
        // Track the funcall.
        //
        index.insert(name.to_owned());
        //
        // Collect the funcalls.
        //
        let funcalls: HashSet<_> = ctxt
            .stream
            .iter()
            .filter_map(|v| match v {
                LabelOrOpCode::Funcall(v) => Some(v.to_owned()),
                LabelOrOpCode::Get(v) => Some(v.to_owned()),
                _ => None,
            })
            .filter(|v| !index.contains(v.as_ref()))
            .collect();
        //
        // Process the funcalls.
        //
        funcalls.into_iter().try_for_each(|v| {
            //
            // Grab the block.
            //
            let Some((_, ctxt)) = self.blocks.iter().find(|(k, _)| k == &v) else {
                return Err(Error::UnresolvedSymbol(v));
            };
            //
            // Process the block.
            //
            self.collect_live_defuns_for_context(v.as_ref(), ctxt, index)
        })
    }

    fn collect_live_defuns(&self) -> Result<Option<HashSet<String>>, Error> {
        let mut result = HashSet::new();
        //
        // Grab the main block.
        //
        let Some((name, ctxt)) = self.blocks.iter().find(|(k, _)| k.as_ref() == "main") else {
            return Ok(None);
        };
        //
        // Collect the funcalls.
        //
        self.collect_live_defuns_for_context(name.as_ref(), ctxt, &mut result)?;
        //
        // Done.
        //
        Ok(Some(result))
    }
}

//
// Statement loading.
//

impl Compiler {
    fn load_statement(&mut self, stmt: TopLevelStatement) -> Result<(), Error> {
        match stmt {
            TopLevelStatement::Function(_, v) => self.compile_function(v, false),
            TopLevelStatement::Macro(_, v) => self.compile_function(v, true),
            TopLevelStatement::Use(_, v) => self.load_modules(v),
        }
    }
}

//
// Module loading.
//

impl Compiler {
    fn load_modules(&mut self, stmts: Statements) -> Result<(), Error> {
        stmts.iter().try_for_each(|v| {
            //
            // Grab the quote.
            //
            let Statement::Quote(_, quote) = v else {
                return Err(Error::ExpectedQuote);
            };
            //
            // Process the quote.
            //
            match quote {
                Quote::Pair(car, cdr) => {
                    //
                    // Split the quote
                    //
                    // Get the name of the module.
                    //
                    let Quote::Symbol(name) = car.as_ref() else {
                        return Err(Error::ExpectedSymbol);
                    };
                    //
                    // Collect the names of the items.
                    //
                    let items: Vec<_> = cdr
                        .iter()
                        .map(|v| match v {
                            Quote::Symbol(v) => Ok(v.as_ref()),
                            _ => Err(Error::ExpectedSymbol),
                        })
                        .collect::<Result<_, _>>()?;
                    self.load_module(name, Some(&items))
                }
                Quote::Symbol(v) => self.load_module(v, None),
                _ => Err(Error::ExpectedPairOrSymbol),
            }
        })
    }

    fn load_module(&mut self, name: &str, _items: Option<&[&str]>) -> Result<(), Error> {
        //
        // Compute the source path.
        //
        let root = std::env::var("SLISP_LIBRARY_PATH")?;
        let path = format!("{root}/{name}.sl");
        //
        // Open the source file.
        //
        let mut source = String::new();
        let mut file = std::fs::File::open(path)?;
        file.read_to_string(&mut source)?;
        //
        // Create a temporary compiler for the source file.
        //
        let mut compiler = Self::default();
        compiler.lift_operators()?;
        //
        // Parse the source file.
        //
        let parser = ListsParser::new();
        let atoms = parser
            .parse(&mut compiler, &source)
            .map_err(|v| Error::Parse(v.to_string()))?;
        //
        // Rewrite the atoms using our intermediate representation.
        //
        let mut stmts: Vec<_> = atoms
            .into_iter()
            .map(|v| TopLevelStatement::from_atom(v, &self.macros))
            .collect::<Result<_, _>>()?;
        //
        // Filter function declarations.
        //
        if let Some(items) = _items {
            stmts.retain(|v| match v {
                TopLevelStatement::Function(_, v) | TopLevelStatement::Macro(_, v) => {
                    items.contains(&v.name().as_ref())
                }
                TopLevelStatement::Use(..) => true,
            });
        }
        //
        // Process the statements.
        //
        stmts.into_iter().try_for_each(|v| self.load_statement(v))
    }
}

//
// Compilation.
//

impl Compiler {
    fn compile_function(&mut self, defun: FunctionDefinition, is_macro: bool) -> Result<(), Error> {
        let arity = defun.arguments().arity();
        let argcnt = defun.arguments().len();
        let mut ctxt = Context::new(arity);
        //
        // Make sure the function does not exist.
        //
        if self.defuns.contains_key(defun.name()) {
            let name = defun.name().to_string().into_boxed_str();
            return Err(Error::FunctionAlreadyDefined(name));
        }
        //
        // Track the function arguments.
        //
        ctxt.track_arguments(defun.arguments());
        //
        // Track the function definition.
        //
        self.defuns.insert(defun.name().clone(), arity);
        //
        // Rotate the arguments and the return address.
        //
        if argcnt > 0 {
            ctxt.stream.push_back(OpCode::Rot(argcnt + 1).into());
        }
        //
        // Compile the statements.
        //
        self.compile_statements(&mut ctxt, defun.statements())?;
        //
        // Generate the postamble if necessary.
        //
        if !defun.statements().is_tail_call() {
            //
            // Pop the arguments.
            //
            if argcnt > 0 {
                ctxt.stream.push_back(OpCode::Rot(argcnt + 1).into());
                ctxt.stream.push_back(OpCode::Pop(argcnt).into());
            }
            //
            // Inject the return call.
            //
            ctxt.stream.push_back(OpCode::Ret.into());
            ctxt.stackn -= 1;
        }
        //
        // Save the block.
        //
        self.blocks.push((defun.name().clone(), ctxt));
        //
        // Optionally mark the function as a macro.
        //
        if is_macro {
            self.macros.insert(defun.name().clone());
        }
        //
        // Done.
        //
        Ok(())
    }

    fn compile_arguments(&mut self, ctxt: &mut Context, stmts: &Statements) -> Result<(), Error> {
        stmts
            .iter()
            .rev()
            .try_for_each(|v| self.compile_statement(ctxt, v))
    }

    fn compile_bindings(
        &mut self,
        ctxt: &mut Context,
        bindings: &[(Box<str>, Statement)],
    ) -> Result<(), Error> {
        bindings.iter().try_for_each(|(symbol, stmt)| {
            //
            // Compile the statement.
            //
            self.compile_statement(ctxt, stmt)?;
            //
            // Save the symbol's index.
            //
            ctxt.locals
                .entry(symbol.clone())
                .or_default()
                .push(ctxt.stackn - 1);
            //
            // Done.
            //
            Ok(())
        })
    }

    fn clear_bindings(
        &mut self,
        ctxt: &mut Context,
        bindings: &[(Box<str>, Statement)],
    ) -> Result<(), Error> {
        bindings.iter().try_for_each(|(symbol, _)| {
            //
            // Clear the binding.
            //
            if let Some(v) = ctxt.locals.get_mut(symbol) {
                //
                // Remove the top reference.
                //
                v.pop();
                //
                // Clear the entry if there is no more references.
                //
                if v.is_empty() {
                    ctxt.locals.remove(symbol);
                }
            }
            //
            // Done.
            //
            Ok(())
        })
    }

    fn compile_statements(&mut self, ctxt: &mut Context, stmts: &Statements) -> Result<(), Error> {
        stmts.iter().enumerate().try_for_each(|(indx, stmt)| {
            //
            // Drop the previous result.
            //
            if indx > 0 {
                ctxt.stream.push_back(OpCode::Pop(1).into());
                ctxt.stackn -= 1;
            }
            //
            // Compile the statement.
            //
            self.compile_statement(ctxt, stmt)
        })
    }

    pub(crate) fn compile_statement(
        &mut self,
        ctxt: &mut Context,
        stmt: &Statement,
    ) -> Result<(), Error> {
        match stmt {
            Statement::Apply(_, op, args, Location::Any) => {
                //
                // Compile the arguments.
                //
                self.compile_arguments(ctxt, args)?;
                //
                // Compile the operator.
                //
                self.compile_statement(ctxt, op)?;
                //
                // If the operator is not internal, generate a call.
                //
                match op.as_ref() {
                    Statement::Operator(..) => (),
                    _ => ctxt.stream.push_back(OpCode::Call(args.len()).into()),
                }
                //
                // Update the stack.
                //
                ctxt.stackn -= args.len();
                //
                // Done.
                //
                Ok(())
            }
            Statement::Apply(_, op, args, Location::Tail) => {
                //
                // Get the symbol of the tail call.
                //
                let Statement::Symbol(_, symbol) = op.as_ref() else {
                    return Err(Error::ExpectedSymbol);
                };
                //
                // Compile the arguments.
                //
                self.compile_arguments(ctxt, args)?;
                //
                // Grab the argument count.
                //
                let arity = self.defuns.get(symbol).copied().unwrap();
                //
                // Pack the arguments depending on the arity.
                //
                let argcnt = match arity {
                    Arity::All => {
                        /*
                         * Pack the arguments into a list.
                         */
                        ctxt.stream.push_back(OpCode::Lst(args.len()).into());
                        /*
                         * Pop the argument.
                         */
                        ctxt.stream.push_back(OpCode::Rot(2).into());
                        ctxt.stream.push_back(OpCode::Pop(1).into());
                        /*
                         * Update the stack.
                         */
                        ctxt.stackn -= args.len();
                        ctxt.stackn += 1;
                        /*
                         * Return the argument count.
                         */
                        1
                    }
                    Arity::Some(n) if n == 1 => {
                        let cnt = n as usize;
                        /*
                         * Rotate the stack to push the original arguments ahead.
                         */
                        ctxt.stream.push_back(OpCode::Rot(2 * cnt).into());
                        /*
                         * Pop the old arguments.
                         */
                        ctxt.stream.push_back(OpCode::Pop(cnt).into());
                        /*
                         * Return the argument count.
                         */
                        cnt
                    }
                    Arity::Some(n) => {
                        let cnt = n as usize;
                        /*
                         * Rotate the stack to push the original arguments ahead.
                         */
                        ctxt.stream.push_back(OpCode::Rtm(2 * cnt, cnt).into());
                        /*
                         * Pop the old arguments.
                         */
                        ctxt.stream.push_back(OpCode::Pop(cnt).into());
                        /*
                         * Return the argument count.
                         */
                        cnt
                    }
                    Arity::SomeWithRem(n) if args.len() - n as usize == 0 => {
                        let cnt = n as usize + 1;
                        /*
                         * Push an empty list.
                         */
                        ctxt.stream.push_back(OpCode::Psh(Immediate::Nil).into());
                        /*
                         * Restore the new arguments order.
                         */
                        ctxt.stream.push_back(OpCode::Rot(cnt).into());
                        /*
                         * Rotate the stack to push the original arguments ahead.
                         */
                        ctxt.stream.push_back(OpCode::Rot(2 * cnt).into());
                        /*
                         * Pop the old arguments.
                         */
                        ctxt.stream.push_back(OpCode::Pop(cnt).into());
                        /*
                         * Return the argument count.
                         */
                        cnt
                    }
                    Arity::SomeWithRem(n) => {
                        let cnt = n as usize + 1;
                        let rem = args.len() - n as usize;
                        /*
                         * Prepare the new arguments.
                         */
                        ctxt.stream.push_back(OpCode::Rtm(args.len(), rem).into());
                        /*
                         * Pack the remainder arguments into a list.
                         */
                        ctxt.stream.push_back(OpCode::Lst(rem).into());
                        /*
                         * Restore the new arguments order.
                         */
                        ctxt.stream.push_back(OpCode::Rot(cnt).into());
                        /*
                         * Rotate the stack to push the original arguments ahead.
                         */
                        ctxt.stream.push_back(OpCode::Rtm(2 * cnt, cnt).into());
                        /*
                         * Pop the old arguments.
                         */
                        ctxt.stream.push_back(OpCode::Pop(cnt).into());
                        /*
                         * Return the argument count.
                         */
                        cnt
                    }
                    Arity::None => 0,
                };
                //
                // Compute the offset of the branch.
                //
                let offset = ctxt.stream.len() - (argcnt > 0) as usize;
                //
                // Generate the branch.
                //
                ctxt.stream.push_back(OpCode::Br(-(offset as isize)).into());
                //
                // Done.
                //
                Ok(())
            }
            Statement::Expand(_, name, args) => {
                //
                // Clone the compiler state and unmark the macro.
                //
                let mut comp = self.clone();
                comp.macros.remove(name);
                //
                // Quote the arguments.
                //
                let args = args.iter().rev().fold(Atom::nil(), |acc, v| {
                    Atom::cons(Atom::cons(Atom::symbol("quote"), v.atom()), acc)
                });
                //
                // Evaluate the macro.
                //
                let expr = Atom::cons(Atom::symbol(name), args);
                let expn = comp.eval(expr)?;
                //
                // Check if the result is valid.
                //
                if expn.is_nil() {
                    return Err(Error::MacroExpansion(name.clone()));
                }
                //
                // Compile the result.
                //
                let stmt = Statement::from_atom(expn, &self.macros)?;
                self.compile_statement(ctxt, &stmt)
            }
            Statement::Lambda(_, args, statements) => {
                let mut next = Context::new(args.arity());
                //
                // Generate a name for the lambda.
                //
                let name = self.label("LAMBDA");
                //
                // Grab the closure.
                //
                let mut closure = stmt.closure();
                //
                // Remove the global symbols from the closure.
                //
                self.defuns.keys().for_each(|v| {
                    closure.remove(v);
                });
                //
                // Compute the preamble depth.
                //
                let argcnt = args.len() + closure.len();
                //
                // Track the function arguments.
                //
                next.track_arguments_and_closure(args, &closure);
                //
                // Compile the statements.
                //
                self.compile_statements(&mut next, statements)?;
                //
                // Generate the preamble and the postamble.
                //
                if argcnt > 0 {
                    //
                    // Generate the preamble.
                    //
                    next.stream.push_front(OpCode::Rot(argcnt + 1).into());
                    //
                    // Generate the postamble.
                    //
                    if argcnt > 0 {
                        next.stream.push_back(OpCode::Rot(argcnt + 1).into());
                        next.stream.push_back(OpCode::Pop(argcnt).into());
                    }
                }
                //
                // Inject the return call.
                //
                next.stream.push_back(OpCode::Ret.into());
                next.stackn -= 1;
                //
                // Grab the closure symbols.
                //
                closure.iter().try_for_each(|v| {
                    //
                    // Get the symbol index.
                    //
                    let Some(index) = ctxt.locals.get(v).and_then(|v| v.last()) else {
                        return Err(Error::InvalidSymbol(v.clone()));
                    };
                    //
                    // Inject the push.
                    //
                    let opcode = OpCode::Get(ctxt.stackn - *index).into();
                    ctxt.stream.push_back(opcode);
                    ctxt.stackn += 1;
                    //
                    // Done.
                    //
                    Ok(())
                })?;
                //
                // Inject the funcall.
                //
                ctxt.stream.push_back(LabelOrOpCode::Funcall(name.clone()));
                ctxt.stackn += 1;
                //
                // Pack the closure.
                //
                ctxt.stream.push_back(OpCode::Pak(closure.len() + 1).into());
                ctxt.stackn -= closure.len();
                //
                // Save the block.
                //
                self.blocks.push((name, next));
                //
                // Done.
                //
                Ok(())
            }
            Statement::Operator(_, v) => {
                //
                // Get the opcode for the operator.
                //
                let opcode = match v {
                    //
                    // Arithmetics.
                    //
                    Operator::Add => OpCode::Add.into(),
                    Operator::Ge => OpCode::Ge.into(),
                    Operator::Gt => OpCode::Gt.into(),
                    Operator::Le => OpCode::Le.into(),
                    Operator::Lt => OpCode::Lt.into(),
                    Operator::Sub => OpCode::Sub.into(),
                    //
                    // Logic.
                    //
                    Operator::And => OpCode::And.into(),
                    Operator::Equ => OpCode::Equ.into(),
                    Operator::Neq => OpCode::Neq.into(),
                    Operator::Not => OpCode::Not.into(),
                    Operator::Or => OpCode::Or.into(),
                    //
                    // List.
                    //
                    Operator::Car => OpCode::Car.into(),
                    Operator::Cdr => OpCode::Cdr.into(),
                    Operator::Cons => OpCode::Cons.into(),
                    //
                    // String.
                    //
                    Operator::Str => OpCode::Str.into(),
                    //
                    // Predicates.
                    //
                    Operator::IsChr => OpCode::IsChr.into(),
                    Operator::IsNum => OpCode::IsNum.into(),
                    Operator::IsLst => OpCode::IsLst.into(),
                    Operator::IsNil => OpCode::IsNil.into(),
                    Operator::IsSym => OpCode::IsSym.into(),
                    Operator::IsTru => OpCode::IsTru.into(),
                    Operator::IsWld => OpCode::IsWld.into(),
                };
                //
                // Push the opcode.
                //
                ctxt.stream.push_back(opcode);
                //
                // Update the stack.
                //
                ctxt.stackn += 1;
                //
                // Done.
                //
                Ok(())
            }
            Statement::SysCall(_, sym) => {
                //
                // Get the syscall parameters.
                //
                let (index, argcnt) = syscalls::get(sym)?;
                //
                // Push the opcode.
                //
                let syscall = Immediate::Syscall(index, argcnt);
                ctxt.stream.push_back(OpCode::Psh(syscall).into());
                //
                // Update the stack.
                //
                ctxt.stackn += 1;
                //
                // Done.
                //
                Ok(())
            }
            Statement::IfThenElse(_, cond, then, else_) => {
                //
                // Compile the condition.
                //
                self.compile_statement(ctxt, cond)?;
                ctxt.stackn -= 1;
                //
                // Generate the branch to the else block.
                //
                let name = self.label("BEGIN_ELSE");
                let start = ctxt.stream.len();
                let label = LabelOrOpCode::BranchIfNot(name.clone());
                ctxt.stream.push_back(label);
                //
                // Compile THEN.
                //
                self.compile_statement(ctxt, then)?;
                //
                // Ignore the stack value for the THEN branch.
                //
                ctxt.stackn -= 1;
                //
                // Track the label.
                //
                let delta = (ctxt.stream.len() - start) + 1;
                self.labels.insert(name, delta);
                //
                // Build the label past the else block.
                //
                let name = self.label("END_ELSE");
                let start = ctxt.stream.len();
                //
                // Generate the branch past the else block if necessary.
                //
                if !then.is_tail_call() {
                    let label = LabelOrOpCode::Branch(name.clone());
                    ctxt.stream.push_back(label);
                }
                //
                // Compile ELSE.
                //
                match else_ {
                    Some(else_) => self.compile_statement(ctxt, else_)?,
                    None => {
                        let value = Statement::Value(Atom::nil(), Value::Nil);
                        self.compile_statement(ctxt, &value)?
                    }
                }
                //
                // Track the label.
                //
                let delta = ctxt.stream.len() - start;
                self.labels.insert(name, delta);
                //
                // Done.
                //
                Ok(())
            }
            Statement::Let(_, bindings, statements) => {
                let argcnt = bindings.len();
                //
                // Compile the bindings.
                //
                self.compile_bindings(ctxt, bindings)?;
                //
                // Compile the statements.
                //
                self.compile_statements(ctxt, statements)?;
                //
                // Pop the bindings.
                //
                if argcnt > 0 {
                    ctxt.stream.push_back(OpCode::Rot(argcnt + 1).into());
                    ctxt.stream.push_back(OpCode::Pop(argcnt).into());
                }
                //
                // Clear the bindings from the locals.
                //
                self.clear_bindings(ctxt, bindings)?;
                //
                // Done.
                //
                Ok(())
            }
            Statement::Prog(_, stmts) => self.compile_statements(ctxt, stmts),
            Statement::Backquote(_, quote) => self.compile_backquote(ctxt, quote),
            Statement::Quote(_, quote) => Self::compile_quote(ctxt, quote),
            Statement::Symbol(_, symbol) => self.compile_symbol(ctxt, symbol),
            Statement::Value(_, value) => Self::compile_value(ctxt, value),
        }
    }

    fn compile_backquote(&mut self, ctxt: &mut Context, quote: &Backquote) -> Result<(), Error> {
        match quote {
            Backquote::Pair(car, cdr) => {
                //
                // Process car and cdr.
                //
                self.compile_backquote(ctxt, cdr.as_ref())?;
                self.compile_backquote(ctxt, car.as_ref())?;
                //
                // Push cons, consume 2 and producing 1.
                //
                let opcode = OpCode::Cons.into();
                //
                // Push the opcode.
                //
                ctxt.stream.push_back(opcode);
                //
                // Update the stack.
                //
                ctxt.stackn -= 1;
                //
                // Done.
                //
                Ok(())
            }
            Backquote::Symbol(v) => {
                //
                // Convert the symbol.
                //
                let mut symbol = [0_u8; 15];
                symbol[..v.len()].copy_from_slice(v.as_bytes());
                //
                // Push the opcode.
                //
                let opcode = OpCode::Psh(Immediate::Symbol(symbol)).into();
                ctxt.stream.push_back(opcode);
                //
                // Update the stack.
                //
                ctxt.stackn += 1;
                //
                // Done.
                //
                Ok(())
            }
            Backquote::Unquote(stmt) => self.compile_statement(ctxt, stmt),
            Backquote::Value(value) => Self::compile_value(ctxt, value),
        }
    }

    fn compile_quote(ctxt: &mut Context, quote: &Quote) -> Result<(), Error> {
        match quote {
            Quote::Pair(car, cdr) => {
                //
                // Process car and cdr.
                //
                Self::compile_quote(ctxt, cdr.as_ref())?;
                Self::compile_quote(ctxt, car.as_ref())?;
                //
                // Push cons, consume 2 and producing 1.
                //
                let opcode = OpCode::Cons.into();
                //
                // Push the opcode.
                //
                ctxt.stream.push_back(opcode);
                //
                // Update the stack.
                //
                ctxt.stackn -= 1;
                //
                // Done.
                //
                Ok(())
            }
            Quote::Symbol(v) => {
                //
                // Convert the symbol.
                //
                let mut symbol = [0_u8; 15];
                symbol[..v.len()].copy_from_slice(v.as_bytes());
                //
                // Push the opcode.
                //
                let opcode = OpCode::Psh(Immediate::Symbol(symbol)).into();
                ctxt.stream.push_back(opcode);
                //
                // Update the stack.
                //
                ctxt.stackn += 1;
                //
                // Done.
                //
                Ok(())
            }
            Quote::Value(value) => Self::compile_value(ctxt, value),
        }
    }

    fn compile_symbol(&mut self, ctxt: &mut Context, symbol: &Box<str>) -> Result<(), Error> {
        //
        // Get the opcode.
        //
        let opcode = match ctxt.locals.get(symbol).and_then(|v| v.last()) {
            Some(index) => OpCode::Get(ctxt.stackn - *index).into(),
            None => LabelOrOpCode::Get(symbol.clone()),
        };
        //
        // Push the opcode.
        //
        ctxt.stream.push_back(opcode);
        //
        // Update the stack tracker.
        //
        ctxt.stackn += 1;
        //
        // Done.
        //
        Ok(())
    }

    fn compile_value(ctxt: &mut Context, value: &Value) -> Result<(), Error> {
        //
        // Get the opcode.
        //
        let opcode = match value {
            Value::Nil => OpCode::Psh(Immediate::Nil).into(),
            Value::True => OpCode::Psh(Immediate::True).into(),
            Value::Char(v) => OpCode::Psh(Immediate::Char(*v)).into(),
            Value::Number(v) => OpCode::Psh(Immediate::Number(*v)).into(),
            Value::Wildcard => OpCode::Psh(Immediate::Wildcard).into(),
        };
        //
        // Push the opcode.
        //
        ctxt.stream.push_back(opcode);
        //
        // Update the stack tracker.
        //
        ctxt.stackn += 1;
        //
        // Done.
        //
        Ok(())
    }
}

//
// Lifters.
//

impl Compiler {
    pub fn lift_operators(&mut self) -> Result<(), Error> {
        //
        // Define the top-level statements for the operators.
        //
        let stmts = vec![
            //
            // Arithmetics.
            //
            Self::lift(Operator::Add),
            Self::lift(Operator::Sub),
            Self::lift(Operator::Ge),
            Self::lift(Operator::Gt),
            Self::lift(Operator::Le),
            Self::lift(Operator::Lt),
            //
            // Logic.
            //
            Self::lift(Operator::And),
            Self::lift(Operator::Equ),
            Self::lift(Operator::Neq),
            Self::lift(Operator::Not),
            Self::lift(Operator::Or),
            //
            // List.
            //
            Self::lift(Operator::Car),
            Self::lift(Operator::Cdr),
            Self::lift(Operator::Cons),
            //
            // Predicates.
            //
            Self::lift(Operator::IsChr),
            Self::lift(Operator::IsNum),
            Self::lift(Operator::IsLst),
            Self::lift(Operator::IsNil),
            Self::lift(Operator::IsSym),
            Self::lift(Operator::IsTru),
            Self::lift(Operator::IsWld),
        ];
        //
        // Compile the statements.
        //
        stmts.into_iter().try_for_each(|v| self.load_statement(v))
    }

    fn lift(op: Operator) -> TopLevelStatement {
        let opname = op.to_string();
        let argcnt = op.arity();
        let macros = HashSet::default();
        //
        // Build the argument list.
        //
        let args = (0..argcnt).rev().fold(Atom::nil(), |acc, v| {
            let name = format!("_{v}");
            let symb = Atom::symbol(name.as_str());
            Atom::cons(symb, acc)
        });
        //
        // Build the apply statement.
        //
        let apply = Atom::cons(Atom::symbol(opname.as_str()), args.clone());
        //
        // Build the atom for the operator.
        //
        let atom = Atom::cons(
            Atom::symbol(opname.as_str()),
            Atom::cons(args, Atom::cons(apply, Atom::nil())),
        );
        //
        // Build the function definition.
        //
        let defun = FunctionDefinition::from_atom(atom.clone(), &macros).unwrap();
        //
        // Done.
        //
        TopLevelStatement::Function(atom, defun)
    }
}
