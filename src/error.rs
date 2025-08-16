use std::env::VarError;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Environment(#[from] VarError),
    #[error("Expected function call")]
    ExpectedFunctionCall,
    #[error("Expected function definition")]
    ExpectedFunctionDefinition,
    #[error("Expected lambda definition")]
    ExpectedLambdaDefinition,
    #[error("Expected module load")]
    ExpectedModuleLoad,
    #[error("Expected pair")]
    ExpectedPair,
    #[error("Expected pair or immediate")]
    ExpectedPairOrImmediate,
    #[error("Expected pair or symbol")]
    ExpectedPairOrSymbol,
    #[error("Expected quote")]
    ExpectedQuote,
    #[error("Expected statement")]
    ExpectedStatement,
    #[error("Expected symbol")]
    ExpectedSymbol,
    #[error("Expected top-level statement (def or load)")]
    ExpectedTopLevelStatement,
    #[error("Expected value")]
    ExpectedValue,
    #[error("Function already defined: {0}")]
    FunctionAlreadyDefined(Box<str>),
    #[error("Function definition can only happen at the top level")]
    FunctionDefinitionTopLevelOnly,
    #[error("Invalid label: {0}")]
    InvalidLabel(Box<str>),
    #[error("Invalid symbol: {0}")]
    InvalidSymbol(Box<str>),
    #[error("Invalid system call: {0}")]
    InvalidSystemCall(Box<str>),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Macro expansion failed: {0}")]
    MacroExpansion(Box<str>),
    #[error("Main endpoint not defined")]
    MainNotDefined,
    #[cfg(test)]
    #[error("No such context")]
    NoSuchContext,
    #[cfg(test)]
    #[error("Operation not supported")]
    NotSupported,
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Unquote outside backquote context")]
    UnquoteOutsideBackquote,
    #[error("Unresolved symbol: {0}")]
    UnresolvedSymbol(Box<str>),
}
