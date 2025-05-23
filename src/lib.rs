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

pub mod compiler;
pub mod error;
pub mod opcodes;
pub mod stack;
pub mod types;

//
// Tests.
//

#[cfg(test)]
mod tests;
