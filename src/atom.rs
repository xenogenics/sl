use std::{ops::Add, rc::Rc, str::Chars};

use crate::{error::Error, heap, opcodes, stack};

//
// Location.
//

#[derive(Clone, Copy)]
pub enum Span {
    None,
    Offset(usize, usize),
}

impl Add for Span {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Span::None, Span::None)
            | (Span::None, Span::Offset(_, _))
            | (Span::Offset(_, _), Span::None) => Span::None,
            (Span::Offset(a, _), Span::Offset(_, b)) => Span::Offset(a, b),
        }
    }
}

impl std::fmt::Debug for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Span::None => Ok(()),
            Span::Offset(a, b) => write!(f, "[{a}..{b}]"),
        }
    }
}

//
// Atom.
//

pub enum Atom {
    Nil(Span),
    True(Span),
    Char(Span, u8),
    Number(Span, i64),
    Pair(Span, Rc<Atom>, Rc<Atom>),
    String(Span, Box<str>),
    Symbol(Span, Box<str>),
    Wildcard(Span),
}

//
// Location.
//

impl Atom {
    pub fn span(&self) -> Span {
        match self {
            Atom::Nil(loc)
            | Atom::True(loc)
            | Atom::Char(loc, _)
            | Atom::Number(loc, _)
            | Atom::Pair(loc, ..)
            | Atom::String(loc, _)
            | Atom::Symbol(loc, _)
            | Atom::Wildcard(loc) => *loc,
        }
    }

    pub fn with_span(&self, loc: Span) -> Rc<Self> {
        match self {
            Atom::Nil(_) => Atom::Nil(loc).into(),
            Atom::True(_) => Atom::True(loc).into(),
            Atom::Char(_, v) => Atom::Char(loc, *v).into(),
            Atom::Number(_, v) => Atom::Number(loc, *v).into(),
            Atom::Pair(_, car, cdr) => Atom::Pair(loc, car.clone(), cdr.clone()).into(),
            Atom::String(_, v) => Atom::String(loc, v.clone()).into(),
            Atom::Symbol(_, v) => Atom::Symbol(loc, v.clone()).into(),
            Atom::Wildcard(_) => Atom::Wildcard(loc).into(),
        }
    }
}

impl std::fmt::Debug for Atom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Atom::Nil(loc) => write!(f, "{loc:?}nil"),
            Atom::True(loc) => write!(f, "{loc:?}t"),
            Atom::Char(loc, v) => write!(f, "{loc:?}char({v})"),
            Atom::Number(loc, v) => write!(f, "{loc:?}number({v})"),
            Atom::Pair(loc, a, b) => write!(f, "{loc:?}({a:?} {b:?})"),
            Atom::String(loc, v) => write!(f, "{loc:?}string({v})"),
            Atom::Symbol(loc, v) => write!(f, "{loc:?}symbol({v})"),
            Atom::Wildcard(loc) => write!(f, "{loc:?}_"),
        }
    }
}

//
// Constructors.
//

impl Atom {
    pub fn char(v: u8) -> Rc<Self> {
        Self::Char(Span::None, v).into()
    }

    pub fn nil() -> Rc<Atom> {
        Self::Nil(Span::None).into()
    }

    pub fn number(v: i64) -> Rc<Atom> {
        Self::Number(Span::None, v).into()
    }

    pub fn string(v: &str) -> Rc<Atom> {
        let mut result = String::new();
        /*
         * Trim the double quotes.
         */
        let v = v.trim_matches('"');
        /*
         * Get the next character.
         */
        let mut chars = v.chars();
        while let Some(c) = Self::next_char(&mut chars) {
            result.push(c);
        }
        /*
         * Done.
         */
        Self::String(Span::None, result.into_boxed_str()).into()
    }

    pub fn symbol(v: &str) -> Rc<Atom> {
        Self::Symbol(Span::None, v.into()).into()
    }

    pub fn t() -> Rc<Atom> {
        Self::True(Span::None).into()
    }

    pub fn wildcard() -> Rc<Atom> {
        Self::Wildcard(Span::None).into()
    }

    pub fn next_char(chars: &mut Chars<'_>) -> Option<char> {
        chars.next().and_then(|c| {
            if c == '\\' {
                match chars.next() {
                    Some('0') => Some('\0'),
                    Some('e') => Some('\x1B'),
                    Some('n') => Some('\n'),
                    Some('r') => Some('\r'),
                    Some('s') => Some(' '),
                    Some('t') => Some('\t'),
                    Some('"') => Some('"'),
                    Some('\\') => Some('\\'),
                    v => v,
                }
            } else {
                Some(c)
            }
        })
    }
}

//
// Predicates.
//

impl Atom {
    pub const fn is_nil(&self) -> bool {
        matches!(self, Atom::Nil(_))
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
            Atom::Pair(_, car, cdr) => {
                let inloc = cdr.span() + b.span();
                let outloc = car.span() + b.span();
                let inner = Atom::conc(cdr.clone(), b).with_span(inloc);
                Atom::cons(car.clone(), inner).with_span(outloc)
            }
            _ => b,
        }
    }

    pub fn cons(a: Rc<Atom>, b: Rc<Atom>) -> Rc<Atom> {
        Self::Pair(Span::None, a, b).into()
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
            Atom::Nil(_) => Ok(()),
            Atom::Pair(_, car, cdr) => {
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
            Atom::Nil(_) => write!(f, "nil"),
            Atom::True(_) => write!(f, "T"),
            Atom::Char(_, c) => match *c as char {
                '\0' => write!(f, "^\\0"),
                '\x1B' => write!(f, "^\\e"),
                '\n' => write!(f, "^\\n"),
                '\r' => write!(f, "^\\r"),
                ' ' => write!(f, "^\\s"),
                '\t' => write!(f, "^\\t"),
                '"' => write!(f, "^\""),
                '\\' => write!(f, "^\\"),
                _ => write!(f, "^{}", *c as char),
            },
            Atom::Number(_, n) => write!(f, "{n}"),
            Atom::Pair(..) => {
                write!(f, "(")?;
                self.fmt_pair(f)?;
                write!(f, ")")
            }
            Atom::String(_, v) => write!(f, "\"{v}\""),
            Atom::Symbol(_, v) => write!(f, "{v}"),
            Atom::Wildcard(_) => write!(f, "_"),
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
            Atom::Pair(_, car, cdr) => {
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
            opcodes::Immediate::Nil => Ok(Atom::Nil(Span::None).into()),
            opcodes::Immediate::True => Ok(Atom::True(Span::None).into()),
            opcodes::Immediate::Char(v) => Ok(Atom::Char(Span::None, v).into()),
            opcodes::Immediate::Number(v) => Ok(Atom::Number(Span::None, v).into()),
            opcodes::Immediate::Symbol(v) => {
                let index = v.iter().position(|v| *v == 0).unwrap_or(v.len());
                let value = String::from_utf8_lossy(&v[0..index]).to_string();
                Ok(Atom::Symbol(Span::None, value.into_boxed_str()).into())
            }
            opcodes::Immediate::Wildcard => Ok(Atom::Wildcard(Span::None).into()),
            _ => todo!(),
        }
    }
}

//
// TryFrom<heap::Value>.
//

impl TryFrom<heap::Value> for Rc<Atom> {
    type Error = Error;

    fn try_from(value: heap::Value) -> Result<Self, Self::Error> {
        match value {
            heap::Value::Immediate(v) => v.try_into(),
            heap::Value::Pair(car, cdr) => {
                let car = car.as_ref().clone().try_into()?;
                let cdr = cdr.as_ref().clone().try_into()?;
                Ok(Atom::Pair(Span::None, car, cdr).into())
            }
            heap::Value::Bytes(v) => {
                let value = v
                    .iter()
                    .rev()
                    .fold(Atom::nil(), |acc, v| Atom::cons(Atom::char(*v), acc));
                Ok(value)
            }
            heap::Value::String(v) => {
                let value = v.as_c_str().to_str().map_err(|_| Error::InvalidString)?;
                Ok(Atom::string(value))
            }
            _ => Err(Error::ExpectedPairOrImmediate(Span::None)),
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
            _ => Err(Error::ExpectedPairOrImmediate(Span::None)),
        }
    }
}
