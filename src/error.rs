use std::env::VarError;

use thiserror::Error;

use crate::atom::Span;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Environment(#[from] VarError),
    #[error("Expected function call")]
    ExpectedFunctionCall(Span),
    #[error("Expected function definition")]
    ExpectedFunctionDefinition(Span),
    #[error("Expected lambda definition")]
    ExpectedLambdaDefinition(Span),
    #[error("Expected macro")]
    ExpectedMacro(Span),
    #[error("Expected module load")]
    ExpectedModuleLoad(Span),
    #[error("Expected pair")]
    ExpectedPair(Span),
    #[error("Expected pair or immediate")]
    ExpectedPairOrImmediate(Span),
    #[error("Expected pair or symbol")]
    ExpectedPairOrSymbol(Span),
    #[error("Expected quote")]
    ExpectedQuote(Span),
    #[error("Expected statement")]
    ExpectedStatement(Span),
    #[error("Expected symbol")]
    ExpectedSymbol(Span),
    #[error("Expected top-level statement (def or load)")]
    ExpectedTopLevelStatement(Span),
    #[error("Expected value")]
    ExpectedValue(Span),
    #[error("Function definition can only happen at the top level")]
    FunctionDefinitionTopLevelOnly,
    #[error("Invalid foreign type: {0}")]
    InvalidForeignType(Box<str>),
    #[error("Invalid label: {0}")]
    InvalidLabel(Box<str>),
    #[error("Invalid string")]
    InvalidString,
    #[error("Invalid symbol: {0}")]
    InvalidSymbol(Box<str>),
    #[error("Invalid system call: {0}")]
    InvalidSystemCall(Box<str>),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Macro expansion failed: {0}")]
    MacroExpansion(Box<str>),
    #[error("Main endpoint not defined")]
    EntrypointNotDefined,
    #[cfg(test)]
    #[error("No such context")]
    NoSuchContext,
    #[cfg(test)]
    #[error("Operation not supported")]
    NotSupported,
    #[error("Open self image failed")]
    OpenSelfImage,
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Unquote outside backquote context")]
    UnquoteOutsideBackquote(Span),
    #[error("Unresolved symbol: {0}")]
    UnresolvedSymbol(Box<str>),
}
