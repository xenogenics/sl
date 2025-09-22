use std::{io::Read, rc::Rc, str::FromStr};

use clap::{Parser, arg};
use lalrpop_util::ParseError;
use rustyline::{
    Completer, Editor, Helper, Highlighter, Hinter,
    error::ReadlineError,
    validate::{ValidationContext, ValidationResult, Validator},
};
use sl::{
    atom::Atom,
    compiler::{Artifacts, Compiler, CompilerTrait},
    grammar::{ExpressionParser, ListsParser},
    stack,
    vm::{RunParameters, VirtualMachine},
};
use strum_macros::EnumString;
use thiserror::Error;

#[derive(Parser)]
struct Arguments {
    #[arg(long)]
    history_file: Option<String>,
    #[arg(short, long, value_delimiter = ',')]
    preload: Vec<String>,
    #[arg(short, long, default_value_t = 128)]
    stack_size: usize,
}

//
// Error.
//

#[derive(Debug, Error)]
enum Error {
    #[error(transparent)]
    Mnml(#[from] sl::error::Error),
    #[error(transparent)]
    Encode(#[from] bincode::error::EncodeError),
    #[error("Invalid command: {0}")]
    InvalidCommand(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Missing command parameter: {0}")]
    MissingcommandParameter(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Readline error: {0}")]
    Readline(#[from] ReadlineError),
}

//
// Command.
//

enum Command {
    Depth(usize),
    Expand(String),
    Inspect(String),
    Quit,
    Trace(Status),
}

#[derive(Default, EnumString)]
#[strum(serialize_all = "lowercase")]
enum Status {
    On,
    #[default]
    Off,
}

impl FromStr for Command {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<_> = s.split(" ").collect();
        //
        // Check that there is a value.
        //
        if parts.is_empty() {
            return Err(Error::InvalidCommand(s.to_string()));
        }
        //
        // Check the command.
        //
        match parts[0] {
            //
            // Depth.
            //
            ".depth" if parts.len() >= 2 => {
                let status = usize::from_str(&parts[1]).unwrap_or_default();
                Ok(Self::Depth(status))
            }
            ".depth" => Err(Error::MissingcommandParameter("symbol".into())),
            //
            // Expand.
            //
            ".expand" if parts.len() >= 2 => Ok(Self::Expand(parts[1..].join(" "))),
            ".expand" => Err(Error::MissingcommandParameter("symbol".into())),
            //
            // Inspect.
            //
            ".inspect" if parts.len() >= 2 => Ok(Self::Inspect(parts[1].to_string())),
            ".inpect" => Err(Error::MissingcommandParameter("symbol".into())),
            //
            // Quit.
            //
            ".quit" => Ok(Self::Quit),
            //
            // Trace.
            //
            ".trace" if parts.len() >= 2 => {
                let status = Status::from_str(&parts[1]).unwrap_or_default();
                Ok(Self::Trace(status))
            }
            ".trace" => Err(Error::MissingcommandParameter("symbol".into())),
            //
            // Invalid command.
            //
            _ => Err(Error::InvalidCommand(s.to_string())),
        }
    }
}

//
// Null compiler.
//

#[derive(Clone, Copy)]
struct NullCompiler;

impl CompilerTrait for NullCompiler {
    fn eval(self, _: Rc<Atom>, _: RunParameters) -> Result<Rc<Atom>, sl::error::Error> {
        Ok(Atom::nil())
    }

    fn expand(self, _: Rc<Atom>, _: RunParameters) -> Result<Rc<Atom>, sl::error::Error> {
        Ok(Atom::nil())
    }

    fn load(&mut self, _: Rc<Atom>) -> Result<(), sl::error::Error> {
        Ok(())
    }

    fn compile(self, _: &str) -> Result<Artifacts, sl::error::Error> {
        Ok(Artifacts::default())
    }
}

//
// Input validator.
//

#[derive(Completer, Helper, Highlighter, Hinter)]
struct InputValidator;

impl Validator for InputValidator {
    fn validate(&self, ctx: &mut ValidationContext) -> rustyline::Result<ValidationResult> {
        use ValidationResult::{Incomplete, Invalid, Valid};
        //
        // Grab the input.
        //
        let input = ctx.input().trim();
        //
        // Check if it's a command.
        //
        if input.as_bytes()[0] == b'.' {
            match Command::from_str(input) {
                Ok(_) => return Ok(Valid(None)),
                Err(_) => return Ok(Invalid(Some(" --< invalid command".to_string()))),
            }
        }
        //
        // Declare the parser and compiler.
        //
        let parser = ExpressionParser::new();
        let mut compiler = NullCompiler;
        //
        // Try to parse it.
        //
        match parser.parse(&mut compiler, input) {
            Ok(_) => Ok(Valid(None)),
            Err(ParseError::InvalidToken { location }) => {
                let error = format!(" --< invalid token at {location}");
                Ok(Invalid(Some(error)))
            }
            Err(ParseError::UnrecognizedEof { .. }) => Ok(Incomplete),
            Err(ParseError::UnrecognizedToken {
                token: (start, token, end),
                expected: _,
            }) => Ok(Invalid(Some(format!(
                " --< unexpected token '{token}' at [{start}..{end}]"
            )))),
            Err(e) => Ok(Invalid(Some(e.to_string()))),
        }
    }
}

//
// Macro expansion.
//

fn expand(mut compiler: Compiler, stmt: &str) -> Result<(), sl::error::Error> {
    //
    // Declare the parser.
    //
    let parser = ExpressionParser::new();
    //
    // Parse the statement.
    //
    let atom = match parser.parse(&mut compiler, stmt) {
        Ok(atom) => atom,
        Err(err) => {
            println!("! {err}");
            return Ok(());
        }
    };
    //
    // Expand the macro.
    //
    match compiler.expand(atom, RunParameters::default()) {
        Ok(value) => println!("{value}"),
        Err(err) => println!("! {err}"),
    }
    //
    // Done.
    //
    Ok(())
}

//
// Bytecode inspector.
//

fn inspect(compiler: Compiler, name: &str) -> Result<(), sl::error::Error> {
    //
    // Generate the bytecode.
    //
    let artifacts = compiler.compile(name)?;
    //
    // Get the entrypoint.
    //
    let symbols = artifacts.symbols();
    let opcodes = artifacts.opcodes();
    let index = symbols.iter().position(|(v, _, _)| v.as_ref() == name);
    //
    // Make sure the index exists.
    //
    let Some(index) = index else {
        println!("! symbol not found: {name}");
        return Ok(());
    };
    //
    // Get the opcode boundaries.
    //
    let begin = symbols[index].1;
    let end = if index == symbols.len() - 1 {
        opcodes.len()
    } else {
        symbols[index + 1].1
    };
    //
    // Dump the binary.
    //
    opcodes[begin..end].iter().enumerate().for_each(|(i, op)| {
        println!("{:04} {op:?}", begin + i);
    });
    //
    // Done.
    //
    Ok(())
}

//
// Evaluator.
//

fn eval(
    mut compiler: Compiler,
    expr: Rc<Atom>,
    params: RunParameters,
) -> Result<stack::Value, sl::error::Error> {
    //
    // Wrap the statement into a top-level function.
    //
    let entrypoint = Atom::cons(
        Atom::symbol("def"),
        Atom::cons(
            Atom::symbol("__repl__"),
            Atom::cons(
                Atom::nil(),
                Atom::cons(Atom::nil(), Atom::cons(expr, Atom::nil())),
            ),
        ),
    );
    //
    // Load the top-level statement.
    //
    compiler.load(entrypoint)?;
    //
    // Generate the bytecode.
    //
    let artifacts = compiler.compile("__repl__")?;
    //
    // Build the virtual machine.
    //
    let mut vm = VirtualMachine::new("__repl__", params);
    //
    // Run.
    //
    vm.run(artifacts)
}

//
// Main.
//

fn main() -> Result<(), Error> {
    //
    // Parse the arguments.
    //
    let args = Arguments::parse();
    let mut trace = false;
    let mut depth = 10;
    //
    // Create the line editor.
    //
    let mut rl = Editor::new()?;
    rl.set_helper(Some(InputValidator));
    //
    // Load the history file.
    //
    if let Some(file) = args.history_file.as_deref() {
        rl.load_history(file).unwrap_or_default();
    }
    //
    // Create the parser.
    //
    let parser = ExpressionParser::new();
    //
    // Create the compiler.
    //
    let mut compiler = Compiler::default();
    compiler.lift_operators()?;
    //
    // Process the files to preload.
    //
    for preload in args.preload.as_slice() {
        //
        // Read the file.
        //
        let mut source = String::new();
        let mut file = std::fs::File::open(preload)?;
        file.read_to_string(&mut source)?;
        //
        // Parse the file.
        //
        let parser = ListsParser::new();
        let _ = parser
            .parse(&mut compiler, &source)
            .map_err(|v| Error::Parse(v.to_string()))?;
    }
    //
    // REPL.
    //
    loop {
        //
        // Read the line.
        //
        let readline = rl.readline("> ");
        //
        // Process the line.
        //
        match readline {
            Ok(line) => {
                //
                // Update the history.
                //
                rl.add_history_entry(line.as_str())?;
                //
                // Check if it's an internal command.
                //
                if let Ok(cmd) = Command::from_str(line.trim()) {
                    match cmd {
                        Command::Depth(value) => {
                            depth = value;
                            continue;
                        }
                        Command::Expand(statement) => {
                            expand(compiler.clone(), &statement)?;
                            continue;
                        }
                        Command::Inspect(symbol) => {
                            inspect(compiler.clone(), &symbol)?;
                            continue;
                        }
                        Command::Quit => break,
                        Command::Trace(Status::On) => {
                            trace = true;
                            continue;
                        }
                        Command::Trace(Status::Off) => {
                            trace = false;
                            continue;
                        }
                    }
                }
                //
                // Parse the line.
                //
                // NOTE(xrg): input has been validated.
                //
                let atom = parser.parse(&mut compiler, &line).unwrap();
                //
                // Try to load the statement as a top-level statement.
                //
                match compiler.load(atom.clone()) {
                    Ok(()) => continue,
                    Err(err) => match err {
                        sl::error::Error::ExpectedTopLevelStatement(_)
                        | sl::error::Error::ExpectedFunctionCall(_)
                        | sl::error::Error::ExpectedSymbol(_) => (),
                        _ => {
                            println!("! {err}");
                            continue;
                        }
                    },
                }
                //
                // Build the run parameters.
                //
                let params = RunParameters::new(args.stack_size, trace, depth);
                //
                // Try to evaluate the statement.
                //
                match eval(compiler.clone(), atom, params) {
                    Ok(value) => println!("{value}"),
                    Err(error) => println!("! {error}"),
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
    //
    // Save the history file.
    //
    if let Some(file) = args.history_file.as_deref() {
        rl.save_history(file).unwrap_or_default();
    }
    //
    // Done.
    //
    Ok(())
}
