use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Expected function call")]
    ExpectedFunctionCall,
    #[error("Expected pair")]
    ExpectedPair,
    #[error("Expected symbol")]
    ExpectedSymbol,
    #[error("Function already defined: {0}")]
    FunctionAlreadyDefined(Box<str>),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Invalid symbol: {0}")]
    InvalidSymbol(Box<str>),
}
