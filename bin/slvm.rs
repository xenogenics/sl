use clap::{Parser, arg};
use sl::{
    atom::Atom,
    compiler::Artifacts,
    vm::{RunParameters, VirtualMachine},
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
    #[arg(long, default_value_t = 10)]
    depth: usize,
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, hide = true)]
    others: Vec<String>,
}

impl Into<RunParameters> for Arguments {
    fn into(self) -> RunParameters {
        RunParameters::new(self.stack_size, self.trace, self.depth)
    }
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
    // Convert the unprocessed arguments to an atom.
    //
    let argv = args
        .others
        .iter()
        .rev()
        .fold(Atom::nil(), |acc, e| Atom::cons(Atom::string(e), acc));
    //
    // Prepend the file name and push them to the VM.
    //
    let argv = Atom::cons(Atom::string(&args.file), argv);
    //
    // Build the virtual machine.
    //
    let params: RunParameters = args.into();
    let mut vm = VirtualMachine::new("main", params);
    //
    // Push the CLI arguments.
    //
    vm.push(argv.try_into()?);
    //
    // Run the binary.
    //
    let result = vm.run(artifacts)?;
    //
    // Print the stack.
    //
    println!("{:?}", result);
    //
    // Done.
    //
    Ok(())
}
