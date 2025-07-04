#![allow(clippy::borrowed_box)]

#[macro_use]
extern crate lalrpop_util;

//
// Grammar.
//

lalrpop_mod!(
    #[allow(clippy::all)]
    pub grammar
);

//
// Modules.
//

pub mod atom;
pub mod compiler;
pub mod error;
pub mod heap;
pub mod ir;
pub mod opcodes;
pub mod stack;
pub mod syscalls;
pub mod vm;

//
// Tests.
//

#[cfg(test)]
mod tests;
