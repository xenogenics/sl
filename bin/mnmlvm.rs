use std::collections::HashMap;

use clap::{arg, Parser};
use mnml::opcodes::{Immediate, OpCode};
use thiserror::Error;

#[derive(Parser)]
struct Arguments {
    #[arg(short, long)]
    file: String,
}

#[derive(Debug, Error)]
enum Error {
    #[error(transparent)]
    Compile(#[from] mnml::error::Error),
    #[error(transparent)]
    Decode(#[from] bincode::error::DecodeError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Main function not defined")]
    MainNotDefined,
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
    let state: (HashMap<String, usize>, Vec<OpCode>) =
        bincode::decode_from_std_read(&mut file, conf)?;
    //
    // Look-up the main function.
    //
    let Some(mut pc) = state.0.get("main").copied() else {
        return Err(Error::MainNotDefined);
    };
    //
    // Declare the stack.
    //
    let mut stack: Vec<Immediate> = Vec::default();
    //
    // Push the initial return value.
    //
    stack.push(Immediate::Link(state.1.len()));
    //
    // Interpreter loop.
    //
    loop {
        //
        // Check if we are done.
        //
        if pc >= state.1.len() {
            break;
        }
        //
        // Execute the opcode.
        //
        match state.1[pc] {
            OpCode::Add => {
                let b: i64 = stack.pop().unwrap().into();
                let a: i64 = stack.pop().unwrap().into();
                stack.push(Immediate::Number(a + b));
            }
            OpCode::Ge => {
                let b: i64 = stack.pop().unwrap().into();
                let a: i64 = stack.pop().unwrap().into();
                stack.push((a >= b).into());
            }
            OpCode::Gt => {
                let b: i64 = stack.pop().unwrap().into();
                let a: i64 = stack.pop().unwrap().into();
                stack.push((a > b).into());
            }
            OpCode::Le => {
                let b: i64 = stack.pop().unwrap().into();
                let a: i64 = stack.pop().unwrap().into();
                stack.push((a <= b).into());
            }
            OpCode::Lt => {
                let b: i64 = stack.pop().unwrap().into();
                let a: i64 = stack.pop().unwrap().into();
                stack.push((a < b).into());
            }
            OpCode::Sub => {
                let b: i64 = stack.pop().unwrap().into();
                let a: i64 = stack.pop().unwrap().into();
                stack.push(Immediate::Number(a - b));
            }
            OpCode::Br(v) => {
                pc = pc + v;
                continue;
            }
            OpCode::Brl(v) => {
                stack.push(Immediate::Link(pc + 1));
                pc = v;
                continue;
            }
            OpCode::Brn(v) => {
                if stack.pop().unwrap().is_nil() {
                    pc += v;
                    continue;
                }
            }
            OpCode::Hlt => todo!(),
            OpCode::Ret => {
                let res = stack.pop().unwrap();
                let ret = stack.pop().unwrap();
                stack.push(res);
                pc = ret.into();
                continue;
            }
            OpCode::Dup(v) => {
                for i in stack.len() - v..stack.len() {
                    stack.push(stack[i]);
                }
            }
            OpCode::Pck(v) => {
                stack.push(stack[stack.len() - v].clone());
            }
            OpCode::Pop(v) => {
                for _ in 0..v {
                    stack.pop();
                }
            }
            OpCode::Psh(immediate) => {
                stack.push(immediate);
            }
            OpCode::Rot(n) => {
                let v = stack.pop().unwrap();
                stack.insert(stack.len() - (n - 1), v);
            }
            OpCode::Swp => {
                let a = stack.pop().unwrap();
                let b = stack.pop().unwrap();
                stack.push(a);
                stack.push(b);
            }
            OpCode::Ld => todo!(),
            OpCode::St => todo!(),
        }
        //
        // Increment the program counter.
        //
        pc += 1;
    }
    //
    // Print the stack.
    //
    println!("{stack:?}");
    //
    // Done.
    //
    Ok(())
}
