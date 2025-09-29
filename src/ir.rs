use std::{collections::BTreeSet, fmt::Display, rc::Rc, str::FromStr};

use bincode::{Decode, Encode};
use strum_macros::{Display, EnumIter, EnumString};

use crate::{atom::Atom, error::Error, opcodes::Arity};

//
// Binding.
//

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Binding {
    Deconstructed(Vec<(Box<str>, Vec<Operator>)>),
    Passthrough(Box<str>),
}

impl Binding {
    pub fn has_deconstructions(&self) -> bool {
        matches!(self, Self::Deconstructed(_))
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Deconstructed(v) => v.len(),
            Self::Passthrough(_) => 1,
        }
    }

    pub fn names(&self) -> Box<dyn std::iter::DoubleEndedIterator<Item = &Box<str>> + '_> {
        match self {
            Self::Deconstructed(v) => Box::new(v.iter().map(|v| &v.0)),
            Self::Passthrough(v) => Box::new(Some(v).into_iter()),
        }
    }

    fn from_atom(atom: Rc<Atom>) -> Result<Self, Error> {
        match atom.as_ref() {
            Atom::Pair(..) => {
                let mut result = vec![];
                Self::collect(atom, vec![], &mut result)?;
                Ok(Self::Deconstructed(result))
            }
            Atom::Symbol(_, v) if Keyword::from_str(v).is_ok() => {
                Err(Error::ReservedKeyword(v.clone()))
            }
            Atom::Symbol(_, v) => Ok(Self::Passthrough(v.clone())),
            _ => Err(Error::ExpectedPairOrSymbol(atom.span())),
        }
    }

    fn collect(
        atom: Rc<Atom>,
        ops: Vec<Operator>,
        res: &mut Vec<(Box<str>, Vec<Operator>)>,
    ) -> Result<(), Error> {
        match atom.as_ref() {
            Atom::Pair(_, car, cdr) => {
                //
                // Process the CAR branch.
                //
                let mut next = ops.clone();
                next.push(Operator::Car);
                Self::collect(car.clone(), next, res)?;
                //
                // Process the CDR branch.
                //
                let mut next = ops.clone();
                next.push(Operator::Cdr);
                Self::collect(cdr.clone(), next, res)?;
                //
                // Done.
                //
                Ok(())
            }
            Atom::Symbol(_, v) => {
                res.push((v.clone(), ops));
                Ok(())
            }
            Atom::Nil(_) | Atom::Wildcard(_) => Ok(()),
            _ => {
                println!("{atom}");
                Err(Error::ExpectedPairOrSymbol(atom.span()))
            }
        }
    }
}

//
// Bindings.
//

#[derive(Clone, Debug)]
pub struct Bindings(Vec<(Binding, Statement)>);

impl Bindings {
    pub fn has_deconstructions(&self) -> bool {
        self.0
            .iter()
            .map(|(v, _)| v.has_deconstructions())
            .reduce(|a, b| a || b)
            .unwrap_or_default()
    }

    pub fn iter(&self) -> impl std::iter::DoubleEndedIterator<Item = &(Binding, Statement)> {
        self.0.iter()
    }
}

//
// Arguments.
//

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Arguments {
    Capture(Box<str>),
    List(Vec<Binding>),
    ListAndCapture(Vec<Binding>, Box<str>),
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

    pub fn names(&self) -> Box<dyn std::iter::DoubleEndedIterator<Item = &Box<str>> + '_> {
        match self {
            Arguments::Capture(v) => Box::new(Some(v).into_iter()),
            Arguments::List(items) => Box::new(items.iter().flat_map(|v| v.names())),
            Arguments::ListAndCapture(items, last) => {
                Box::new(items.iter().flat_map(|v| v.names()).chain(Some(last)))
            }
            Arguments::None => Box::new(None.iter()),
        }
    }

    pub fn bindings(&self) -> Box<dyn std::iter::DoubleEndedIterator<Item = &Binding> + '_> {
        match self {
            Arguments::List(groups) | Arguments::ListAndCapture(groups, _) => {
                Box::new(groups.iter())
            }
            _ => Box::new(None.into_iter()),
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

    pub fn has_deconstructions(&self) -> bool {
        match self {
            Arguments::List(groups) | Arguments::ListAndCapture(groups, _) => groups
                .iter()
                .map(|v| v.has_deconstructions())
                .reduce(|a, b| a || b)
                .unwrap_or_default(),
            _ => false,
        }
    }

    fn from_pair(atom: Rc<Atom>, mut bindings: Vec<Binding>) -> Result<Self, Error> {
        /*
         * Split the atom.
         */
        let Atom::Pair(_, car, cdr) = atom.as_ref() else {
            return Err(Error::ExpectedPair(atom.span()));
        };
        /*
         * Save the argumetn group.
         */
        let binding = Binding::from_atom(car.clone())?;
        bindings.push(binding);
        /*
         * Check CDR.
         */
        match cdr.as_ref() {
            Atom::Nil(_) => Ok(Self::List(bindings)),
            Atom::Pair(..) => Self::from_pair(cdr.clone(), bindings),
            Atom::Symbol(_, v) => Ok(Self::ListAndCapture(bindings, v.clone())),
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
// Keywords.
//

#[derive(EnumString)]
#[strum(serialize_all = "lowercase")]
enum Keyword {
    Def,
    Ext,
    If,
    Let,
    Mac,
    Prog,
    #[strum(serialize = "self")]
    This,
    Val,
}

//
// Built-in operator.
//

#[derive(Clone, Copy, Debug, Display, EnumIter, EnumString, Eq, PartialEq)]
pub enum Operator {
    #[strum(serialize = "apply")]
    Apply,
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
    #[strum(serialize = "%")]
    Mod,
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
    // Bytes, string, and symbol operations.
    //
    #[strum(serialize = "bytes")]
    Bytes,
    #[strum(serialize = "chr")]
    Chr,
    #[strum(serialize = "str")]
    Str,
    #[strum(serialize = "sym")]
    Sym,
    #[strum(serialize = "unpack")]
    Unpack,
    //
    // Predicates.
    //
    #[strum(serialize = "byt?")]
    IsByt,
    #[strum(serialize = "chr?")]
    IsChr,
    #[strum(serialize = "num?")]
    IsNum,
    #[strum(serialize = "lst?")]
    IsLst,
    #[strum(serialize = "nil?")]
    IsNil,
    #[strum(serialize = "str?")]
    IsStr,
    #[strum(serialize = "sym?")]
    IsSym,
    #[strum(serialize = "tru?")]
    IsTru,
    #[strum(serialize = "wld?")]
    IsWld,
}

impl Operator {
    pub fn argument_count(&self) -> usize {
        match self {
            Operator::Apply => 2,
            Operator::Add => 2,
            Operator::Sub => 2,
            Operator::Mul => 2,
            Operator::Div => 2,
            Operator::Mod => 2,
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
            Operator::Chr => 1,
            Operator::Str => 1,
            Operator::Sym => 1,
            Operator::Unpack => 1,
            Operator::IsByt => 1,
            Operator::IsChr => 1,
            Operator::IsNum => 1,
            Operator::IsLst => 1,
            Operator::IsNil => 1,
            Operator::IsStr => 1,
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
    Let(Rc<Atom>, Bindings, Statements),
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
    This(Rc<Atom>),
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
            | Statement::This(atom)
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
                args.names().for_each(|s| {
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
                let mut v = bindings.iter().fold(BTreeSet::new(), |mut acc, (b, v)| {
                    let mut v = v.closure();
                    args.iter().for_each(|s| {
                        v.remove(s);
                    });
                    args.extend(b.names().cloned());
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
                            let Atom::Pair(_, binding, stmt) = v.as_ref() else {
                                return Err(Error::ExpectedPair(v.span()));
                            };
                            //
                            // Parse the binding.
                            //
                            let binding = Binding::from_atom(binding.clone())?;
                            //
                            // Parse the statement.
                            //
                            let v = Statement::from_atom(stmt.clone(), macros)?;
                            //
                            // Done.
                            //
                            Ok((binding, v))
                        })
                        .collect::<Result<_, _>>()?;
                    //
                    // Parse the statements.
                    //
                    let stmts = Statements::from_atom(stmts.clone(), macros)?;
                    //
                    // Done.
                    //
                    Ok(Self::Let(atom, Bindings(bindings), stmts))
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
                    Ok(Operator::Or) => {
                        //
                        // Process the arguments.
                        //
                        let args = Statements::from_atom(rem.clone(), macros)?;
                        //
                        // Check the number of arguments.
                        //
                        if args.len() > 2 {
                            return Err(Error::TooManyArguments(atom.span()));
                        }
                        //
                        // If there is enough arguments, generate the call.
                        //
                        if args.len() == 2 {
                            //
                            // Check if the second argument requires some kind
                            // of complex evaluation.
                            //
                            let short_circuit = match args.0[1] {
                                Statement::Apply(..)
                                | Statement::Expand(..)
                                | Statement::Lambda(..)
                                | Statement::IfThenElse(..)
                                | Statement::Let(..)
                                | Statement::Prog(..) => true,
                                Statement::Operator(..)
                                | Statement::Backquote(..)
                                | Statement::Quote(..)
                                | Statement::Symbol(..)
                                | Statement::This(_)
                                | Statement::Value(..) => false,
                            };
                            //
                            // If so, generate a short circuit.
                            //
                            if short_circuit {
                                //
                                // Build the atom.
                                //
                                let atom = Atom::cons(
                                    Atom::symbol("if"),
                                    Atom::cons(
                                        args.0[0].atom(),
                                        Atom::cons(
                                            Atom::t(),
                                            Atom::cons(args.0[1].atom(), Atom::nil()),
                                        ),
                                    ),
                                );
                                //
                                // Parse the expression.
                                //
                                Statement::from_atom(atom, macros)
                            }
                            //
                            // Otherwise, generate the operator call.
                            //
                            else {
                                let stmt = Statement::Operator(Atom::symbol(v), Operator::Or);
                                Ok(Self::Apply(atom, stmt.into(), args, CallSite::Any))
                            }
                        }
                        //
                        // Otherwise, generate a symbol application.
                        //
                        else {
                            let stmt = Statement::Symbol(sym.clone(), name.clone());
                            Ok(Self::Apply(atom, stmt.into(), args, CallSite::Any))
                        }
                    }
                    Ok(op) => {
                        //
                        // Process the arguments.
                        //
                        let args = Statements::from_atom(rem.clone(), macros)?;
                        //
                        // Check the number of arguments.
                        //
                        if args.len() > op.argument_count() {
                            return Err(Error::TooManyArguments(atom.span()));
                        }
                        //
                        // If there is enough arguments, generate an operator call.
                        //
                        let stmt = if args.len() == op.argument_count() {
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
                        Ok(Self::Apply(atom, stmt, args, CallSite::Any))
                    }
                    Err(_) if macros.contains(v) => {
                        let stmts = Statements::from_atom_quoted(rem.clone(), macros)?;
                        Ok(Self::Expand(atom.clone(), name.clone(), stmts))
                    }
                    Err(_) if v == "self" => {
                        let stmt = Box::new(Statement::This(sym.clone()));
                        let stmts = Statements::from_atom(rem.clone(), macros)?;
                        Ok(Self::Apply(atom, stmt, stmts, CallSite::Any))
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

    fn identify_tail_calls(&mut self, name: &str, args: &Arguments) {
        match self {
            Statement::Apply(_, stmt, vals, location) => {
                //
                // Check if the application is a candidate for TCO.
                //
                let is_candidate = match stmt.as_ref() {
                    Statement::This(_) => true,
                    Statement::Symbol(_, v) => v.as_ref() == name,
                    _ => false,
                };
                //
                // Mark the candidate.
                //
                if is_candidate {
                    match args.arity() {
                        Arity::All => {
                            *location = CallSite::Tail;
                        }
                        Arity::Some(n) if vals.len() == n as usize => {
                            *location = CallSite::Tail;
                        }
                        Arity::SomeWithRem(n) if vals.len() >= n as usize => {
                            *location = CallSite::Tail;
                        }
                        Arity::None if vals.is_empty() => {
                            *location = CallSite::Tail;
                        }
                        _ => (),
                    }
                }
            }
            Statement::IfThenElse(_, _, then, else_) => {
                //
                // Process THEN.
                //
                then.identify_tail_calls(name, args);
                //
                // Process ELSE.
                //
                if let Some(else_) = else_.as_mut() {
                    else_.identify_tail_calls(name, args);
                }
            }
            Statement::Let(_, _, stmts) => stmts.identify_tail_calls(name, args),
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

    fn identify_tail_calls(&mut self, name: &str, args: &Arguments) {
        //
        // Get the last statement.
        //
        let Some(stmt) = self.0.last_mut() else {
            return;
        };
        //
        // Identify tail calls on the last statement.
        //
        stmt.identify_tail_calls(name, args);
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
        // Make sure it's not a reserved keyword.
        //
        if Keyword::from_str(name).is_ok() {
            return Err(Error::ReservedKeyword(name.clone()));
        }
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
        self.1.names().for_each(|s| {
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
        // Make sure it's not a reserved keyword.
        //
        if Keyword::from_str(name).is_ok() {
            return Err(Error::ReservedKeyword(name.clone()));
        }
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
        stmts.identify_tail_calls(name.as_ref(), &args);
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
pub struct ConstantDefinition(Box<str>, Quote);

impl ConstantDefinition {
    pub fn new(name: Box<str>, value: Quote) -> Self {
        Self(name, value)
    }

    pub fn name(&self) -> &Box<str> {
        &self.0
    }

    pub fn value(&self) -> &Quote {
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
        // Make sure it's not a reserved keyword.
        //
        if Keyword::from_str(name).is_ok() {
            return Err(Error::ReservedKeyword(name.clone()));
        }
        //
        // Extract the value.
        //
        let Atom::Pair(_, value, _) = rem.as_ref() else {
            return Err(Error::ExpectedPair(rem.span()));
        };
        //
        // Build the value.
        //
        let value = Quote::try_from(value.clone())?;
        //
        // Done.
        //
        Ok(Self(name.clone(), value))
    }
}
