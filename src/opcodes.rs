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
    Symbol([u8; 15]),
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
    Sub,
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
    // List operations.
    //
    Car,
    Cdr,
    Cons,
    //
    // Predicates.
    //
    IsLst,
    IsNil,
    //
    // Control flow.
    //
    Br(isize),
    Brn(isize),
    Call,
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
}

pub type OpCodes = Vec<OpCode>;
