//
// Immediate values.
//

use bincode::{Decode, Encode};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode)]
pub enum Immediate {
    Nil,
    True,
    Char(u8),
    Link(usize),
    Number(i64),
}

impl Immediate {
    pub fn is_nil(&self) -> bool {
        match self {
            Immediate::Nil => true,
            _ => false,
        }
    }
}

impl Into<i64> for Immediate {
    fn into(self) -> i64 {
        match self {
            Self::Nil => 0,
            Self::True => 1,
            Self::Char(v) => v as i64,
            Self::Link(v) => v as i64,
            Self::Number(v) => v,
        }
    }
}

impl Into<usize> for Immediate {
    fn into(self) -> usize {
        match self {
            Self::Nil => 0,
            Self::True => 1,
            Self::Char(v) => v as usize,
            Self::Link(v) => v,
            Self::Number(v) => v as usize,
        }
    }
}

impl From<bool> for Immediate {
    fn from(value: bool) -> Self {
        if value {
            Self::True
        } else {
            Self::Nil
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
    // Arithmetics.
    //
    Add,
    Ge,
    Gt,
    Le,
    Lt,
    Sub,
    //
    // Control flow.
    //
    Br(usize),
    Brl(usize),
    Brn(usize),
    Hlt,
    Ret,
    //
    // Stack operations.
    //
    Dup(usize),
    Pck(usize),
    Pop(usize),
    Psh(Immediate),
    Rot(usize),
    Swp,
    //
    // Memory operations.
    //
    Ld,
    St,
}
