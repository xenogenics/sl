use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

use crate::{
    atom::Atom,
    error::Error,
    opcodes::{Immediate, OpCode, OpCodes},
};

//
// Context.
//

#[derive(Default, Debug)]
pub(crate) struct Context {
    locals: HashMap<Box<str>, Vec<usize>>,
    stackn: usize,
    stream: Stream,
}

impl Context {
    fn track(&mut self, atom: Rc<Atom>) -> Result<usize, Error> {
        atom.iter().enumerate().try_fold(0, |acc, (_, atom)| {
            //
            // Make sure the argument is a symbol.
            //
            let Atom::Symbol(v) = atom.as_ref() else {
                return Err(Error::ExpectedSymbol);
            };
            //
            // Update the argument map.
            //
            self.locals
                .entry(v.to_owned())
                .or_default()
                .push(self.stackn);
            //
            // Update the stack position.
            //
            self.stackn += 1;
            //
            // Done.
            //
            Ok(acc + 1)
        })
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

type Stream = Vec<LabelOrOpCode>;

//
// Compiler.
//

#[derive(Default)]
pub struct Compiler {
    blocks: Vec<(Box<str>, Context)>,
    defuns: HashSet<Box<str>>,
    labels: HashMap<Box<str>, usize>,
    lcount: usize,
}

impl Compiler {
    pub fn compile(
        mut self,
        stmt: Vec<Rc<Atom>>,
    ) -> Result<(Vec<(Box<str>, usize)>, OpCodes), Error> {
        //
        // Compile the statements.
        //
        stmt.into_iter()
            .map(|v| self.compile_defun(v))
            .collect::<Result<(), _>>()?;
        //
        // Serialize the streams.
        //
        let (index, stream) = self.blocks.into_iter().fold(
            (Vec::new(), Vec::new()),
            |(mut offsets, mut opcodes), (name, ctxt)| {
                offsets.push((name, opcodes.len()));
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
                    let address = self.labels.get(&v).copied().ok_or(Error::InvalidLabel(v))?;
                    Ok(OpCode::Br(address))
                }
                LabelOrOpCode::BranchIfNot(v) => {
                    let address = self.labels.get(&v).copied().ok_or(Error::InvalidLabel(v))?;
                    Ok(OpCode::Brn(address))
                }
                LabelOrOpCode::Funcall(v) => {
                    //
                    // Get the address of the label.
                    //
                    let address = index
                        .iter()
                        .find_map(|(k, a)| (k == &v).then_some(a))
                        .copied()
                        .ok_or(Error::InvalidLabel(v))?;
                    //
                    // Generate the BRL opcode.
                    //
                    Ok(OpCode::Psh(Immediate::Funcall(address)))
                }
                LabelOrOpCode::OpCode(v) => Ok(v),
            })
            .collect::<Result<_, Error>>()?;
        //
        // Done.
        //
        Ok((index, opcodes))
    }

    fn compile_defun(&mut self, atom: Rc<Atom>) -> Result<(), Error> {
        let mut ctxt = Context::default();
        //
        // Split the function call.
        //
        let Atom::Pair(a, b) = atom.as_ref() else {
            return Err(Error::ExpectedFunctionCall);
        };
        //
        // Get the function symbol.
        //
        let Atom::Symbol(symbol) = a.as_ref() else {
            return Err(Error::ExpectedSymbol);
        };
        //
        // Make sure we have a function definition.
        //
        if symbol.as_ref() != "def" {
            return Err(Error::ExpectedFunctionDefinition);
        }
        //
        // Extract the function name.
        //
        let Atom::Pair(name, rem) = b.as_ref() else {
            return Err(Error::ExpectedPair);
        };
        //
        // Make sure the function name is a symbol.
        //
        let Atom::Symbol(name) = name.as_ref() else {
            return Err(Error::ExpectedSymbol);
        };
        //
        // Make sure the function does not exist.
        //
        if self.defuns.contains(name.as_ref()) {
            return Err(Error::FunctionAlreadyDefined(name.clone()));
        }
        //
        // Extract the arguments.
        //
        let Atom::Pair(args, rem) = rem.as_ref() else {
            return Err(Error::ExpectedPair);
        };
        //
        // Track the function arguments.
        //
        let argcnt = ctxt.track(args.clone())?;
        //
        // Track the function definition.
        //
        self.defuns.insert(name.clone());
        //
        // Rotate the arguments and the return address.
        //
        if argcnt > 0 {
            ctxt.stream.push(OpCode::Rot(argcnt + 1).into());
        }
        //
        // Compile the statements.
        //
        self.compile_statements(&mut ctxt, rem.clone())?;
        //
        // Pop the arguments.
        //
        if argcnt > 0 {
            ctxt.stream.push(OpCode::Rot(argcnt + 1).into());
            ctxt.stream.push(OpCode::Pop(argcnt).into());
        }
        //
        // Inject the return call.
        //
        ctxt.stream.push(OpCode::Ret.into());
        //
        // Save the block.
        //
        self.blocks.push((name.clone(), ctxt));
        //
        // Done.
        //
        Ok(())
    }

    fn compile_lambda(&mut self, name: Box<str>, atom: Rc<Atom>) -> Result<(), Error> {
        let mut ctxt = Context::default();
        //
        // Split the lambda call.
        //
        let Atom::Pair(args, rem) = atom.as_ref() else {
            return Err(Error::ExpectedPair);
        };
        //
        // Track the function arguments.
        //
        let argcnt = ctxt.track(args.clone())?;
        //
        // Rotate the arguments and the return address.
        //
        if argcnt > 0 {
            ctxt.stream.push(OpCode::Rot(argcnt + 1).into());
        }
        //
        // Compile the statements.
        //
        self.compile_statements(&mut ctxt, rem.clone())?;
        //
        // Pop the arguments.
        //
        if argcnt > 0 {
            ctxt.stream.push(OpCode::Rot(argcnt + 1).into());
            ctxt.stream.push(OpCode::Pop(argcnt).into());
        }
        //
        // Inject the return call.
        //
        ctxt.stream.push(OpCode::Ret.into());
        //
        // Save the block.
        //
        self.blocks.push((name, ctxt));
        //
        // Done.
        //
        Ok(())
    }

    pub(crate) fn compile_funcall(
        &mut self,
        ctxt: &mut Context,
        atom: Rc<Atom>,
    ) -> Result<(), Error> {
        //
        // Split the function call.
        //
        let Atom::Pair(atom, rem) = atom.as_ref() else {
            return Err(Error::ExpectedFunctionCall);
        };
        //
        // Process the atom.
        //
        match atom.as_ref() {
            //
            // For symbols, check its value for built-ins.
            //
            Atom::Symbol(sym) => match sym.as_ref() {
                //
                // Arithmetics.
                //
                "+" => {
                    let argcnt = self.compile_arguments(ctxt, rem.clone())?;
                    ctxt.stream.push(OpCode::Add.into());
                    ctxt.stackn -= argcnt;
                    Ok(())
                }
                ">=" => {
                    let argcnt = self.compile_arguments(ctxt, rem.clone())?;
                    ctxt.stream.push(OpCode::Ge.into());
                    ctxt.stackn -= argcnt;
                    Ok(())
                }
                ">" => {
                    let argcnt = self.compile_arguments(ctxt, rem.clone())?;
                    ctxt.stream.push(OpCode::Gt.into());
                    ctxt.stackn -= argcnt;
                    Ok(())
                }
                "<=" => {
                    let argcnt = self.compile_arguments(ctxt, rem.clone())?;
                    ctxt.stream.push(OpCode::Le.into());
                    ctxt.stackn -= argcnt;
                    Ok(())
                }
                "<" => {
                    let argcnt = self.compile_arguments(ctxt, rem.clone())?;
                    ctxt.stream.push(OpCode::Lt.into());
                    ctxt.stackn -= argcnt;
                    Ok(())
                }
                "-" => {
                    let argcnt = self.compile_arguments(ctxt, rem.clone())?;
                    ctxt.stream.push(OpCode::Sub.into());
                    ctxt.stackn -= argcnt;
                    Ok(())
                }
                //
                // Quote operation.
                //
                "quote" => {
                    self.compile_quote(ctxt, rem.clone())?;
                    Ok(())
                }
                //
                // List operations.
                //
                "car" => {
                    let argcnt = self.compile_arguments(ctxt, rem.clone())?;
                    ctxt.stream.push(OpCode::Car.into());
                    ctxt.stackn -= argcnt;
                    Ok(())
                }
                "cdr" => {
                    let argcnt = self.compile_arguments(ctxt, rem.clone())?;
                    ctxt.stream.push(OpCode::Cdr.into());
                    ctxt.stackn -= argcnt;
                    Ok(())
                }
                "cons" => {
                    let argcnt = self.compile_arguments(ctxt, rem.clone())?;
                    ctxt.stream.push(OpCode::Cons.into());
                    ctxt.stackn -= argcnt;
                    Ok(())
                }
                //
                // Control flow: if.
                //
                "if" => {
                    //
                    // Unpack the condition.
                    //
                    let Atom::Pair(cond, args) = rem.as_ref() else {
                        return Err(Error::ExpectedPair);
                    };
                    //
                    // Compile the condition.
                    //
                    self.compile_value(ctxt, cond.clone())?;
                    ctxt.stackn -= 1;
                    //
                    // Unpack THEN.
                    //
                    let Atom::Pair(then, args) = args.as_ref() else {
                        return Err(Error::ExpectedPair);
                    };
                    //
                    // Unpack ELSE.
                    //
                    let else_ = match args.as_ref() {
                        Atom::Nil => Atom::nil(),
                        Atom::Pair(else_, _) => else_.clone(),
                        _ => return Err(Error::ExpectedPair),
                    };
                    //
                    // Generate the branch to the else block.
                    //
                    let name = self.label("BEGIN_ELSE");
                    let start = ctxt.stream.len();
                    let label = LabelOrOpCode::BranchIfNot(name.clone());
                    ctxt.stream.push(label);
                    //
                    // Compile THEN.
                    //
                    self.compile_value(ctxt, then.clone())?;
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
                    // Generate the branch past the else block.
                    //
                    let name = self.label("END_ELSE");
                    let start = ctxt.stream.len();
                    let label = LabelOrOpCode::Branch(name.clone());
                    ctxt.stream.push(label);
                    //
                    // Compile ELSE.
                    //
                    self.compile_value(ctxt, else_.clone())?;
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
                //
                // Value binding: let.
                //
                "let" => {
                    //
                    // Split the atom.
                    //
                    let Atom::Pair(bindings, stmts) = rem.as_ref() else {
                        return Err(Error::ExpectedPair);
                    };
                    //
                    // Compile the bindings.
                    //
                    let argcnt = self.compile_bindings(ctxt, bindings.clone())?;
                    //
                    // Compile the statements.
                    //
                    self.compile_statements(ctxt, stmts.clone())?;
                    //
                    // Pop the bindings.
                    //
                    if argcnt > 0 {
                        ctxt.stream.push(OpCode::Rot(argcnt + 1).into());
                        ctxt.stream.push(OpCode::Pop(argcnt).into());
                    }
                    //
                    // Clear the bindings from the locals.
                    //
                    self.clear_bindings(ctxt, bindings.clone())?;
                    //
                    // Done.
                    //
                    Ok(())
                }
                //
                // Function definition are forbidden.
                //
                "def" => Err(Error::FunctionDefinitionTopLevelOnly),
                //
                // Lambda definition.
                //
                "\\" => {
                    //
                    // Generate a name for the lambda.
                    //
                    let name = self.label("LAMBDA");
                    //
                    // Generate the lambda.
                    //
                    self.compile_lambda(name.clone(), rem.clone())?;
                    //
                    // Generate the function call.
                    //
                    ctxt.stream.push(LabelOrOpCode::Funcall(name));
                    ctxt.stream.push(OpCode::Pak(1).into());
                    //
                    // Done.
                    //
                    Ok(())
                }
                //
                // Global function.
                //
                _ if self.defuns.contains(sym) => {
                    //
                    // Compile the arguments.
                    //
                    let arglen = self.compile_arguments(ctxt, rem.clone())?;
                    //
                    // Generate the funcall label.
                    //
                    ctxt.stream.push(LabelOrOpCode::Funcall(sym.clone()));
                    //
                    // Generate the call opcode.
                    //
                    ctxt.stream.push(OpCode::Call.into());
                    //
                    // Update the stack tracker.
                    //
                    ctxt.stackn -= arglen;
                    //
                    // Done.
                    //
                    Ok(())
                }
                //
                // Other.
                //
                symbol => {
                    //
                    // Make sure it's a known symbol.
                    //
                    let Some(index) = ctxt.locals.get(symbol).and_then(|v| v.last()).copied()
                    else {
                        return Err(Error::InvalidSymbol(sym.clone()));
                    };
                    //
                    // Compile the arguments.
                    //
                    let argcnt = self.compile_arguments(ctxt, rem.clone())?;
                    //
                    // Grab the closure.
                    //
                    let position = ctxt.stackn - index;
                    ctxt.stream.push(OpCode::Get(position).into());
                    //
                    // Update the stack tracker.
                    //
                    ctxt.stackn += 1;
                    //
                    // Generate the call opcode.
                    //
                    ctxt.stream.push(OpCode::Call.into());
                    //
                    // Update the stack tracker.
                    //
                    ctxt.stackn -= argcnt + 1;
                    //
                    // Done.
                    //
                    Ok(())
                }
            },
            //
            // For any other type, evaluate the atom.
            //
            _ => {
                //
                // Compile the arguments.
                //
                let arglen = self.compile_arguments(ctxt, rem.clone())?;
                //
                // Evaluate the atom.
                //
                self.compile_value(ctxt, atom.clone())?;
                //
                // Generate the call opcode.
                //
                ctxt.stream.push(OpCode::Call.into());
                //
                // Update the stack tracker.
                //
                ctxt.stackn -= arglen;
                //
                // Done.
                //
                Ok(())
            }
        }
    }

    fn compile_arguments(&mut self, ctxt: &mut Context, atom: Rc<Atom>) -> Result<usize, Error> {
        atom.iter().try_fold(0, |acc, atom| {
            self.compile_value(ctxt, atom)?;
            Ok(acc + 1)
        })
    }

    fn compile_bindings(&mut self, ctxt: &mut Context, atom: Rc<Atom>) -> Result<usize, Error> {
        atom.iter().try_fold(0, |acc, atom| {
            //
            // Make sure the binding is a pair.
            //
            let Atom::Pair(symbol, stmt) = atom.as_ref() else {
                return Err(Error::ExpectedPair);
            };
            //
            // Make sure the symbol is a symbol.
            //
            let Atom::Symbol(symbol) = symbol.as_ref() else {
                return Err(Error::ExpectedSymbol);
            };
            //
            // Compile the statement.
            //
            self.compile_value(ctxt, stmt.clone())?;
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
            Ok(acc + 1)
        })
    }

    fn clear_bindings(&mut self, ctxt: &mut Context, atom: Rc<Atom>) -> Result<(), Error> {
        atom.iter().try_for_each(|atom| {
            //
            // Make sure the binding is a pair.
            //
            let Atom::Pair(symbol, _) = atom.as_ref() else {
                return Err(Error::ExpectedPair);
            };
            //
            // Make sure the symbol is a symbol.
            //
            let Atom::Symbol(symbol) = symbol.as_ref() else {
                return Err(Error::ExpectedSymbol);
            };
            //
            // Clear the binding.
            //
            ctxt.locals.entry(symbol.clone()).or_default().pop();
            //
            // Done.
            //
            Ok(())
        })
    }

    fn compile_statements(&mut self, ctxt: &mut Context, atom: Rc<Atom>) -> Result<(), Error> {
        atom.iter().enumerate().try_for_each(|(indx, atom)| {
            //
            // Drop the previous result.
            //
            if indx > 0 {
                ctxt.stream.push(OpCode::Pop(1).into());
                ctxt.stackn -= 1;
            }
            //
            // Compile the statement.
            //
            self.compile_value(ctxt, atom.clone())
        })
    }

    fn compile_quote(&mut self, ctxt: &mut Context, atom: Rc<Atom>) -> Result<(), Error> {
        match atom.as_ref() {
            Atom::Nil => {
                ctxt.stream.push(OpCode::Psh(Immediate::Nil).into());
                ctxt.stackn += 1;
                Ok(())
            }
            Atom::True => {
                ctxt.stream.push(OpCode::Psh(Immediate::True).into());
                ctxt.stackn += 1;
                Ok(())
            }
            Atom::Char(v) => {
                ctxt.stream.push(OpCode::Psh(Immediate::Char(*v)).into());
                ctxt.stackn += 1;
                Ok(())
            }
            Atom::Number(v) => {
                ctxt.stream.push(OpCode::Psh(Immediate::Number(*v)).into());
                ctxt.stackn += 1;
                Ok(())
            }
            Atom::Pair(car, cdr) => {
                self.compile_quote(ctxt, car.clone())?;
                self.compile_quote(ctxt, cdr.clone())?;
                ctxt.stream.push(OpCode::Cons.into());
                ctxt.stackn -= 1;
                Ok(())
            }
            Atom::String(_) => todo!(),
            Atom::Symbol(_) => todo!(),
            Atom::Wildcard => todo!(),
        }
    }

    fn compile_value(&mut self, ctxt: &mut Context, atom: Rc<Atom>) -> Result<(), Error> {
        match atom.as_ref() {
            Atom::Nil => {
                ctxt.stream.push(OpCode::Psh(Immediate::Nil).into());
                ctxt.stackn += 1;
                Ok(())
            }
            Atom::True => {
                ctxt.stream.push(OpCode::Psh(Immediate::True).into());
                ctxt.stackn += 1;
                Ok(())
            }
            Atom::Char(v) => {
                ctxt.stream.push(OpCode::Psh(Immediate::Char(*v)).into());
                ctxt.stackn += 1;
                Ok(())
            }
            Atom::Number(v) => {
                ctxt.stream.push(OpCode::Psh(Immediate::Number(*v)).into());
                ctxt.stackn += 1;
                Ok(())
            }
            Atom::Pair(_, _) => {
                self.compile_funcall(ctxt, atom)?;
                ctxt.stackn += 1;
                Ok(())
            }
            Atom::String(_) => todo!(),
            Atom::Symbol(v) => {
                let index = *ctxt
                    .locals
                    .get(v)
                    .and_then(|v| v.last())
                    .ok_or(Error::InvalidSymbol(v.clone()))?;
                let position = ctxt.stackn - index;
                ctxt.stream.push(OpCode::Get(position).into());
                ctxt.stackn += 1;
                Ok(())
            }
            Atom::Wildcard => todo!(),
        }
    }

    fn label(&mut self, prefix: &str) -> Box<str> {
        let label = format!("{prefix}_{:04}", self.lcount).into_boxed_str();
        self.lcount += 1;
        label
    }
}
