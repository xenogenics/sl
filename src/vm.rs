use std::rc::Rc;

use crate::{
    error::Error,
    heap,
    opcodes::{Immediate, OpCode},
    stack::{Stack, Value},
};

pub struct VirtualMachine {
    stack: Stack,
}

impl VirtualMachine {
    pub fn new(capacity: usize) -> Self {
        Self {
            stack: Stack::new(capacity),
        }
    }

    pub fn run(&mut self, syms: Vec<(Box<str>, usize)>, ops: Vec<OpCode>) -> Result<(), Error> {
        //
        // Look-up the main function.
        //
        let main_fn = syms
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
        // Push the initial return value.
        //
        self.stack.push(Value::Link(ops.len()).into());
        //
        // Interpreter loop.
        //
        loop {
            //
            // Check if we are done.
            //
            if pc >= ops.len() {
                break;
            }
            //
            // Execute the opcode.
            //
            match ops[pc] {
                //
                // Arithmetics.
                //
                OpCode::Add => {
                    let b = self.stack.pop().immediate().number();
                    let a = self.stack.pop().immediate().number();
                    let v = Value::from(Immediate::Number(a + b));
                    self.stack.push(v.into());
                }
                OpCode::Ge => {
                    let b = self.stack.pop().immediate().number();
                    let a = self.stack.pop().immediate().number();
                    self.stack.push(Value::from(Immediate::from(a >= b)).into());
                }
                OpCode::Gt => {
                    let b = self.stack.pop().immediate().number();
                    let a = self.stack.pop().immediate().number();
                    self.stack.push(Value::from(Immediate::from(a > b)).into());
                }
                OpCode::Le => {
                    let b = self.stack.pop().immediate().number();
                    let a = self.stack.pop().immediate().number();
                    self.stack.push(Value::from(Immediate::from(a <= b)).into());
                }
                OpCode::Lt => {
                    let b = self.stack.pop().immediate().number();
                    let a = self.stack.pop().immediate().number();
                    self.stack.push(Value::from(Immediate::from(a < b)).into());
                }
                OpCode::Sub => {
                    let b = self.stack.pop().immediate().number();
                    let a = self.stack.pop().immediate().number();
                    let v = Value::from(Immediate::Number(a - b));
                    self.stack.push(v.into());
                }
                //
                // List operations.
                //
                OpCode::Car => {
                    let result = match self.stack.pop() {
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
                    self.stack.push(result);
                }
                OpCode::Cdr => {
                    let result = match self.stack.pop() {
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
                    self.stack.push(result);
                }
                OpCode::Cons => {
                    let (a, b) = match (self.stack.pop(), self.stack.pop()) {
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
                    self.stack
                        .push(Value::Heap(Rc::new(heap::Value::Pair(a, b))));
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
                    let address = match self.stack.pop() {
                        Value::Closure(v) => {
                            self.stack.unpack(v);
                            self.stack.pop().immediate().funcall()
                        }
                        Value::Immediate(Immediate::Funcall(v)) => v,
                        _ => panic!("Expected a function address or closure"),
                    };
                    //
                    // Push the return link and jump.
                    //
                    self.stack.push(Value::Link(pc + 1).into());
                    pc = address;
                    continue;
                }

                OpCode::Brn(v) => {
                    if matches!(self.stack.pop(), Value::Immediate(Immediate::Nil)) {
                        pc += v;
                        continue;
                    }
                }
                OpCode::Hlt => todo!(),
                OpCode::Ret => {
                    pc = self.stack.unlink().link();
                    continue;
                }
                //
                // self.stack operations.
                //
                OpCode::Dup(v) => self.stack.dup(v),
                OpCode::Get(v) => self.stack.get(v),
                OpCode::Pak(v) => self.stack.pack(v),
                OpCode::Pop(v) => self.stack.drop(v),
                OpCode::Psh(v) => self.stack.push(Value::from(v).into()),
                OpCode::Rot(n) => self.stack.rotate(n),
                OpCode::Swp => self.stack.swap(),
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
        println!("{:?}", self.stack);
        //
        // Done.
        //
        Ok(())
    }
}
