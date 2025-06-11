use bincode::{Decode, Encode};

//
// Immediate values.
//

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode)]
pub enum Immediate {
    Nil,
    True,
    Char(u8),
    Number(i64),
    Funcall(usize),
}

impl Immediate {
    pub const fn funcall(&self) -> usize {
        match self {
            Immediate::Funcall(v) => *v,
            _ => panic!("Expected a funcall"),
        }
    }

    pub const fn number(&self) -> i64 {
        match self {
            Immediate::Number(v) => *v,
            _ => panic!("Expected a number"),
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
    // List operations.
    //
    Car,
    Cdr,
    Cons,
    //
    // Control flow.
    //
    Br(isize),
    Brn(isize),
    Call,
    Hlt,
    Ret,
    //
    // Stack operations.
    //
    Dup(usize),
    Get(usize),
    Pak(usize),
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

pub type OpCodes = Vec<OpCode>;
