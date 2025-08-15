use clap::{Parser, arg};
use sl::{compiler::SymbolsAndOpCodes, vm::VirtualMachine};
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
    Decode(#[from] bincode::error::DecodeError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

fn main() -> Result<(), Error> {
    //
    // Parse the arguments.
    //
    let args = Arguments::parse();
    //
    // Open the bytecode file.
    //
    let mut file = std::fs::File::open(&args.file)?;
    //
    // Decode the bytecode file.
    //
    let conf = bincode::config::standard();
    let (syms, ops): SymbolsAndOpCodes = bincode::decode_from_std_read(&mut file, conf)?;
    //
    // Build the virtual machine.
    //
    let mut vm = VirtualMachine::new("main", args.stack_size, args.trace);
    //
    // Run the binary.
    //
    let result = vm.run(syms, ops)?;
    //
    // Print the stack.
    //
    println!("{:?}", result);
    //
    // Done.
    //
    Ok(())
}
