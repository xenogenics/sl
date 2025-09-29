use std::rc::Rc;

use bincode::{Decode, Encode};

use crate::{atom::Atom, error::Error};

//
// Arity.
//

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Encode, Decode)]
#[repr(u32)]
pub enum Arity {
    #[default]
    All,
    Some(u16),
    SomeWithRem(u16),
    None,
}

impl std::fmt::Display for Arity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Arity::All => write!(f, "*"),
            Arity::Some(n) => write!(f, "{n}"),
            Arity::SomeWithRem(n) => write!(f, "{n}+"),
            Arity::None => write!(f, "0"),
        }
    }
}

//
// Immediate values.
//

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode)]
pub enum Immediate {
    Nil,
    True,
    Char(u8),
    Number(i64),
    Extcall(u32),
    Funcall(u32, Arity),
    Symbol([u8; 15]),
    Wildcard,
}

impl Immediate {
    pub const fn extcall(idx: usize) -> Self {
        Self::Extcall(idx as u32)
    }

    pub const fn funcall(idx: usize, arity: Arity) -> Self {
        Self::Funcall(idx as u32, arity)
    }

    pub const fn as_funcall(&self) -> (u32, Arity) {
        match self {
            Immediate::Funcall(idx, cnt) => (*idx, *cnt),
            _ => panic!("Expected a funcall"),
        }
    }

    pub const fn as_number(&self) -> i64 {
        match self {
            Immediate::Char(v) => *v as i64,
            Immediate::Number(v) => *v,
            _ => panic!("Expected a number"),
        }
    }
}

impl std::fmt::Display for Immediate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Immediate::Nil => write!(f, "nil"),
            Immediate::True => write!(f, "T"),
            Immediate::Char(c) => match *c as char {
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

            Immediate::Number(n) => write!(f, "{n}"),
            Immediate::Extcall(v) => write!(f, "#X({v})",),
            Immediate::Funcall(v, arity) => write!(f, "#F({v},{arity})"),
            Immediate::Symbol(v) => {
                let len = v.iter().position(|v| *v == 0).unwrap_or(v.len());
                let sym = unsafe { std::str::from_utf8_unchecked(&v[..len]) };
                write!(f, "{sym}")
            }
            Immediate::Wildcard => write!(f, "_"),
        }
    }
}

impl From<bool> for Immediate {
    fn from(value: bool) -> Self {
        if value { Self::True } else { Self::Nil }
    }
}

impl From<i64> for Immediate {
    fn from(value: i64) -> Self {
        Self::Number(value)
    }
}

impl TryFrom<Rc<Atom>> for Immediate {
    type Error = Error;

    fn try_from(value: Rc<Atom>) -> Result<Self, Self::Error> {
        match value.as_ref() {
            Atom::Nil(_) => Ok(Self::Nil),
            Atom::True(_) => Ok(Self::True),
            Atom::Char(_, v) => Ok(Self::Char(*v)),
            Atom::Number(_, v) => Ok(Self::Number(*v)),
            Atom::Symbol(_, v) => {
                let bytes = v.as_bytes();
                let mut raw = [0; 15];
                raw[0..bytes.len()].copy_from_slice(bytes);
                Ok(Immediate::Symbol(raw))
            }
            Atom::Wildcard(_) => Ok(Self::Wildcard),
            _ => Err(Error::ExpectedImmediate(value.span())),
        }
    }
}

//
// Opcodes.
//

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode)]
#[repr(u8)]
pub enum OpCode {
    //
    // Function application.
    //
    Apply,
    //
    // Arithmetics.
    //
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Ge,
    Gt,
    Le,
    Lt,
    //
    // Logic operations.
    //
    And,
    Equ,
    Neq,
    Not,
    Or,
    //
    // Bit operations.
    //
    BitAnd,
    BitNot,
    BitOr,
    BitXor,
    //
    // List operations.
    //
    Car,
    Cdr,
    Conc,
    Cons,
    //
    // Bytes, string, and symbol operations.
    //
    Bytes,
    Chr,
    Str,
    Sym,
    Unpack,
    //
    // Predicates.
    //
    IsByt,
    IsChr,
    IsLst,
    IsNil,
    IsNum,
    IsStr,
    IsSym,
    IsTru,
    IsWld,
    //
    // Control flow.
    //
    Br(isize),
    Brn(isize),
    Call(usize),
    Ret,
    //
    // Stack operations.
    //
    Dup(usize),
    Get(usize),
    Lst(usize),
    Pak(usize, usize),
    Pop(usize),
    Psh(Immediate),
    Rot(usize),
    Rtm(usize, usize),
    Swp,
}

impl std::fmt::Display for OpCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpCode::Apply => write!(f, "Apply"),
            OpCode::Add => write!(f, "Add"),
            OpCode::Sub => write!(f, "Sub"),
            OpCode::Mul => write!(f, "Mul"),
            OpCode::Div => write!(f, "Div"),
            OpCode::Mod => write!(f, "Mod"),
            OpCode::Ge => write!(f, "Ge"),
            OpCode::Gt => write!(f, "Gt"),
            OpCode::Le => write!(f, "Le"),
            OpCode::Lt => write!(f, "Lt"),
            OpCode::And => write!(f, "And"),
            OpCode::Equ => write!(f, "Equ"),
            OpCode::Neq => write!(f, "Neq"),
            OpCode::Not => write!(f, "Not"),
            OpCode::Or => write!(f, "Or"),
            OpCode::BitAnd => write!(f, "BitAnd"),
            OpCode::BitNot => write!(f, "BitNot"),
            OpCode::BitOr => write!(f, "BitOr"),
            OpCode::BitXor => write!(f, "BitXor"),
            OpCode::Car => write!(f, "Car"),
            OpCode::Cdr => write!(f, "Cdr"),
            OpCode::Conc => write!(f, "Conc"),
            OpCode::Cons => write!(f, "Cons"),
            OpCode::Bytes => write!(f, "Bytes"),
            OpCode::Chr => write!(f, "Chr"),
            OpCode::Unpack => write!(f, "Split"),
            OpCode::Str => write!(f, "Str"),
            OpCode::Sym => write!(f, "Sym"),
            OpCode::IsByt => write!(f, "IsByt"),
            OpCode::IsChr => write!(f, "IsChr"),
            OpCode::IsLst => write!(f, "IsLst"),
            OpCode::IsNil => write!(f, "IsNil"),
            OpCode::IsNum => write!(f, "IsNum"),
            OpCode::IsStr => write!(f, "IsStr"),
            OpCode::IsSym => write!(f, "IsSym"),
            OpCode::IsTru => write!(f, "IsTru"),
            OpCode::IsWld => write!(f, "IsWld"),
            OpCode::Br(v) => write!(f, "Br({v})"),
            OpCode::Brn(v) => write!(f, "Brn({v})"),
            OpCode::Call(v) => write!(f, "Call({v})"),
            OpCode::Ret => write!(f, "Ret"),
            OpCode::Dup(n) => write!(f, "Dup({n})"),
            OpCode::Get(n) => write!(f, "Get({n})"),
            OpCode::Lst(n) => write!(f, "Lst({n})"),
            OpCode::Pak(m, n) => write!(f, "Pak({m},{n})"),
            OpCode::Pop(n) => write!(f, "Pop({n})"),
            OpCode::Psh(v) => write!(f, "Psh({v})"),
            OpCode::Rot(n) => write!(f, "Rot({n})"),
            OpCode::Rtm(m, n) => write!(f, "Rtm({m},{n})"),
            OpCode::Swp => write!(f, "Swp"),
        }
    }
}

pub type OpCodes = Vec<OpCode>;
