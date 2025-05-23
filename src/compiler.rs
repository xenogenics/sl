use std::{collections::HashMap, rc::Rc};

use crate::{
    error::Error,
    opcodes::{Immediate, OpCode},
    types::Atom,
};

#[derive(Default)]
pub struct Compiler {
    labels: HashMap<Box<str>, (usize, usize)>,
    locals: HashMap<Box<str>, Vec<usize>>,
    stackn: usize,
}

impl Compiler {
    pub fn compile(
        &mut self,
        stmt: Vec<Rc<Atom>>,
    ) -> Result<(HashMap<Box<str>, usize>, Vec<OpCode>), Error> {
        //
        // Compile the statements.
        //
        let opcodes = stmt
            .into_iter()
            .try_fold(Vec::new(), |acc, v| self.compile_funcall(v, acc))?;
        //
        // Clean-up the labels.
        //
        let labels = self.labels.drain().map(|(k, (v, _))| (k, v)).collect();
        //
        // Done.
        //
        Ok((labels, opcodes))
    }

    fn compile_funcall(
        &mut self,
        atom: Rc<Atom>,
        mut opcodes: Vec<OpCode>,
    ) -> Result<Vec<OpCode>, Error> {
        //
        // Split the function call.
        //
        let Atom::Pair(a, b) = atom.as_ref() else {
            return Err(Error::ExpectedFunctionCall);
        };
        //
        // Get the function symbol.
        //
        let Atom::Symbol(s) = a.as_ref() else {
            return Err(Error::ExpectedSymbol);
        };
        //
        // Process the symbol.
        //
        match s.as_ref() {
            //
            // Arithmetics.
            //
            "+" => {
                let mut opcodes = self.compile_arguments(b.clone(), opcodes)?;
                opcodes.push(OpCode::Add);
                self.stackn -= 2;
                Ok(opcodes)
            }
            ">=" => {
                let mut opcodes = self.compile_arguments(b.clone(), opcodes)?;
                opcodes.push(OpCode::Ge);
                self.stackn -= 2;
                Ok(opcodes)
            }
            ">" => {
                let mut opcodes = self.compile_arguments(b.clone(), opcodes)?;
                opcodes.push(OpCode::Gt);
                self.stackn -= 2;
                Ok(opcodes)
            }
            "<=" => {
                let mut opcodes = self.compile_arguments(b.clone(), opcodes)?;
                opcodes.push(OpCode::Le);
                self.stackn -= 2;
                Ok(opcodes)
            }
            "<" => {
                let mut opcodes = self.compile_arguments(b.clone(), opcodes)?;
                opcodes.push(OpCode::Lt);
                self.stackn -= 2;
                Ok(opcodes)
            }
            "-" => {
                let mut opcodes = self.compile_arguments(b.clone(), opcodes)?;
                opcodes.push(OpCode::Sub);
                self.stackn -= 2;
                Ok(opcodes)
            }
            //
            // Control flow: if.
            //
            "if" => {
                //
                // Unpack the condition.
                //
                let Atom::Pair(cond, args) = b.as_ref() else {
                    return Err(Error::ExpectedPair);
                };
                //
                // Compile the condition.
                //
                let mut opcodes = self.compile_value(cond.clone(), opcodes)?;
                self.stackn -= 1;
                //
                // Unpack THEN.
                //
                let Atom::Pair(then, args) = args.as_ref() else {
                    return Err(Error::ExpectedPair);
                };
                //
                // Compile THEN.
                //
                let then_opcodes = self.compile_value(then.clone(), Vec::new())?;
                //
                // Unpack ELSE.
                //
                let else_ = match args.as_ref() {
                    Atom::Nil => Atom::nil(),
                    Atom::Pair(else_, _) => else_.clone(),
                    _ => return Err(Error::ExpectedPair),
                };
                //
                // Ignore the stack value for the THEN branch.
                //
                self.stackn -= 1;
                //
                // Generate the branch.
                //
                opcodes.push(OpCode::Brn(then_opcodes.len() + 2));
                //
                // Append the THEN opcodes.
                //
                opcodes.extend_from_slice(&then_opcodes);
                //
                // Compile ELSE.
                //
                let else_opcodes = self.compile_value(else_.clone(), Vec::new())?;
                //
                // Generate the branch.
                //
                opcodes.push(OpCode::Br(else_opcodes.len() + 1));
                //
                // Append the ELSE opcodes.
                //
                opcodes.extend_from_slice(&else_opcodes);
                //
                // Done.
                //
                Ok(opcodes)
            }
            //
            // Function definition: def.
            //
            "def" => {
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
                if self.labels.contains_key(name.as_ref()) {
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
                let argcnt = self.track_arguments(0, args.clone())?;
                self.stackn = argcnt;
                //
                // Save the function address.
                //
                self.labels.insert(name.clone(), (opcodes.len(), argcnt));
                //
                // Rotate the arguments and the return address.
                //
                if argcnt > 0 {
                    opcodes.push(OpCode::Rot(self.locals.len() + 1));
                }
                //
                // Compile the statements.
                //
                let mut opcodes = self.compile_statements(0, rem.clone(), opcodes)?;
                //
                // Pop the arguments.
                //
                if argcnt > 0 {
                    opcodes.push(OpCode::Rot(argcnt + 1));
                    opcodes.push(OpCode::Pop(argcnt));
                }
                //
                // Inject the return call.
                //
                opcodes.push(OpCode::Ret);
                self.stackn -= 1;
                //
                // Clear the local mapping.
                //
                self.locals.clear();
                //
                // Done.
                //
                Ok(opcodes)
            }
            //
            // Value binding: let.
            //
            "let" => {
                //
                // Split the atom.
                //
                let Atom::Pair(bindings, stmts) = b.as_ref() else {
                    return Err(Error::ExpectedPair);
                };
                //
                // Compile the bindings.
                //
                let pre = self.stackn;
                let opcodes = self.compile_bindings(bindings.clone(), opcodes)?;
                let argcnt = self.stackn - pre;
                //
                // Compile the statements.
                //
                let mut opcodes = self.compile_statements(0, stmts.clone(), opcodes)?;
                //
                // Pop the bindings.
                //
                if argcnt > 0 {
                    opcodes.push(OpCode::Rot(argcnt + 1));
                    opcodes.push(OpCode::Pop(argcnt));
                }
                //
                // Clear the bindings from the locals.
                //
                self.clear_bindings(bindings.clone())?;
                //
                // Done.
                //
                Ok(opcodes)
            }
            //
            // Other symbol.
            //
            symbol => match self.labels.get(symbol).copied() {
                Some((index, arglen)) => {
                    let mut opcodes = self.compile_arguments(b.clone(), opcodes)?;
                    opcodes.push(OpCode::Brl(index));
                    self.stackn -= arglen;
                    Ok(opcodes)
                }
                None => Err(Error::InvalidSymbol(symbol.into())),
            },
        }
    }

    fn compile_arguments(
        &mut self,
        atom: Rc<Atom>,
        opcodes: Vec<OpCode>,
    ) -> Result<Vec<OpCode>, Error> {
        match atom.as_ref() {
            Atom::Nil => Ok(opcodes),
            Atom::Pair(a, b) => {
                let opcodes = self.compile_value(a.clone(), opcodes)?;
                self.compile_arguments(b.clone(), opcodes)
            }
            _ => Err(Error::ExpectedPair),
        }
    }

    fn compile_bindings(
        &mut self,
        atom: Rc<Atom>,
        opcodes: Vec<OpCode>,
    ) -> Result<Vec<OpCode>, Error> {
        //
        // Split the atom.
        //
        let (binding, rem) = match atom.as_ref() {
            Atom::Nil => return Ok(opcodes),
            Atom::Pair(binding, rem) => (binding, rem),
            _ => return Err(Error::ExpectedPair),
        };
        //
        // Make sure the binding is a pair.
        //
        let Atom::Pair(symbol, stmt) = binding.as_ref() else {
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
        let opcodes = self.compile_value(stmt.clone(), opcodes)?;
        //
        // Save the symbol's index.
        //
        self.locals
            .entry(symbol.clone())
            .or_default()
            .push(self.stackn - 1);
        //
        // Process the remaining bindings.
        //
        self.compile_bindings(rem.clone(), opcodes)
    }

    fn clear_bindings(&mut self, atom: Rc<Atom>) -> Result<(), Error> {
        //
        // Split the atom.
        //
        let (binding, rem) = match atom.as_ref() {
            Atom::Nil => return Ok(()),
            Atom::Pair(binding, rem) => (binding, rem),
            _ => return Err(Error::ExpectedPair),
        };
        //
        // Make sure the binding is a pair.
        //
        let Atom::Pair(symbol, _) = binding.as_ref() else {
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
        self.locals.entry(symbol.clone()).or_default().pop();
        //
        // Process the remaining bindings.
        //
        self.clear_bindings(rem.clone())
    }

    fn compile_statements(
        &mut self,
        indx: usize,
        atom: Rc<Atom>,
        mut opcodes: Vec<OpCode>,
    ) -> Result<Vec<OpCode>, Error> {
        //
        // Split the atom.
        //
        let (atom, rest) = match atom.as_ref() {
            Atom::Nil => return Ok(opcodes),
            Atom::Pair(atom, rest) => (atom, rest),
            _ => return Err(Error::ExpectedPair),
        };
        //
        // Drop the previous result.
        //
        if indx > 0 {
            opcodes.push(OpCode::Pop(1));
            self.stackn -= 1;
        }
        //
        // Compile the statement.
        //
        let opcodes = self.compile_value(atom.clone(), opcodes)?;
        //
        // Process the remaining statements.
        //
        self.compile_statements(indx + 1, rest.clone(), opcodes)
    }

    fn compile_value(
        &mut self,
        atom: Rc<Atom>,
        mut opcodes: Vec<OpCode>,
    ) -> Result<Vec<OpCode>, Error> {
        match atom.as_ref() {
            Atom::Nil => {
                opcodes.push(OpCode::Psh(Immediate::Nil));
                self.stackn += 1;
                Ok(opcodes)
            }
            Atom::True => {
                opcodes.push(OpCode::Psh(Immediate::True));
                self.stackn += 1;
                Ok(opcodes)
            }
            Atom::Char(v) => {
                opcodes.push(OpCode::Psh(Immediate::Char(*v)));
                self.stackn += 1;
                Ok(opcodes)
            }
            Atom::Number(v) => {
                opcodes.push(OpCode::Psh(Immediate::Number(*v)));
                self.stackn += 1;
                Ok(opcodes)
            }
            Atom::Pair(_, _) => {
                let opcodes = self.compile_funcall(atom, opcodes)?;
                self.stackn += 1;
                Ok(opcodes)
            }
            Atom::String(_) => todo!(),
            Atom::Symbol(v) => {
                let index = *self
                    .locals
                    .get(v)
                    .and_then(|v| v.last())
                    .ok_or(Error::InvalidSymbol(v.clone()))?;
                let position = self.stackn - index;
                opcodes.push(OpCode::Pck(position));
                self.stackn += 1;
                Ok(opcodes)
            }
            Atom::Wildcard => todo!(),
        }
    }

    fn track_arguments(&mut self, index: usize, atom: Rc<Atom>) -> Result<usize, Error> {
        //
        // Split the atom.
        //
        let (arg, rem) = match atom.as_ref() {
            Atom::Nil => return Ok(index),
            Atom::Pair(arg, rem) => (arg, rem),
            _ => return Err(Error::ExpectedPair),
        };
        //
        // Make sure the argument is a symbol.
        //
        let Atom::Symbol(v) = arg.as_ref() else {
            return Err(Error::ExpectedSymbol);
        };
        //
        // Update the argument map.
        //
        self.locals.entry(v.to_owned()).or_default().push(index);
        //
        // Process the rest of the arguments.
        //
        self.track_arguments(index + 1, rem.clone())
    }
}
