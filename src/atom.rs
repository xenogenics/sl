use std::rc::Rc;

use crate::{error::Error, heap, opcodes, stack};

//
// Atom.
//

pub enum Atom {
    Nil,
    True,
    Char(u8),
    Number(i64),
    Pair(Rc<Atom>, Rc<Atom>),
    String(Box<str>),
    Symbol(Box<str>),
    Wildcard,
}

impl std::fmt::Debug for Atom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Atom::Nil => write!(f, "nil"),
            Atom::True => write!(f, "t"),
            Atom::Char(v) => write!(f, "char({v})"),
            Atom::Number(v) => write!(f, "number({v})"),
            Atom::Pair(a, b) => write!(f, "({a:?} {b:?})"),
            Atom::String(v) => write!(f, "string({v})"),
            Atom::Symbol(v) => write!(f, "symbol({v})"),
            Atom::Wildcard => write!(f, "_"),
        }
    }
}

//
// Constructors.
//

impl Atom {
    pub fn char(v: u8) -> Rc<Self> {
        Self::Char(v).into()
    }

    pub fn nil() -> Rc<Atom> {
        Self::Nil.into()
    }

    pub fn number(v: i64) -> Rc<Atom> {
        Self::Number(v).into()
    }

    pub fn string(v: &str) -> Rc<Atom> {
        let mut result = String::new();
        /*
         * Trim the double quotes.
         */
        let v = v.trim_matches('"');
        /*
         * Parse the escape sequences.
         */
        let mut chars = v.chars();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some('0') => result.push('\0'),
                    Some('e') => result.push('\x1B'),
                    Some('n') => result.push('\n'),
                    Some('r') => result.push('\r'),
                    Some('t') => result.push('\t'),
                    Some('"') => result.push('"'),
                    Some('\\') => result.push('\\'),
                    Some(c) => result.push(c), // Handle invalid escape
                    None => break,
                }
            } else {
                result.push(c);
            }
        }
        /*
         * Done.
         */
        Self::String(result.into_boxed_str()).into()
    }

    pub fn symbol(v: &str) -> Rc<Atom> {
        Self::Symbol(v.into()).into()
    }

    pub fn t() -> Rc<Atom> {
        Self::True.into()
    }

    pub fn wildcard() -> Rc<Atom> {
        Self::Wildcard.into()
    }
}

//
// Predicates.
//

impl Atom {
    pub const fn is_nil(&self) -> bool {
        matches!(self, Atom::Nil)
    }

    pub const fn is_pair(&self) -> bool {
        matches!(self, Atom::Pair(..))
    }
}

//
// List operations.
//

impl Atom {
    pub fn conc(a: Rc<Atom>, b: Rc<Atom>) -> Rc<Atom> {
        match a.as_ref() {
            Atom::Pair(car, cdr) => Atom::cons(car.clone(), Atom::conc(cdr.clone(), b)),
            _ => b,
        }
    }

    pub fn cons(a: Rc<Atom>, b: Rc<Atom>) -> Rc<Atom> {
        Self::Pair(a, b).into()
    }

    pub fn iter(self: &Rc<Atom>) -> impl std::iter::Iterator<Item = Rc<Atom>> {
        AtomIterator(self.clone())
    }

    #[allow(clippy::should_implement_trait)]
    pub fn into_iter(self: Rc<Atom>) -> impl std::iter::Iterator<Item = Rc<Atom>> {
        AtomIterator(self)
    }
}

//
// Display.
//

impl Atom {
    fn fmt_pair(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Atom::Nil => Ok(()),
            Atom::Pair(car, cdr) => {
                if cdr.is_nil() {
                    write!(f, "{car}")
                } else {
                    write!(f, "{car} ")?;
                    cdr.fmt_pair(f)
                }
            }
            v => write!(f, ". {v}"),
        }
    }
}

impl std::fmt::Display for Atom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Atom::Nil => write!(f, "nil"),
            Atom::True => write!(f, "t"),
            Atom::Char(c) => write!(f, "^{}", *c as char),
            Atom::Number(n) => write!(f, "{n}"),
            Atom::Pair(..) => {
                write!(f, "(")?;
                self.fmt_pair(f)?;
                write!(f, ")")
            }
            Atom::String(v) => write!(f, "\"{v}\""),
            Atom::Symbol(v) => write!(f, "{v}"),
            Atom::Wildcard => write!(f, "_"),
        }
    }
}

//
// Iterator.
//

pub struct AtomIterator(Rc<Atom>);

impl std::iter::Iterator for AtomIterator {
    type Item = Rc<Atom>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0.as_ref() {
            Atom::Pair(car, cdr) => {
                let value = car.clone();
                self.0 = cdr.clone();
                Some(value)
            }
            _ => None,
        }
    }
}

//
// From<opcodes::Immediate>.
//
//

impl TryFrom<opcodes::Immediate> for Rc<Atom> {
    type Error = Error;

    fn try_from(value: opcodes::Immediate) -> Result<Self, Self::Error> {
        match value {
            opcodes::Immediate::Nil => Ok(Atom::Nil.into()),
            opcodes::Immediate::True => Ok(Atom::True.into()),
            opcodes::Immediate::Char(v) => Ok(Atom::Char(v).into()),
            opcodes::Immediate::Number(v) => Ok(Atom::Number(v).into()),
            opcodes::Immediate::Symbol(v) => {
                let index = v.iter().position(|v| *v == 0).unwrap_or(v.len());
                let value = String::from_utf8_lossy(&v[0..index]).to_string();
                Ok(Atom::Symbol(value.into_boxed_str()).into())
            }
            opcodes::Immediate::Wildcard => Ok(Atom::Wildcard.into()),
            _ => todo!(),
        }
    }
}

//
// TryFrom<stack::Value>.
//

impl TryFrom<heap::Value> for Rc<Atom> {
    type Error = Error;

    fn try_from(value: heap::Value) -> Result<Self, Self::Error> {
        match value {
            heap::Value::Immediate(v) => v.try_into(),
            heap::Value::Pair(car, cdr) => {
                let car = car.as_ref().clone().try_into()?;
                let cdr = cdr.as_ref().clone().try_into()?;
                Ok(Atom::Pair(car, cdr).into())
            }
            _ => Err(Error::ExpectedPairOrImmediate),
        }
    }
}

//
// TryFrom<stack::Value>.
//

impl TryFrom<stack::Value> for Rc<Atom> {
    type Error = Error;

    fn try_from(value: stack::Value) -> Result<Self, Self::Error> {
        match value {
            stack::Value::Heap(v) => v.as_ref().clone().try_into(),
            stack::Value::Immediate(v) => v.try_into(),
            _ => Err(Error::ExpectedPairOrImmediate),
        }
    }
}
