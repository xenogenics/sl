use std::{rc::Rc, str::FromStr};

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
    grammar::ExpressionParser,
    heap,
    opcodes::Immediate,
    stack,
    vm::VirtualMachine,
};
use thiserror::Error;

#[derive(Parser)]
struct Arguments {
    #[arg(short, long, default_value_t = 128)]
    stack_size: usize,
    #[arg(long)]
    trace: bool,
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
    #[error("Readline error: {0}")]
    Readline(#[from] ReadlineError),
}

//
// Command.
//

enum Command {
    Expand(String),
    Inspect(String),
    Quit,
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
            ".expand" if parts.len() >= 2 => Ok(Self::Expand(parts[1..].join(" "))),
            ".inspect" if parts.len() >= 2 => Ok(Self::Inspect(parts[1].to_string())),
            ".inpect" => Err(Error::MissingcommandParameter("symbol".into())),
            ".quit" => Ok(Self::Quit),
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
    fn eval(self, _: Rc<Atom>, _: usize, _: bool) -> Result<Rc<Atom>, sl::error::Error> {
        Ok(Atom::nil())
    }

    fn expand(self, _: Rc<Atom>, _: usize, _: bool) -> Result<Rc<Atom>, sl::error::Error> {
        Ok(Atom::nil())
    }

    fn load(&mut self, _: Rc<Atom>) -> Result<(), sl::error::Error> {
        Ok(())
    }

    fn compile(self) -> Result<Artifacts, sl::error::Error> {
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
    match compiler.expand(atom, 1024, false) {
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
    let artifacts = compiler.compile()?;
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
    stack_size: usize,
    trace: bool,
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
    let artifacts = compiler.compile()?;
    //
    // Build the virtual machine.
    //
    let mut vm = VirtualMachine::new("__repl__", stack_size, trace);
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
    //
    // Create the line editor.
    //
    let mut rl = Editor::new()?;
    rl.set_helper(Some(InputValidator));
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
                        Command::Expand(statement) => {
                            expand(compiler.clone(), &statement)?;
                            continue;
                        }
                        Command::Inspect(symbol) => {
                            inspect(compiler.clone(), &symbol)?;
                            continue;
                        }
                        Command::Quit => break,
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
                        | sl::error::Error::ExpectedFunctionCall(_) => (),
                        _ => {
                            println!("! {err}");
                            continue;
                        }
                    },
                }
                //
                // Try to evaluate the statement.
                //
                match eval(compiler.clone(), atom, args.stack_size, args.trace) {
                    Ok(value) => match value {
                        stack::Value::Closure(_) => println!("#<closure>"),
                        stack::Value::Heap(value) => match value.as_ref() {
                            heap::Value::Closure(_) => println!("#<closure>"),
                            heap::Value::Immediate(value) => match value {
                                Immediate::Extcall(idx) => println!("#<extcall[{idx}]>"),
                                Immediate::Funcall(idx, _) => println!("#<funcall[{idx}]>"),
                                _ => {
                                    let value: Rc<Atom> = value.clone().try_into().unwrap();
                                    println!("{value}");
                                }
                            },
                            heap::Value::Bytes(..)
                            | heap::Value::Pair(..)
                            | heap::Value::String(..) => {
                                let value: Rc<Atom> = value.as_ref().clone().try_into().unwrap();
                                println!("{value}");
                            }
                        },
                        stack::Value::Immediate(value) => match value {
                            Immediate::Extcall(idx) => println!("#<extcall[{idx}]>"),
                            Immediate::Funcall(idx, _) => println!("#<funcall[{idx}]>"),
                            _ => {
                                let value: Rc<Atom> = value.try_into().unwrap();
                                println!("{value}");
                            }
                        },
                        stack::Value::Link(_) => todo!(),
                    },
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
    // Done.
    //
    Ok(())
}
