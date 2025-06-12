use std::io::Read;

use clap::{arg, Parser};
use sl::{compiler::Compiler, grammar::ListsParser, vm::VirtualMachine};
use thiserror::Error;

#[derive(Parser)]
struct Arguments {
    #[arg(short, long)]
    file: String,
    #[arg(short, long, default_value_t = 128)]
    stack_size: usize,
    #[arg(long)]
    trace: bool,
}

#[derive(Debug, Error)]
enum Error {
    #[error(transparent)]
    Mnml(#[from] sl::error::Error),
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
    compiler.lift_operators()?;
    let (syms, ops) = compiler.compile(atoms)?;
    //
    // Build the virtual machine.
    //
    let mut vm = VirtualMachine::new(args.stack_size, args.trace);
    //
    // Run the binary.
    //
    vm.run(syms, ops)?;
    //
    // Done.
    //
    Ok(())
}
