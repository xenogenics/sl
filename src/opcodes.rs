use bincode::{Decode, Encode};

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

//
// Opcodes.
//

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode)]
#[repr(u8)]
pub enum OpCode {
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
    Split,
    Str,
    Sym,
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

pub type OpCodes = Vec<OpCode>;
