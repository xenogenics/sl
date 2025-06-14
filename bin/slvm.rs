use clap::{arg, Parser};
use sl::{opcodes::OpCode, vm::VirtualMachine};
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
    let (syms, ops): (Vec<(Box<str>, usize, usize)>, Vec<OpCode>) =
        bincode::decode_from_std_read(&mut file, conf)?;
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
