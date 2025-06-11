use std::rc::Rc;

use crate::{
    error::Error,
    heap,
    opcodes::{Immediate, OpCode},
    stack::{Stack, Value},
};

pub struct VirtualMachine {
    stack: Stack,
    trace: bool,
}

impl VirtualMachine {
    pub fn new(capacity: usize, trace: bool) -> Self {
        Self {
            stack: Stack::new(capacity),
            trace,
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
            // Print trace.
            //
            if self.trace {
                println!("{:?}", self.stack);
                println!("{pc:04} {:?}", ops[pc]);
            }
            //
            // Execute the opcode.
            //
            match ops[pc] {
                //
                // Arithmetics.
                //
                OpCode::Add => {
                    let a = self.stack.pop().immediate().number();
                    let b = self.stack.pop().immediate().number();
                    let v = Value::from(Immediate::Number(a + b));
                    self.stack.push(v.into());
                }
                OpCode::Sub => {
                    let a = self.stack.pop().immediate().number();
                    let b = self.stack.pop().immediate().number();
                    let v = Value::from(Immediate::Number(a - b));
                    self.stack.push(v.into());
                }
                OpCode::Ge => {
                    let a = self.stack.pop().immediate().number();
                    let b = self.stack.pop().immediate().number();
                    self.stack.push(Value::from(Immediate::from(a >= b)).into());
                }
                OpCode::Gt => {
                    let a = self.stack.pop().immediate().number();
                    let b = self.stack.pop().immediate().number();
                    self.stack.push(Value::from(Immediate::from(a > b)).into());
                }
                OpCode::Le => {
                    let a = self.stack.pop().immediate().number();
                    let b = self.stack.pop().immediate().number();
                    self.stack.push(Value::from(Immediate::from(a <= b)).into());
                }
                OpCode::Lt => {
                    let a = self.stack.pop().immediate().number();
                    let b = self.stack.pop().immediate().number();
                    self.stack.push(Value::from(Immediate::from(a < b)).into());
                }
                //
                // Logics.
                //
                OpCode::And => {
                    let a = matches!(self.stack.pop(), Value::Immediate(Immediate::Nil));
                    let b = matches!(self.stack.pop(), Value::Immediate(Immediate::Nil));
                    self.stack.push(Value::Immediate((!a && !b).into()));
                }
                OpCode::Equ => {
                    let a = self.stack.pop();
                    let b = self.stack.pop();
                    self.stack.push(Value::Immediate((a == b).into()));
                }
                OpCode::Neq => {
                    let a = self.stack.pop();
                    let b = self.stack.pop();
                    self.stack.push(Value::Immediate((a != b).into()));
                }
                OpCode::Not => {
                    let a = matches!(self.stack.pop(), Value::Immediate(Immediate::Nil));
                    self.stack.push(Value::Immediate(a.into()));
                }
                OpCode::Or => {
                    let a = matches!(self.stack.pop(), Value::Immediate(Immediate::Nil));
                    let b = matches!(self.stack.pop(), Value::Immediate(Immediate::Nil));
                    self.stack.push(Value::Immediate((!a || !b).into()));
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
                        (Value::Closure(a), Value::Closure(b)) => {
                            let a = Rc::new(heap::Value::Closure(a));
                            let b = Rc::new(heap::Value::Closure(b));
                            (a, b)
                        }
                        (Value::Closure(a), Value::Heap(b)) => {
                            let a = Rc::new(heap::Value::Closure(a));
                            (a, b.clone())
                        }
                        (Value::Closure(a), Value::Immediate(b)) => {
                            let a = Rc::new(heap::Value::Closure(a));
                            let b = Rc::new(heap::Value::Immediate(b));
                            (a, b)
                        }
                        (Value::Heap(a), Value::Closure(b)) => {
                            let b = Rc::new(heap::Value::Closure(b));
                            (a.clone(), b)
                        }
                        (Value::Heap(a), Value::Heap(b)) => (a.clone(), b.clone()),
                        (Value::Heap(a), Value::Immediate(b)) => {
                            let b = Rc::new(heap::Value::Immediate(b));
                            (a.clone(), b)
                        }
                        (Value::Immediate(a), Value::Closure(b)) => {
                            let a = Rc::new(heap::Value::Immediate(a));
                            let b = Rc::new(heap::Value::Closure(b));
                            (a, b)
                        }
                        (Value::Immediate(a), Value::Heap(b)) => {
                            let a = Rc::new(heap::Value::Immediate(a));
                            (a, b.clone())
                        }
                        (Value::Immediate(a), Value::Immediate(b)) => {
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
                // Predicates.
                //
                OpCode::IsLst => {
                    let r = match self.stack.pop() {
                        Value::Heap(value) => match value.as_ref() {
                            heap::Value::Pair(..) => true,
                            _ => false,
                        },
                        _ => false,
                    };
                    self.stack.push(Value::Immediate(r.into()));
                }
                OpCode::IsNil => {
                    let r = matches!(self.stack.pop(), Value::Immediate(Immediate::Nil));
                    self.stack.push(Value::Immediate(r.into()));
                }
                //
                // Control flow.
                //
                OpCode::Br(v) => {
                    pc = (pc as isize + v) as usize;
                    continue;
                }
                OpCode::Brn(v) => {
                    if matches!(self.stack.pop(), Value::Immediate(Immediate::Nil)) {
                        pc = (pc as isize + v) as usize;
                        continue;
                    }
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
