use clap::{Parser, arg};
use sl::compiler::Artifacts;
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
    let artifacts: Artifacts = bincode::decode_from_std_read(&mut file, conf)?;
    //
    // Split the artifacts.
    //
    let syms = artifacts.symbols();
    let ops = artifacts.opcodes();
    //
    // Dump the binary.
    //
    ops.iter().enumerate().for_each(|(i, op)| {
        if let Some((e, _, _)) = syms.iter().find(|(_, n, _)| *n == i) {
            println!("{e}:");
        }
        println!("    {i:04} {op:?}");
    });
    //
    // Done.
    //
    Ok(())
}
