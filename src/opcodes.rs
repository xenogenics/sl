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

//
// Immediate values.
//

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode)]
pub enum Immediate {
    Nil,
    True,
    Char(u8),
    Number(i64),
    Funcall(u32, Arity),
    Syscall(u32, u32),
    Symbol([u8; 15]),
    Wildcard,
}

impl Immediate {
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

impl From<bool> for Immediate {
    fn from(value: bool) -> Self {
        if value { Self::True } else { Self::Nil }
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
    // String operation.
    //
    Str,
    //
    // Predicates.
    //
    IsChr,
    IsLst,
    IsNil,
    IsNum,
    IsSym,
    IsTru,
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
    Pak(usize),
    Pop(usize),
    Psh(Immediate),
    Rot(usize),
    Rtm(usize, usize),
    Swp,
}

pub type OpCodes = Vec<OpCode>;
