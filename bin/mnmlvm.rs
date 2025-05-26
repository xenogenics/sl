use std::rc::Rc;

use clap::{arg, Parser};
use mnml::{
    heap,
    opcodes::{Immediate, OpCode},
    stack::{Stack, Value},
};
use thiserror::Error;

#[derive(Parser)]
struct Arguments {
    #[arg(short, long)]
    file: String,
    #[arg(short, long, default_value_t = 128)]
    stack_size: usize,
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
    let state: (Vec<(Box<str>, usize)>, Vec<OpCode>) =
        bincode::decode_from_std_read(&mut file, conf)?;
    //
    // Look-up the main function.
    //
    let main_fn = state
        .0
        .iter()
        .find_map(|(k, v)| (k.as_ref() == "main").then_some(v))
        .copied();
    //
    // Make sure it exists.
    //
    let Some(mut pc) = main_fn else {
        return Err(Error::MainNotDefined);
    };
    //
    // Declare the stack.
    //
    let mut stack = Stack::new(args.stack_size);
    //
    // Push the initial return value.
    //
    stack.push(Value::Link(state.1.len()).into());
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
            //
            // Arithmetics.
            //
            OpCode::Add => {
                let b = stack.pop().immediate().number();
                let a = stack.pop().immediate().number();
                stack.push(Value::from(Immediate::Number(a + b)).into());
            }
            OpCode::Ge => {
                let b = stack.pop().immediate().number();
                let a = stack.pop().immediate().number();
                stack.push(Value::from(Immediate::from(a >= b)).into());
            }
            OpCode::Gt => {
                let b = stack.pop().immediate().number();
                let a = stack.pop().immediate().number();
                stack.push(Value::from(Immediate::from(a > b)).into());
            }
            OpCode::Le => {
                let b = stack.pop().immediate().number();
                let a = stack.pop().immediate().number();
                stack.push(Value::from(Immediate::from(a <= b)).into());
            }
            OpCode::Lt => {
                let b = stack.pop().immediate().number();
                let a = stack.pop().immediate().number();
                stack.push(Value::from(Immediate::from(a < b)).into());
            }
            OpCode::Sub => {
                let b = stack.pop().immediate().number();
                let a = stack.pop().immediate().number();
                stack.push(Value::from(Immediate::Number(a - b)).into());
            }
            //
            // List operations.
            //
            OpCode::Car => {
                let result = match stack.pop() {
                    Value::Heap(value) => match value.as_ref() {
                        heap::Value::Pair(value, _) => match value.as_ref() {
                            heap::Value::Closure(v) => Value::Closure(v.clone()),
                            heap::Value::Immediate(v) => Value::Immediate(*v),
                            _ => Value::Heap(value.clone()),
                        },
                        _ => Immediate::Nil.into(),
                    },
                    _ => Immediate::Nil.into(),
                };
                stack.push(result);
            }
            OpCode::Cdr => {
                let result = match stack.pop() {
                    Value::Heap(value) => match value.as_ref() {
                        heap::Value::Pair(_, value) => match value.as_ref() {
                            heap::Value::Closure(v) => Value::Closure(v.clone()),
                            heap::Value::Immediate(v) => Value::Immediate(*v),
                            _ => Value::Heap(value.clone()),
                        },
                        _ => Immediate::Nil.into(),
                    },
                    _ => Immediate::Nil.into(),
                };
                stack.push(result);
            }
            OpCode::Cons => {
                let (a, b) = match (stack.pop(), stack.pop()) {
                    (Value::Closure(b), Value::Closure(a)) => {
                        let a = Rc::new(heap::Value::Closure(a));
                        let b = Rc::new(heap::Value::Closure(b));
                        (a, b)
                    }
                    (Value::Closure(b), Value::Heap(a)) => {
                        let b = Rc::new(heap::Value::Closure(b));
                        (a.clone(), b)
                    }
                    (Value::Closure(b), Value::Immediate(a)) => {
                        let a = Rc::new(heap::Value::Immediate(a));
                        let b = Rc::new(heap::Value::Closure(b));
                        (a, b)
                    }
                    (Value::Heap(b), Value::Closure(a)) => {
                        let a = Rc::new(heap::Value::Closure(a));
                        (a, b.clone())
                    }
                    (Value::Heap(b), Value::Heap(a)) => (a.clone(), b.clone()),
                    (Value::Heap(b), Value::Immediate(a)) => {
                        let a = Rc::new(heap::Value::Immediate(a));
                        (a, b.clone())
                    }
                    (Value::Immediate(b), Value::Closure(a)) => {
                        let a = Rc::new(heap::Value::Closure(a));
                        let b = Rc::new(heap::Value::Immediate(b));
                        (a, b)
                    }
                    (Value::Immediate(b), Value::Heap(a)) => {
                        let b = Rc::new(heap::Value::Immediate(b));
                        (a.clone(), b)
                    }
                    (Value::Immediate(b), Value::Immediate(a)) => {
                        let a = Rc::new(heap::Value::Immediate(a));
                        let b = Rc::new(heap::Value::Immediate(b));
                        (a, b)
                    }
                    _ => panic!("Return link cannot be pushed to the heap"),
                };
                stack.push(Value::Heap(Rc::new(heap::Value::Pair(a, b))));
            }
            //
            // Control flow.
            //
            OpCode::Br(v) => {
                pc = pc + v;
                continue;
            }
            OpCode::Call => {
                //
                // Decode the call address.
                //
                let address = match stack.pop() {
                    Value::Closure(v) => {
                        stack.unpack(v);
                        stack.pop().immediate().funcall()
                    }
                    Value::Immediate(Immediate::Funcall(v)) => v,
                    _ => panic!("Expected a function address or closure"),
                };
                //
                // Push the return link and jump.
                //
                stack.push(Value::Link(pc + 1).into());
                pc = address;
                continue;
            }

            OpCode::Brn(v) => {
                if matches!(stack.pop(), Value::Immediate(Immediate::Nil)) {
                    pc += v;
                    continue;
                }
            }
            OpCode::Hlt => todo!(),
            OpCode::Ret => {
                pc = stack.unlink().link();
                continue;
            }
            //
            // Stack operations.
            //
            OpCode::Dup(v) => stack.dup(v),
            OpCode::Get(v) => stack.get(v),
            OpCode::Pak(v) => stack.pack(v),
            OpCode::Pop(v) => stack.drop(v),
            OpCode::Psh(v) => stack.push(Value::from(v).into()),
            OpCode::Rot(n) => stack.rotate(n),
            OpCode::Swp => stack.swap(),
            //
            // Memory operations.
            //
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
