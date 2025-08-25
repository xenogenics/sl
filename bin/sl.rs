use std::{io::Read, rc::Rc};

use clap::{Parser, arg};
use sl::{
    atom::Atom,
    compiler::{Compiler, CompilerTrait},
    grammar::ListsParser,
    vm::VirtualMachine,
};
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
    // Create the compiler.
    //
    let mut compiler = Compiler::default();
    compiler.lift_operators()?;
    //
    // Parse the source file.
    //
    let parser = ListsParser::new();
    let _ = parser
        .parse(&mut compiler, &source)
        .map_err(|v| Error::Parse(v.to_string()))?;
    //
    // Generate the bytecode.
    //
    let artifacts = compiler.compile()?;
    //
    // Build the virtual machine.
    //
    let mut vm = VirtualMachine::new("main", args.stack_size, args.trace);
    //
    // Run the binary.
    //
    let result = vm.run(artifacts)?;
    //
    // Print the stack.
    //
    let atom: Rc<Atom> = result.try_into()?;
    println!("{atom}");
    //
    // Done.
    //
    Ok(())
}
