use std::{collections::BTreeSet, fmt::Display, rc::Rc, str::FromStr};

use bincode::{Decode, Encode};
use strum_macros::EnumString;

use crate::{atom::Atom, error::Error, opcodes::Arity};

//
// Arguments.
//

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Arguments {
    Capture(Box<str>),
    List(Vec<Box<str>>),
    ListAndCapture(Vec<Box<str>>, Box<str>),
    None,
}

impl Arguments {
    pub fn arity(&self) -> Arity {
        match self {
            Arguments::Capture(_) => Arity::All,
            Arguments::List(items) => Arity::Some(items.len() as u16),
            Arguments::ListAndCapture(items, _) => Arity::SomeWithRem(items.len() as u16),
            Arguments::None => Arity::None,
        }
    }

    pub fn iter(&self) -> Box<dyn std::iter::DoubleEndedIterator<Item = &Box<str>> + '_> {
        match self {
            Arguments::Capture(v) => Box::new(Some(v).into_iter()),
            Arguments::List(items) => Box::new(items.iter()),
            Arguments::ListAndCapture(items, last) => Box::new(items.iter().chain(Some(last))),
            Arguments::None => Box::new(None.iter()),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Arguments::Capture(_) => 1,
            Arguments::List(items) => items.len(),
            Arguments::ListAndCapture(items, _) => items.len() + 1,
            Arguments::None => 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Arguments::Capture(_) | Arguments::ListAndCapture(..) => false,
            Arguments::List(items) => items.is_empty(),
            Arguments::None => true,
        }
    }

    fn from_pair(atom: Rc<Atom>, mut syms: Vec<Box<str>>) -> Result<Self, Error> {
        /*
         * Split the atom.
         */
        let Atom::Pair(_, car, cdr) = atom.as_ref() else {
            return Err(Error::ExpectedPair(atom.span()));
        };
        /*
         * Make sure CAR is a symbol.
         */
        let Atom::Symbol(_, symbol) = car.as_ref() else {
            return Err(Error::ExpectedSymbol(car.span()));
        };
        /*
         * Save the symbol.
         */
        syms.push(symbol.clone());
        /*
         * Check CDR.
         */
        match cdr.as_ref() {
            Atom::Nil(_) => Ok(Self::List(syms)),
            Atom::Pair(..) => Self::from_pair(cdr.clone(), syms),
            Atom::Symbol(_, v) => Ok(Self::ListAndCapture(syms, v.clone())),
            _ => Err(Error::ExpectedPairOrSymbol(cdr.span())),
        }
    }
}

impl TryFrom<Rc<Atom>> for Arguments {
    type Error = Error;

    fn try_from(value: Rc<Atom>) -> Result<Self, Error> {
        match value.as_ref() {
            Atom::Nil(_) => Ok(Arguments::None),
            Atom::Pair(..) => Self::from_pair(value, Vec::new()),
            Atom::Symbol(_, v) => Ok(Arguments::Capture(v.clone())),
            _ => Err(Error::ExpectedPairOrSymbol(value.span())),
        }
    }
}

//
// Typed arguments.
//

#[derive(Clone, Copy, Debug, EnumString, Encode, Decode)]
pub enum ExternalType {
    #[strum(serialize = "bytes")]
    Bytes,
    #[strum(serialize = "integer")]
    Integer,
    #[strum(serialize = "string")]
    String,
    #[strum(serialize = "void")]
    Void,
}

impl TryFrom<Rc<Atom>> for ExternalType {
    type Error = Error;

    fn try_from(value: Rc<Atom>) -> Result<Self, Self::Error> {
        //
        // Make sure we have a symbol.
        //
        let Atom::Symbol(_, sym) = value.as_ref() else {
            return Err(Error::ExpectedSymbol(value.span()));
        };
        //
        // Convert the symbol name to a type.
        //
        Self::from_str(sym).map_err(|_| Error::InvalidForeignType(sym.clone()))
    }
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct ExternalArguments(Vec<(Box<str>, ExternalType)>);

impl ExternalArguments {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn types(&self) -> impl std::iter::Iterator<Item = ExternalType> {
        self.0.iter().map(|(_, v)| *v)
    }
}

impl TryFrom<Rc<Atom>> for ExternalArguments {
    type Error = Error;

    fn try_from(value: Rc<Atom>) -> Result<Self, Error> {
        //
        // Make sure we have a pair.
        //
        if !value.is_pair() {
            return Err(Error::ExpectedPair(value.span()));
        }
        //
        // Build the argument list.
        //
        let args = value
            .into_iter()
            .map(|v| -> Result<_, Error> {
                //
                // Make sure we have a pair.
                //
                let Atom::Pair(_, name, ftyp) = v.as_ref() else {
                    return Err(Error::ExpectedPair(v.span()));
                };
                //
                // Make sure the argument name is a symbol.
                //
                let Atom::Symbol(_, name) = name.as_ref() else {
                    return Err(Error::ExpectedSymbol(name.span()));
                };
                //
                // Parse the foreign type.
                //
                let ftyp = ExternalType::try_from(ftyp.clone())?;
                //
                // Done.
                //
                Ok((name.clone(), ftyp))
            })
            .collect::<Result<_, Error>>()?;
        //
        // Done.
        //
        Ok(Self(args))
    }
}

//
// Built-in operator.
//

#[derive(Clone, Copy, Debug, strum_macros::Display, EnumString, Eq, PartialEq)]
pub enum Operator {
    //
    // Arithmetics.
    //
    #[strum(serialize = "+")]
    Add,
    #[strum(serialize = "-")]
    Sub,
    #[strum(serialize = "*")]
    Mul,
    #[strum(serialize = "/")]
    Div,
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
    // Bits.
    //
    #[strum(serialize = "bitand")]
    BitAnd,
    #[strum(serialize = "bitnot")]
    BitNot,
    #[strum(serialize = "bitor")]
    BitOr,
    #[strum(serialize = "bitxor")]
    BitXor,
    //
    // List operations.
    //
    #[strum(serialize = "car")]
    Car,
    #[strum(serialize = "cdr")]
    Cdr,
    #[strum(serialize = "conc")]
    Conc,
    #[strum(serialize = "cons")]
    Cons,
    //
    // Bytes operation.
    //
    #[strum(serialize = "bytes")]
    Bytes,
    #[strum(serialize = "str")]
    Str,
    #[strum(serialize = "unpack")]
    Unpack,
    //
    // Predicates.
    //
    #[strum(serialize = "chr?")]
    IsChr,
    #[strum(serialize = "num?")]
    IsNum,
    #[strum(serialize = "lst?")]
    IsLst,
    #[strum(serialize = "nil?")]
    IsNil,
    #[strum(serialize = "sym?")]
    IsSym,
    #[strum(serialize = "tru?")]
    IsTru,
    #[strum(serialize = "wld?")]
    IsWld,
}

impl Operator {
    pub fn arity(&self) -> usize {
        match self {
            Operator::Add => 2,
            Operator::Sub => 2,
            Operator::Mul => 2,
            Operator::Div => 2,
            Operator::Ge => 2,
            Operator::Gt => 2,
            Operator::Le => 2,
            Operator::Lt => 2,
            Operator::And => 2,
            Operator::Equ => 2,
            Operator::Neq => 2,
            Operator::Not => 1,
            Operator::Or => 2,
            Operator::BitAnd => 2,
            Operator::BitNot => 1,
            Operator::BitOr => 2,
            Operator::BitXor => 2,
            Operator::Car => 1,
            Operator::Cdr => 1,
            Operator::Conc => 2,
            Operator::Cons => 2,
            Operator::Bytes => 1,
            Operator::Str => 1,
            Operator::Unpack => 1,
            Operator::IsChr => 1,
            Operator::IsNum => 1,
            Operator::IsLst => 1,
            Operator::IsNil => 1,
            Operator::IsSym => 1,
            Operator::IsTru => 1,
            Operator::IsWld => 1,
        }
    }
}

//
// Value.
//

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Value {
    #[default]
    Nil,
    True,
    Char(u8),
    Number(i64),
    String(Box<str>),
    Wildcard,
}

impl TryFrom<Rc<Atom>> for Value {
    type Error = Error;

    fn try_from(atom: Rc<Atom>) -> Result<Self, Self::Error> {
        match atom.as_ref() {
            Atom::Nil(_) => Ok(Self::Nil),
            Atom::True(_) => Ok(Self::True),
            Atom::Char(_, v) => Ok(Self::Char(*v)),
            Atom::Number(_, v) => Ok(Self::Number(*v)),
            Atom::String(_, v) => Ok(Self::String(v.clone())),
            Atom::Pair(..) | Atom::Symbol(..) => Err(Error::ExpectedValue(atom.span())),
            Atom::Wildcard(_) => Ok(Self::Wildcard),
        }
    }
}

//
// Application location.
//

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CallSite {
    Any,
    Tail,
}

//
// Backquote.
//

#[derive(Clone, Debug)]
pub enum Backquote {
    Pair(Box<Backquote>, Box<Backquote>),
    Symbol(Box<str>),
    Unquote(Box<Statement>),
    UnquoteSplice(Box<Statement>),
    Value(Value),
}

impl Backquote {
    pub fn iter(&self) -> BackquoteIterator<'_> {
        BackquoteIterator(self)
    }

    pub fn is_nil(&self) -> bool {
        matches!(self, Self::Value(Value::Nil))
    }

    pub fn is_splice(&self) -> bool {
        matches!(self, Self::UnquoteSplice(_))
    }
}

impl Backquote {
    fn from_atom(value: Rc<Atom>, macros: &BTreeSet<Box<str>>) -> Result<Self, Error> {
        match value.as_ref() {
            Atom::Pair(_, car, cdr) => {
                let car = Self::from_unquote(car.clone(), macros)?;
                let cdr = Self::from_unquote(cdr.clone(), macros)?;
                Ok(Self::Pair(car.into(), cdr.into()))
            }
            Atom::Symbol(_, v) => Ok(Self::Symbol(v.clone())),
            _ => value.try_into().map(Self::Value),
        }
    }

    fn from_unquote(value: Rc<Atom>, macros: &BTreeSet<Box<str>>) -> Result<Self, Error> {
        if let Atom::Pair(_, car, cdr) = value.as_ref()
            && let Atom::Symbol(_, v) = car.as_ref()
        {
            match v.as_ref() {
                "backquote" => {
                    let value = Atom::cons(Atom::symbol("quote"), cdr.clone());
                    Self::from_atom(value, macros)
                }
                "unquote" => Statement::from_atom(cdr.clone(), macros)
                    .map(Box::new)
                    .map(Self::Unquote),
                "unquote-splice" => Statement::from_atom(cdr.clone(), macros)
                    .map(Box::new)
                    .map(Self::UnquoteSplice),
                _ => Self::from_atom(value, macros),
            }
        } else {
            Self::from_atom(value, macros)
        }
    }
}

pub struct BackquoteIterator<'a>(&'a Backquote);

impl<'a> std::iter::Iterator for BackquoteIterator<'a> {
    type Item = &'a Backquote;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0 {
            Backquote::Pair(car, cdr) => {
                let result = car;
                self.0 = cdr;
                Some(result)
            }
            _ => None,
        }
    }
}

//
// Quote.
//

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Quote {
    Pair(Box<Quote>, Box<Quote>),
    Symbol(Box<str>),
    Value(Value),
}

impl Quote {
    pub fn iter(&self) -> QuoteIterator<'_> {
        QuoteIterator(self)
    }
}

impl TryFrom<Rc<Atom>> for Quote {
    type Error = Error;

    fn try_from(value: Rc<Atom>) -> Result<Self, Self::Error> {
        match value.as_ref() {
            Atom::Pair(_, car, cdr) => {
                let car = Self::try_from(car.clone())?;
                let cdr = Self::try_from(cdr.clone())?;
                Ok(Self::Pair(car.into(), cdr.into()))
            }
            Atom::Symbol(_, v) => Ok(Self::Symbol(v.clone())),
            _ => value.try_into().map(Self::Value),
        }
    }
}

pub struct QuoteIterator<'a>(&'a Quote);

impl<'a> std::iter::Iterator for QuoteIterator<'a> {
    type Item = &'a Quote;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0 {
            Quote::Pair(car, cdr) => {
                let result = car;
                self.0 = cdr;
                Some(result)
            }
            _ => None,
        }
    }
}

//
// Statement.
//

#[derive(Clone, Debug)]
pub enum Statement {
    //
    // Function application.
    //
    Apply(Rc<Atom>, Box<Statement>, Statements, CallSite),
    Expand(Rc<Atom>, Box<str>, Statements),
    Lambda(Rc<Atom>, Arguments, Statements),
    Operator(Rc<Atom>, Operator),
    //
    // Control structures.
    //
    IfThenElse(
        Rc<Atom>,
        Box<Statement>,
        Box<Statement>,
        Option<Box<Statement>>,
    ),
    Let(Rc<Atom>, Vec<(Box<str>, Statement)>, Statements),
    Prog(Rc<Atom>, Statements),
    //
    // Quotes.
    //
    Backquote(Rc<Atom>, Backquote),
    Quote(Rc<Atom>, Quote),
    //
    // Symbol & value.
    //
    Symbol(Rc<Atom>, Box<str>),
    Value(Rc<Atom>, Value),
}

impl Statement {
    pub fn atom(&self) -> Rc<Atom> {
        match self {
            Statement::Apply(atom, ..)
            | Statement::Expand(atom, ..)
            | Statement::Lambda(atom, ..)
            | Statement::Operator(atom, _)
            | Statement::IfThenElse(atom, ..)
            | Statement::Let(atom, ..)
            | Statement::Prog(atom, _)
            | Statement::Backquote(atom, _)
            | Statement::Quote(atom, _)
            | Statement::Symbol(atom, _)
            | Statement::Value(atom, _) => atom.clone(),
        }
    }

    pub fn closure(&self) -> BTreeSet<Box<str>> {
        match self {
            Statement::Apply(_, sym, stmts, _) => {
                let mut v = sym.closure();
                v.extend(stmts.closure());
                v
            }
            Statement::Lambda(_, args, stmts) => {
                let mut v = stmts.closure();
                args.iter().for_each(|s| {
                    v.remove(s);
                });
                v
            }
            Statement::IfThenElse(_, cond, then, None) => {
                let mut v = cond.closure();
                v.extend(then.closure());
                v
            }
            Statement::IfThenElse(_, cond, then, Some(else_)) => {
                let mut v = cond.closure();
                v.extend(then.closure());
                v.extend(else_.closure());
                v
            }
            Statement::Let(_, bindings, stmts) => {
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
            Statement::Symbol(_, sym) => {
                let mut v = BTreeSet::new();
                v.insert(sym.clone());
                v
            }
            _ => BTreeSet::default(),
        }
    }

    pub fn from_atom(atom: Rc<Atom>, macros: &BTreeSet<Box<str>>) -> Result<Self, Error> {
        match atom.as_ref() {
            Atom::Pair(..) => Self::from_pair(atom.clone(), macros),
            Atom::Symbol(_, sym) => Ok(Self::Symbol(atom.clone(), sym.clone())),
            _ => {
                let val = Value::try_from(atom.clone())?;
                Ok(Self::Value(atom, val))
            }
        }
    }

    fn from_pair(atom: Rc<Atom>, macros: &BTreeSet<Box<str>>) -> Result<Self, Error> {
        //
        // Split the atom.
        //
        let Atom::Pair(_, sym, rem) = atom.as_ref() else {
            unreachable!();
        };
        //
        // Process the atom.
        //
        match sym.as_ref() {
            //
            // For symbols, check its value for built-ins.
            //
            Atom::Symbol(_, name) => match name.as_ref() {
                //
                // Quote: quasiquote.
                //
                "backquote" => {
                    let val = Backquote::from_atom(rem.clone(), macros)?;
                    Ok(Self::Backquote(atom, val))
                }
                //
                // Quote: quote.
                //
                "quote" => {
                    let val = Quote::try_from(rem.clone())?;
                    Ok(Self::Quote(atom, val))
                }
                //
                // Quote: unquote.
                //
                "unquote" => Err(Error::UnquoteOutsideBackquote(atom.span())),
                "unquote-splice" => Err(Error::UnquoteOutsideBackquote(atom.span())),
                //
                // Control flow: if.
                //
                "if" => {
                    //
                    // Unpack the condition.
                    //
                    let Atom::Pair(_, cond, args) = rem.as_ref() else {
                        return Err(Error::ExpectedPair(rem.span()));
                    };
                    //
                    // Parse the condition.
                    //
                    let cond = Statement::from_atom(cond.clone(), macros)?;
                    //
                    // Unpack THEN.
                    //
                    let Atom::Pair(_, then, args) = args.as_ref() else {
                        return Err(Error::ExpectedPair(args.span()));
                    };
                    //
                    // Parse THEN.
                    //
                    let then = Statement::from_atom(then.clone(), macros)?;
                    //
                    // Unpack ELSE.
                    //
                    let else_ = match args.as_ref() {
                        Atom::Nil(_) => None,
                        Atom::Pair(_, else_, _) => {
                            let v = Statement::from_atom(else_.clone(), macros)?;
                            Some(Box::new(v))
                        }
                        _ => return Err(Error::ExpectedPair(args.span())),
                    };
                    //
                    // Done.
                    //
                    Ok(Self::IfThenElse(atom, cond.into(), then.into(), else_))
                }
                //
                // Value binding: let.
                //
                "let" => {
                    //
                    // Split the atom.
                    //
                    let Atom::Pair(_, bindings, stmts) = rem.as_ref() else {
                        return Err(Error::ExpectedPair(rem.span()));
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
                            let Atom::Pair(_, symbol, stmt) = v.as_ref() else {
                                return Err(Error::ExpectedPair(v.span()));
                            };
                            //
                            // Make sure the symbol is a symbol.
                            //
                            let Atom::Symbol(_, symbol) = symbol.as_ref() else {
                                return Err(Error::ExpectedSymbol(symbol.span()));
                            };
                            //
                            // Parse the statement.
                            //
                            let v = Statement::from_atom(stmt.clone(), macros)?;
                            //
                            // Done.
                            //
                            Ok((symbol.clone(), v))
                        })
                        .collect::<Result<_, _>>()?;
                    //
                    // Parse the statements.
                    //
                    let stmts = Statements::from_atom(stmts.clone(), macros)?;
                    //
                    // Done.
                    //
                    Ok(Self::Let(atom, bindings, stmts))
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
                    let Atom::Pair(_, args, rem) = rem.as_ref() else {
                        return Err(Error::ExpectedPair(rem.span()));
                    };
                    //
                    // Build the argument list.
                    //
                    let args: Arguments = args.clone().try_into()?;
                    //
                    // Build the statement list.
                    //
                    let stmts = Statements::from_atom(rem.clone(), macros)?;
                    //
                    // Done.
                    //
                    Ok(Self::Lambda(atom, args, stmts))
                }
                //
                // Prog.
                //
                "prog" => {
                    let stmts = Statements::from_atom(rem.clone(), macros)?;
                    Ok(Self::Prog(atom, stmts))
                }
                //
                // Other symbols.
                //
                v => match Operator::from_str(v) {
                    Ok(op) => {
                        //
                        // Process the arguments.
                        //
                        let stmts = Statements::from_atom(rem.clone(), macros)?;
                        //
                        // If there is enough arguments, generate an operator call.
                        //
                        let stmt = if stmts.len() >= op.arity() {
                            Box::new(Statement::Operator(Atom::symbol(v), op))
                        }
                        //
                        // Otherwise, generate a symbol call.
                        //
                        else {
                            Box::new(Statement::Symbol(sym.clone(), name.clone()))
                        };
                        //
                        // Done.
                        //
                        Ok(Self::Apply(atom, stmt, stmts, CallSite::Any))
                    }
                    Err(_) if macros.contains(v) => {
                        let stmts = Statements::from_atom_quoted(rem.clone(), macros)?;
                        Ok(Self::Expand(atom.clone(), name.clone(), stmts))
                    }
                    Err(_) => {
                        let stmt = Box::new(Statement::Symbol(sym.clone(), name.clone()));
                        let stmts = Statements::from_atom(rem.clone(), macros)?;
                        Ok(Self::Apply(atom, stmt, stmts, CallSite::Any))
                    }
                },
            },
            //
            // For any other type, evaluate the atom.
            //
            _ => {
                let car = Statement::from_atom(sym.clone(), macros).map(Box::new)?;
                let cdr = Statements::from_atom(rem.clone(), macros)?;
                Ok(Self::Apply(atom, car, cdr, CallSite::Any))
            }
        }
    }

    pub fn statements(&self) -> Box<dyn std::iter::Iterator<Item = &Statement> + '_> {
        match self {
            Statement::Apply(_, statement, statements, _) => {
                let iter = Some(statement.as_ref())
                    .into_iter()
                    .chain(statements.iter());
                Box::new(iter)
            }
            Statement::Lambda(_, _, statements) => {
                let iter = statements.iter();
                Box::new(iter)
            }
            Statement::IfThenElse(_, statement, then, else_) => {
                let iter = Some(statement.as_ref())
                    .into_iter()
                    .chain(then.statements())
                    .chain(else_.iter().flat_map(|v| v.statements()));
                Box::new(iter)
            }
            Statement::Let(_, bindings, statements) => {
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
            Statement::Apply(_, stmt, _, location) => {
                if let Statement::Symbol(_, v) = stmt.as_ref()
                    && name == v.as_ref()
                {
                    *location = CallSite::Tail;
                }
            }
            Statement::IfThenElse(_, _, then, else_) => {
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
            Statement::Let(_, _, stmts) => stmts.identify_tail_calls(name),
            _ => (),
        }
    }

    pub fn is_tail_call(&self) -> bool {
        match self {
            Statement::Apply(_, _, _, CallSite::Tail) => true,
            Statement::IfThenElse(_, _, then, else_) => {
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

//
// Statements.
//

#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct Statements(Vec<Statement>);

impl Statements {
    pub fn new(stmts: Vec<Statement>) -> Self {
        Self(stmts)
    }

    pub fn atom(&self) -> Rc<Atom> {
        self.0
            .iter()
            .rev()
            .fold(Atom::nil(), |acc, v| Atom::cons(v.atom(), acc))
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

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
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

    fn from_atom(value: Rc<Atom>, macros: &BTreeSet<Box<str>>) -> Result<Self, Error> {
        let inner = value
            .into_iter()
            .map(|v| Statement::from_atom(v, macros))
            .collect::<Result<_, Error>>()?;
        Ok(Self(inner))
    }

    fn from_atom_quoted(value: Rc<Atom>, macros: &BTreeSet<Box<str>>) -> Result<Self, Error> {
        let inner = value
            .into_iter()
            .map(|v| {
                let atom = Atom::cons(Atom::symbol("quote"), v);
                Statement::from_atom(atom, macros)
            })
            .collect::<Result<_, Error>>()?;
        Ok(Self(inner))
    }
}

//
// Top-level statements.
//

#[derive(Debug)]
pub enum TopLevelStatement {
    Constant(Rc<Atom>, ConstantDefinition),
    External(Rc<Atom>, ExternalDefinition),
    Function(Rc<Atom>, FunctionDefinition),
    Macro(Rc<Atom>, FunctionDefinition),
    Use(Rc<Atom>, Statements),
}

impl TopLevelStatement {
    pub fn atom(&self) -> Rc<Atom> {
        match self {
            Self::Constant(atom, _)
            | Self::External(atom, _)
            | Self::Function(atom, _)
            | Self::Macro(atom, _)
            | Self::Use(atom, _) => atom.clone(),
        }
    }

    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Constant(_, v) => Some(v.name().as_ref()),
            Self::External(_, v) => Some(v.name().as_ref()),
            Self::Function(_, v) => Some(v.name().as_ref()),
            Self::Macro(_, v) => Some(v.name().as_ref()),
            Self::Use(..) => None,
        }
    }

    pub fn closure(&self) -> BTreeSet<Box<str>> {
        match self {
            TopLevelStatement::Function(_, def) | TopLevelStatement::Macro(_, def) => def.closure(),
            _ => Default::default(),
        }
    }

    #[cfg(test)]
    pub fn statements(&self) -> &Statements {
        match self {
            TopLevelStatement::External(..) => panic!("FFI has no statement"),
            TopLevelStatement::Function(_, def) | TopLevelStatement::Macro(_, def) => {
                def.statements()
            }
            TopLevelStatement::Use(_, stmts) => stmts,
            TopLevelStatement::Constant(..) => panic!("Constant has no statement"),
        }
    }

    pub fn from_atom(atom: Rc<Atom>, macros: &BTreeSet<Box<str>>) -> Result<Self, Error> {
        //
        // Split the function call.
        //
        let Atom::Pair(_, sym, rem) = atom.as_ref() else {
            return Err(Error::ExpectedFunctionCall(atom.span()));
        };
        //
        // Get the function symbol.
        //
        let Atom::Symbol(_, name) = sym.as_ref() else {
            return Err(Error::ExpectedSymbol(sym.span()));
        };
        //
        // Process the top-level statement.
        //
        match name.as_ref() {
            "def" => {
                let val = FunctionDefinition::from_atom(rem.clone(), macros)?;
                Ok(Self::Function(atom, val))
            }
            "ext" => {
                let val = ExternalDefinition::from_atom(rem.clone(), macros)?;
                Ok(Self::External(atom, val))
            }
            "mac" => {
                let val = FunctionDefinition::from_atom(rem.clone(), macros)?;
                Ok(Self::Macro(atom, val))
            }
            "use" => {
                let val = Statements::from_atom(rem.clone(), macros)?;
                Ok(Self::Use(atom, val))
            }
            "val" => {
                let val = ConstantDefinition::from_atom(rem.clone(), macros)?;
                Ok(Self::Constant(atom, val))
            }
            _ => Err(Error::ExpectedTopLevelStatement(sym.span())),
        }
    }
}

impl Display for TopLevelStatement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TopLevelStatement::Constant(atom, _)
            | TopLevelStatement::External(atom, _)
            | TopLevelStatement::Function(atom, _)
            | TopLevelStatement::Macro(atom, _)
            | TopLevelStatement::Use(atom, _) => write!(f, "{atom}"),
        }
    }
}

//
// Foreign function definition.
//

#[derive(Clone, Debug, Encode, Decode)]
pub struct ExternalDefinition(Box<str>, Box<str>, ExternalArguments, ExternalType);

impl ExternalDefinition {
    pub fn arity(&self) -> Arity {
        Arity::Some(self.2.0.len() as u16)
    }

    pub fn name(&self) -> &Box<str> {
        &self.0
    }

    pub fn symbol(&self) -> &Box<str> {
        &self.1
    }

    pub fn arguments(&self) -> &ExternalArguments {
        &self.2
    }

    pub fn return_type(&self) -> ExternalType {
        self.3
    }

    pub fn from_atom(atom: Rc<Atom>, _: &BTreeSet<Box<str>>) -> Result<Self, Error> {
        //
        // Extract the function name.
        //
        let Atom::Pair(_, name, rem) = atom.as_ref() else {
            return Err(Error::ExpectedPair(atom.span()));
        };
        //
        // Make sure the function name is a symbol.
        //
        let Atom::Symbol(_, name) = name.as_ref() else {
            return Err(Error::ExpectedSymbol(name.span()));
        };
        //
        // Extract the symbol name.
        //
        let Atom::Pair(_, symbol, rem) = rem.as_ref() else {
            return Err(Error::ExpectedPair(rem.span()));
        };
        //
        // Check the symbol name.
        //
        let symbol = match symbol.as_ref() {
            Atom::Symbol(_, v) => v.clone(),
            Atom::Nil(_) => name.clone(),
            _ => return Err(Error::ExpectedSymbol(symbol.span())),
        };
        //
        // Extract the arguments.
        //
        let Atom::Pair(_, args, rem) = rem.as_ref() else {
            return Err(Error::ExpectedPair(rem.span()));
        };
        //
        // Build the argument list.
        //
        let args: ExternalArguments = args.clone().try_into()?;
        //
        // Check if there is a comment.
        //
        let Atom::Pair(_, maybe_comment, return_type) = rem.as_ref() else {
            return Err(Error::ExpectedPair(rem.span()));
        };
        //
        // Skip the comment if any.
        //
        let rem = match maybe_comment.as_ref() {
            Atom::String(..) => return_type,
            _ => rem,
        };
        //
        // Get the return type.
        //
        let Atom::Pair(_, return_type, _) = rem.as_ref() else {
            return Err(Error::ExpectedPair(rem.span()));
        };
        //
        // Get the return type.
        //
        let rtyp: ExternalType = return_type.clone().try_into()?;
        /*
         * Done.
         */
        Ok(Self(name.clone(), symbol, args, rtyp))
    }
}

//
// Function definition.
//

#[derive(Debug)]
pub struct FunctionDefinition(Box<str>, Arguments, Statements);

impl FunctionDefinition {
    pub fn new(name: Box<str>, args: Arguments, stmts: Statements) -> Self {
        Self(name, args, stmts)
    }

    pub fn name(&self) -> &Box<str> {
        &self.0
    }

    pub fn arguments(&self) -> &Arguments {
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

    pub fn from_atom(atom: Rc<Atom>, macros: &BTreeSet<Box<str>>) -> Result<Self, Error> {
        //
        // Extract the function name.
        //
        let Atom::Pair(_, name, rem) = atom.as_ref() else {
            return Err(Error::ExpectedPair(atom.span()));
        };
        //
        // Make sure the function name is a symbol.
        //
        let Atom::Symbol(_, name) = name.as_ref() else {
            return Err(Error::ExpectedSymbol(name.span()));
        };
        //
        // Extract the arguments.
        //
        let Atom::Pair(_, args, rem) = rem.as_ref() else {
            return Err(Error::ExpectedPair(rem.span()));
        };
        //
        // Build the argument list.
        //
        let args: Arguments = args.clone().try_into()?;
        //
        // Check if there is a comment.
        //
        let Atom::Pair(_, maybe_comment, statements) = rem.as_ref() else {
            return Err(Error::ExpectedPair(rem.span()));
        };
        //
        // Skip the comment if any.
        //
        let rem = match maybe_comment.as_ref() {
            Atom::Nil(_) | Atom::String(..) => statements,
            _ => rem,
        };
        //
        // Build the statement list.
        //
        let mut stmts = Statements::from_atom(rem.clone(), macros)?;
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

//
// Value definition.
//

#[derive(Debug)]
pub struct ConstantDefinition(Box<str>, Value);

impl ConstantDefinition {
    pub fn new(name: Box<str>, value: Value) -> Self {
        Self(name, value)
    }

    pub fn name(&self) -> &Box<str> {
        &self.0
    }

    pub fn value(&self) -> &Value {
        &self.1
    }

    pub fn from_atom(atom: Rc<Atom>, _: &BTreeSet<Box<str>>) -> Result<Self, Error> {
        //
        // Extract the value name.
        //
        let Atom::Pair(_, name, rem) = atom.as_ref() else {
            return Err(Error::ExpectedPair(atom.span()));
        };
        //
        // Make sure the value name is a symbol.
        //
        let Atom::Symbol(_, name) = name.as_ref() else {
            return Err(Error::ExpectedSymbol(name.span()));
        };
        //
        // Extract the value.
        //
        let Atom::Pair(_, value, _) = rem.as_ref() else {
            return Err(Error::ExpectedPair(rem.span()));
        };
        //
        // Build the value.
        //
        let value = Value::try_from(value.clone())?;
        //
        // Done.
        //
        Ok(Self(name.clone(), value))
    }
}
