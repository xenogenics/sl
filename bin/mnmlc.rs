use std::io::Read;

use clap::{arg, Parser};
use mnml::{compiler::Compiler, grammar::ListsParser};
use thiserror::Error;

#[derive(Parser)]
struct Arguments {
    #[arg(short, long)]
    file: String,
    #[arg(short, long)]
    output: String,
}

#[derive(Debug, Error)]
enum Error {
    #[error(transparent)]
    Compile(#[from] mnml::error::Error),
    #[error(transparent)]
    Encode(#[from] bincode::error::EncodeError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(String),
}

fn main() -> Result<(), Error> {
    //
    // Parse the arguments.
    //
    let args = Arguments::parse();
    //
    // Open the source file.
    //
    let mut source = String::new();
    let mut file = std::fs::File::open(&args.file)?;
    file.read_to_string(&mut source)?;
    //
    // Parse the source file.
    //
    let parser = ListsParser::new();
    let atoms = parser
        .parse(&source)
        .map_err(|v| Error::Parse(v.to_string()))?;
    //
    // Compile the atoms.
    //
    let mut compiler = Compiler::default();
    let state = compiler.compile(atoms)?;
    //
    // Write the serialize output.
    //
    let conf = bincode::config::standard();
    let mut file = std::fs::File::create(&args.output)?;
    bincode::encode_into_std_write(state, &mut file, conf)?;
    //
    // Done.
    //
    Ok(())
}
