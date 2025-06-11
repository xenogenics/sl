use std::{collections::BTreeSet, fmt::Display, rc::Rc, str::FromStr};

use strum_macros::EnumString;

use crate::{atom::Atom, error::Error};

//
// Built-in operator.
//

#[derive(Debug, strum_macros::Display, EnumString, Eq, PartialEq)]
pub enum Operator {
    //
    // Arithmetics.
    //
    #[strum(serialize = "+")]
    Add,
    #[strum(serialize = "-")]
    Sub,
    #[strum(serialize = ">=")]
    Ge,
    #[strum(serialize = ">")]
    Gt,
    #[strum(serialize = "<=")]
    Le,
    #[strum(serialize = "<")]
    Lt,
    //
    // Logic.
    //
    #[strum(serialize = "and")]
    And,
    #[strum(serialize = "=")]
    Equ,
    #[strum(serialize = "<>")]
    Neq,
    #[strum(serialize = "not")]
    Not,
    #[strum(serialize = "or")]
    Or,
    //
    // List operations.
    //
    #[strum(serialize = "car")]
    Car,
    #[strum(serialize = "cdr")]
    Cdr,
    #[strum(serialize = "cons")]
    Cons,
    //
    // Predicates.
    //
    #[strum(serialize = "lst?")]
    IsLst,
    #[strum(serialize = "nil?")]
    IsNil,
}

//
// Value.
//

#[derive(Debug, Default, Eq, PartialEq)]
pub enum Value {
    #[default]
    Nil,
    True,
    Char(u8),
    Number(i64),
    Pair(Box<Value>, Box<Value>),
    String(Box<str>),
    Symbol(Box<str>),
}

impl Value {
    pub fn iter(&self) -> ValueIterator<'_> {
        ValueIterator(self)
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::True => write!(f, "T"),
            Value::Char(c) => write!(f, "{}", *c as char),
            Value::Number(v) => write!(f, "{v}"),
            Value::Pair(..) => write!(f, "[..]"),
            Value::String(v) => write!(f, r#""{v}""#),
            Value::Symbol(v) => write!(f, "{v}"),
        }
    }
}

impl TryFrom<Rc<Atom>> for Value {
    type Error = Error;

    fn try_from(atom: Rc<Atom>) -> Result<Self, Self::Error> {
        match atom.as_ref() {
            Atom::Nil => Ok(Self::Nil),
            Atom::True => Ok(Self::True),
            Atom::Char(v) => Ok(Self::Char(*v)),
            Atom::Number(v) => Ok(Self::Number(*v)),
            Atom::Pair(car, cdr) => {
                let car: Value = car.clone().try_into()?;
                let cdr: Value = cdr.clone().try_into()?;
                Ok(Self::Pair(car.into(), cdr.into()))
            }
            Atom::String(v) => Ok(Self::String(v.clone())),
            Atom::Symbol(v) => Ok(Self::Symbol(v.clone())),
            Atom::Wildcard => todo!(),
        }
    }
}

//
// Value iterator.
//

pub struct ValueIterator<'a>(&'a Value);

impl<'a> std::iter::Iterator for ValueIterator<'a> {
    type Item = &'a Value;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0 {
            Value::Pair(car, cdr) => {
                let result = car;
                self.0 = cdr;
                Some(result)
            }
            _ => None,
        }
    }
}

//
// Application location.
//

#[derive(Debug, Eq, PartialEq)]
pub enum Location {
    Any,
    Tail,
}

//
// Statement.
//

#[derive(Debug, Eq, PartialEq)]
pub enum Statement {
    //
    // Function application.
    //
    Apply(Box<Statement>, Statements, Location),
    Lambda(Vec<Box<str>>, Statements),
    Operator(Operator),
    //
    // Control structures.
    //
    IfThenElse(Box<Statement>, Box<Statement>, Option<Box<Statement>>),
    Let(Vec<(Box<str>, Statement)>, Statements),
    //
    // Symbol and value.
    //
    Symbol(Box<str>),
    Value(Value),
}

impl Statement {
    pub fn closure(&self) -> BTreeSet<Box<str>> {
        match self {
            Statement::Apply(sym, stmts, _) => {
                let mut v = sym.closure();
                v.extend(stmts.closure());
                v
            }
            Statement::Lambda(args, stmts) => {
                let mut v = stmts.closure();
                args.iter().for_each(|s| {
                    v.remove(s);
                });
                v
            }
            Statement::IfThenElse(cond, then, None) => {
                let mut v = cond.closure();
                v.extend(then.closure());
                v
            }
            Statement::IfThenElse(cond, then, Some(else_)) => {
                let mut v = cond.closure();
                v.extend(then.closure());
                v.extend(else_.closure());
                v
            }
            Statement::Let(bindings, stmts) => {
                let mut args = BTreeSet::new();
                //
                // Compute the closure for the bindings.
                //
                // NOTE(xrg): binding symbol are incrementally removed from the
                // closures as they are defined.
                //
                let mut v = bindings.iter().fold(BTreeSet::new(), |mut acc, (sym, v)| {
                    let mut v = v.closure();
                    args.iter().for_each(|s| {
                        v.remove(s);
                    });
                    args.insert(sym.clone());
                    acc.extend(v);
                    acc
                });
                //
                // Compute the closure for the statements.
                //
                // NOTE(xrg): the bindings are removed from the closure.
                //
                let mut w = stmts.closure();
                args.iter().for_each(|s| {
                    w.remove(s);
                });
                v.extend(w);
                //
                // Done.
                //
                v
            }
            Statement::Symbol(sym) => {
                let mut v = BTreeSet::new();
                v.insert(sym.clone());
                v
            }
            _ => BTreeSet::default(),
        }
    }

    fn from_pair(atom: Rc<Atom>, rem: Rc<Atom>) -> Result<Self, Error> {
        //
        // Process the atom.
        //
        match atom.as_ref() {
            //
            // For symbols, check its value for built-ins.
            //
            Atom::Symbol(sym) => match sym.as_ref() {
                //
                // Quote.
                //
                "quote" => Value::try_from(rem).map(Statement::Value),
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
                    // Parse the condition.
                    //
                    let cond: Statement = cond.clone().try_into()?;
                    //
                    // Unpack THEN.
                    //
                    let Atom::Pair(then, args) = args.as_ref() else {
                        return Err(Error::ExpectedPair);
                    };
                    //
                    // Parse THEN.
                    //
                    let then: Statement = then.clone().try_into()?;
                    //
                    // Unpack ELSE.
                    //
                    let else_ = match args.as_ref() {
                        Atom::Nil => None,
                        Atom::Pair(else_, _) => {
                            let v = Statement::try_from(else_.clone())?;
                            Some(Box::new(v))
                        }
                        _ => return Err(Error::ExpectedPair),
                    };
                    //
                    // Done.
                    //
                    Ok(Self::IfThenElse(cond.into(), then.into(), else_))
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
                    // Parse the bindings.
                    //
                    let bindings: Vec<_> = bindings
                        .iter()
                        .map(|v| {
                            //
                            // Make sure the binding is a pair.
                            //
                            let Atom::Pair(symbol, stmt) = v.as_ref() else {
                                return Err(Error::ExpectedPair);
                            };
                            //
                            // Make sure the symbol is a symbol.
                            //
                            let Atom::Symbol(symbol) = symbol.as_ref() else {
                                return Err(Error::ExpectedSymbol);
                            };
                            //
                            // Parse the statement.
                            //
                            let v: Statement = stmt.clone().try_into()?;
                            //
                            // Done.
                            //
                            Ok((symbol.clone(), v))
                        })
                        .collect::<Result<_, _>>()?;
                    //
                    // Parse the statements.
                    //
                    let stmts: Statements = stmts.clone().try_into()?;
                    //
                    // Done.
                    //
                    Ok(Self::Let(bindings, stmts))
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
                    // Split the lambda call.
                    //
                    let Atom::Pair(args, rem) = rem.as_ref() else {
                        return Err(Error::ExpectedPair);
                    };
                    //
                    // Build the argument list.
                    //
                    let args = args.iter().try_fold(Vec::new(), |mut acc, v| {
                        //
                        // Make sure the value is a symbol.
                        //
                        let Atom::Symbol(symbol) = v.as_ref() else {
                            return Err(Error::ExpectedSymbol);
                        };
                        //
                        // Push the symbol.
                        //
                        acc.push(symbol.clone());
                        //
                        // Done.
                        //
                        Ok(acc)
                    })?;
                    //
                    // Build the statement list.
                    //
                    let stmts: Statements = rem.clone().try_into()?;
                    //
                    // Done.
                    //
                    Ok(Self::Lambda(args, stmts))
                }
                //
                // Other symbols.
                //
                v => match Operator::from_str(v) {
                    Ok(v) => {
                        let stmt = Box::new(Statement::Operator(v));
                        let stmts: Statements = rem.clone().try_into()?;
                        Ok(Self::Apply(stmt, stmts, Location::Any))
                    }
                    Err(_) => {
                        let stmt = Box::new(Statement::Symbol(sym.clone()));
                        let stmts: Statements = rem.clone().try_into()?;
                        Ok(Self::Apply(stmt, stmts, Location::Any))
                    }
                },
            },
            //
            // For any other type, evaluate the atom.
            //
            _ => {
                let car = Box::new(Statement::try_from(atom.clone())?);
                let cdr: Statements = rem.clone().try_into()?;
                Ok(Self::Apply(car, cdr, Location::Any))
            }
        }
    }

    pub fn statements(&self) -> Box<dyn std::iter::Iterator<Item = &Statement> + '_> {
        match self {
            Statement::Apply(statement, statements, _) => {
                let iter = Some(statement.as_ref())
                    .into_iter()
                    .chain(statements.iter());
                Box::new(iter)
            }
            Statement::Lambda(_, statements) => {
                let iter = statements.iter();
                Box::new(iter)
            }
            Statement::IfThenElse(statement, then, else_) => {
                let iter = Some(statement.as_ref())
                    .into_iter()
                    .chain(then.statements())
                    .chain(else_.into_iter().flat_map(|v| v.statements()));
                Box::new(iter)
            }
            Statement::Let(bindings, statements) => {
                let iter = bindings
                    .iter()
                    .flat_map(|(_, v)| v.statements())
                    .chain(statements.iter());
                Box::new(iter)
            }
            _ => Box::new(None.into_iter()),
        }
    }

    fn identify_tail_calls(&mut self, name: &str) {
        match self {
            Statement::Apply(stmt, _, location) => {
                if let Statement::Symbol(v) = stmt.as_ref() {
                    if name == v.as_ref() {
                        *location = Location::Tail;
                    }
                }
            }
            Statement::IfThenElse(_, then, else_) => {
                //
                // Process THEN.
                //
                then.identify_tail_calls(name);
                //
                // Process ELSE.
                //
                if let Some(else_) = else_.as_mut() {
                    else_.identify_tail_calls(name);
                }
            }
            Statement::Let(_, stmts) => stmts.identify_tail_calls(name),
            _ => (),
        }
    }

    pub fn is_tail_call(&self) -> bool {
        match self {
            Statement::Apply(_, _, Location::Tail) => true,
            Statement::IfThenElse(_, then, else_) => {
                then.is_tail_call()
                    && else_
                        .as_deref()
                        .map(Statement::is_tail_call)
                        .unwrap_or_default()
            }
            _ => false,
        }
    }
}

impl Display for Statement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Statement::Apply(statement, statements, _) => {
                write!(f, "({statement}")?;
                write!(f, "{statements}")?;
                write!(f, ")")
            }
            Statement::Lambda(args, statements) => {
                write!(f, "(\\ (")?;
                for (i, v) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{v}")?;
                }
                write!(f, ")")?;
                write!(f, "{statements}")?;
                write!(f, ")")
            }
            Statement::Operator(v) => write!(f, "{v}"),
            Statement::IfThenElse(cond, then, None) => {
                write!(f, "(if {cond} {then}")
            }
            Statement::IfThenElse(cond, then, Some(else_)) => {
                write!(f, "(if {cond} {then} {else_}")
            }
            Statement::Let(bindings, statements) => {
                write!(f, "(let (")?;
                bindings
                    .iter()
                    .try_for_each(|(k, v)| write!(f, "({k} . {v})"))?;
                write!(f, ")")?;
                write!(f, "{statements}")?;
                write!(f, ")")
            }
            Statement::Symbol(symbol) => write!(f, "{symbol}"),
            Statement::Value(value) => write!(f, "{value}"),
        }
    }
}

impl TryFrom<Rc<Atom>> for Statement {
    type Error = Error;

    fn try_from(atom: Rc<Atom>) -> Result<Self, Self::Error> {
        match atom.as_ref() {
            Atom::Pair(atom, rem) => Self::from_pair(atom.clone(), rem.clone()),
            Atom::Symbol(sym) => Ok(Self::Symbol(sym.clone())),
            _ => Value::try_from(atom).map(Self::Value),
        }
    }
}

//
// Statements.
//

#[derive(Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct Statements(Vec<Statement>);

impl Statements {
    #[cfg(test)]
    pub fn new(stmts: Vec<Statement>) -> Self {
        Self(stmts)
    }

    pub fn closure(&self) -> BTreeSet<Box<str>> {
        self.0.iter().fold(BTreeSet::new(), |mut acc, v| {
            acc.extend(v.closure());
            acc
        })
    }

    pub fn iter(&self) -> impl std::iter::DoubleEndedIterator<Item = &Statement> {
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    fn identify_tail_calls(&mut self, name: &str) {
        //
        // Get the last statement.
        //
        let Some(stmt) = self.0.last_mut() else {
            return;
        };
        //
        // Identify tail calls on the last statement.
        //
        stmt.identify_tail_calls(name);
    }

    pub fn is_tail_call(&self) -> bool {
        self.0
            .last()
            .map(Statement::is_tail_call)
            .unwrap_or_default()
    }
}

impl Display for Statements {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.iter().try_for_each(|v| write!(f, " {v}"))
    }
}

impl TryFrom<Rc<Atom>> for Statements {
    type Error = Error;

    fn try_from(value: Rc<Atom>) -> Result<Self, Self::Error> {
        let inner = value
            .into_iter()
            .map(Statement::try_from)
            .collect::<Result<_, Error>>()?;
        Ok(Self(inner))
    }
}

//
// Top-level statements.
//

pub enum TopLevelStatement {
    FunctionDefinition(FunctionDefinition),
    Load(Statements),
}

impl TryFrom<Rc<Atom>> for TopLevelStatement {
    type Error = Error;

    fn try_from(atom: Rc<Atom>) -> Result<Self, Self::Error> {
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
        // Process the top-level statement.
        //
        match symbol.as_ref() {
            "def" => FunctionDefinition::try_from(atom).map(Self::FunctionDefinition),
            "load" => b.clone().try_into().map(Self::Load),
            _ => Err(Error::ExpectedTopLevelStatement),
        }
    }
}

//
// Function definition.
//

#[derive(Debug, Eq, PartialEq)]
pub struct FunctionDefinition(Box<str>, Vec<Box<str>>, Statements);

impl FunctionDefinition {
    #[cfg(test)]
    pub fn new(name: Box<str>, args: Vec<Box<str>>, stmts: Statements) -> Self {
        Self(name, args, stmts)
    }

    pub fn name(&self) -> &Box<str> {
        &self.0
    }

    pub fn arguments(&self) -> &[Box<str>] {
        &self.1
    }

    pub fn statements(&self) -> &Statements {
        &self.2
    }

    pub fn closure(&self) -> BTreeSet<Box<str>> {
        let mut v = self.2.closure();
        self.1.iter().for_each(|s| {
            v.remove(s);
        });
        v
    }
}

impl Display for FunctionDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(def {} (", self.0)?;
        for (i, v) in self.1.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "{v}")?;
        }
        write!(f, "){})", self.2)
    }
}

impl TryFrom<Rc<Atom>> for FunctionDefinition {
    type Error = Error;

    fn try_from(atom: Rc<Atom>) -> Result<Self, Self::Error> {
        //
        // Split the function call.
        //
        let Atom::Pair(def, rem) = atom.as_ref() else {
            return Err(Error::ExpectedFunctionCall);
        };
        //
        // Get the function symbol.
        //
        let Atom::Symbol(symbol) = def.as_ref() else {
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
        let Atom::Pair(name, rem) = rem.as_ref() else {
            return Err(Error::ExpectedPair);
        };
        //
        // Make sure the function name is a symbol.
        //
        let Atom::Symbol(name) = name.as_ref() else {
            return Err(Error::ExpectedSymbol);
        };
        //
        // Extract the arguments.
        //
        let Atom::Pair(args, rem) = rem.as_ref() else {
            return Err(Error::ExpectedPair);
        };
        //
        // Build the argument list.
        //
        let args = args.iter().try_fold(Vec::new(), |mut acc, v| {
            //
            // Make sure the value is a symbol.
            //
            let Atom::Symbol(symbol) = v.as_ref() else {
                return Err(Error::ExpectedSymbol);
            };
            //
            // Push the symbol.
            //
            acc.push(symbol.clone());
            //
            // Done.
            //
            Ok(acc)
        })?;
        //
        // Check if there is a comment.
        //
        let Atom::Pair(maybe_comment, statements) = rem.as_ref() else {
            return Err(Error::ExpectedPair);
        };
        //
        // Skip the comment if any.
        //
        let rem = match maybe_comment.as_ref() {
            Atom::String(_) => statements,
            _ => rem,
        };
        //
        // Build the statement list.
        //
        let mut stmts: Statements = rem.clone().try_into()?;
        //
        // Identify tail calls.
        //
        stmts.identify_tail_calls(name.as_ref());
        //
        // Done.
        //
        Ok(Self(name.clone(), args, stmts))
    }
}
