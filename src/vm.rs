use std::rc::Rc;

use crate::{
    error::Error,
    heap,
    opcodes::{Arity, Immediate, OpCode},
    stack::{Stack, Value},
    syscalls,
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

    pub fn run(
        &mut self,
        syms: Vec<(Box<str>, usize, Arity)>,
        ops: Vec<OpCode>,
    ) -> Result<(), Error> {
        //
        // Look-up the main function.
        //
        let main_fn = syms
            .iter()
            .find_map(|(k, v, _)| (k.as_ref() == "main").then_some(v))
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
                println!("---- {:?}", self.stack);
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
                    let a = self.stack.pop().as_immediate().as_number();
                    let b = self.stack.pop().as_immediate().as_number();
                    let v = Value::from(Immediate::Number(a + b));
                    self.stack.push(v.into());
                }
                OpCode::Sub => {
                    let a = self.stack.pop().as_immediate().as_number();
                    let b = self.stack.pop().as_immediate().as_number();
                    let v = Value::from(Immediate::Number(a - b));
                    self.stack.push(v.into());
                }
                OpCode::Ge => {
                    let a = self.stack.pop().as_immediate().as_number();
                    let b = self.stack.pop().as_immediate().as_number();
                    self.stack.push(Value::from(Immediate::from(a >= b)).into());
                }
                OpCode::Gt => {
                    let a = self.stack.pop().as_immediate().as_number();
                    let b = self.stack.pop().as_immediate().as_number();
                    self.stack.push(Value::from(Immediate::from(a > b)).into());
                }
                OpCode::Le => {
                    let a = self.stack.pop().as_immediate().as_number();
                    let b = self.stack.pop().as_immediate().as_number();
                    self.stack.push(Value::from(Immediate::from(a <= b)).into());
                }
                OpCode::Lt => {
                    let a = self.stack.pop().as_immediate().as_number();
                    let b = self.stack.pop().as_immediate().as_number();
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
                OpCode::IsChr => {
                    let r = matches!(self.stack.pop(), Value::Immediate(Immediate::Char(_)));
                    self.stack.push(Value::Immediate(r.into()));
                }
                OpCode::IsNum => {
                    let r = matches!(self.stack.pop(), Value::Immediate(Immediate::Number(_)));
                    self.stack.push(Value::Immediate(r.into()));
                }
                OpCode::IsLst => {
                    let r = match self.stack.pop() {
                        Value::Heap(value) => match value.as_ref() {
                            heap::Value::Pair(..) => true,
                            _ => false,
                        },
                        Value::Immediate(Immediate::Nil) => true,
                        _ => false,
                    };
                    self.stack.push(Value::Immediate(r.into()));
                }
                OpCode::IsNil => {
                    let r = matches!(self.stack.pop(), Value::Immediate(Immediate::Nil));
                    self.stack.push(Value::Immediate(r.into()));
                }
                OpCode::IsSym => {
                    let r = matches!(self.stack.pop(), Value::Immediate(Immediate::Symbol(_)));
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
                OpCode::Call(argcnt) => match self.stack.pop() {
                    Value::Closure(v) => {
                        //
                        // Unpack the closure.
                        //
                        let (argpak, paklen) = self.stack.unpack(v);
                        //
                        // Decode the funcall.
                        //
                        match self.stack.pop().as_immediate() {
                            Immediate::Funcall(addr, Arity::All) => {
                                //
                                // Collect the arguments into a list.
                                //
                                self.stack.list(argcnt);
                                //
                                // Push the return link and go to the funcall address.
                                //
                                self.stack.push(Value::Link(pc + 1).into());
                                pc = addr as usize;
                                continue;
                            }
                            Immediate::Funcall(addr, arity @ Arity::Some(argexp)) => {
                                //
                                // Pack in case of currying.
                                //
                                if argcnt + argpak < argexp as usize {
                                    let imm = Immediate::Funcall(addr, arity);
                                    self.stack.push(Value::Immediate(imm));
                                    self.stack.pack(argcnt + argpak, argcnt + paklen);
                                }
                                //
                                // Push the return link and go to the funcall address.
                                //
                                else {
                                    self.stack.push(Value::Link(pc + 1).into());
                                    pc = addr as usize;
                                    continue;
                                }
                            }
                            Immediate::Funcall(addr, arity @ Arity::SomeWithRem(argexp)) => {
                                //
                                // Pack in case of currying.
                                //
                                if argcnt + argpak < argexp as usize {
                                    let imm = Immediate::Funcall(addr, arity);
                                    self.stack.push(Value::Immediate(imm));
                                    self.stack.pack(argcnt + argpak, argcnt + paklen);
                                }
                                //
                                // Push the return link and go to the funcall address.
                                //
                                else {
                                    //
                                    // Prepare the arguments, only if there are more
                                    // provided than expected.
                                    //
                                    if argcnt + argpak > argexp as usize {
                                        for _ in 0..argexp {
                                            self.stack.rotate(argcnt + argpak);
                                        }
                                    }
                                    //
                                    // Collect the remaining arguments into a list.
                                    //
                                    self.stack.list(argcnt + argpak - argexp as usize);
                                    //
                                    // Rotate the arguments.
                                    //
                                    self.stack.rotate(argexp as usize + 1);
                                    //
                                    // Push the link value.
                                    //
                                    self.stack.push(Value::Link(pc + 1).into());
                                    pc = addr as usize;
                                    continue;
                                }
                            }
                            Immediate::Syscall(index, argexp) => {
                                //
                                // Pack in case of currying.
                                //
                                if argcnt + argpak < argexp as usize {
                                    let imm = Immediate::Syscall(index, argexp);
                                    self.stack.push(Value::Immediate(imm));
                                    self.stack.pack(argcnt + argpak, argcnt + paklen);
                                }
                                //
                                // Push the return link and go to the funcall address.
                                //
                                else {
                                    let values = self.stack.slice_n(argexp as usize);
                                    let res = syscalls::call(index, values);
                                    self.stack.drop(argexp as usize);
                                    self.stack.push(res);
                                }
                            }
                            _ => panic!("Expected a funcall or syscall"),
                        }
                    }
                    Value::Immediate(Immediate::Funcall(addr, Arity::All)) => {
                        //
                        // Collect the arguments into a list.
                        //
                        self.stack.list(argcnt);
                        //
                        // Push the return link and go to the funcall address.
                        //
                        self.stack.push(Value::Link(pc + 1).into());
                        pc = addr as usize;
                        continue;
                    }
                    Value::Immediate(Immediate::Funcall(addr, arity @ Arity::Some(argexp))) => {
                        //
                        // Pack in case of currying.
                        //
                        if argcnt < argexp as usize {
                            let imm = Immediate::Funcall(addr, arity);
                            self.stack.push(Value::Immediate(imm));
                            self.stack.pack(argcnt, argcnt + 1);
                        }
                        //
                        // Push the return link and go to the funcall address.
                        //
                        else {
                            self.stack.push(Value::Link(pc + 1).into());
                            pc = addr as usize;
                            continue;
                        }
                    }
                    Value::Immediate(Immediate::Funcall(
                        addr,
                        arity @ Arity::SomeWithRem(argexp),
                    )) => {
                        //
                        // Pack in case of currying.
                        //
                        if argcnt < argexp as usize {
                            let imm = Immediate::Funcall(addr, arity);
                            self.stack.push(Value::Immediate(imm));
                            self.stack.pack(argcnt, argcnt + 1);
                        }
                        //
                        // Push the return link and go to the funcall address.
                        //
                        else {
                            //
                            // Prepare the arguments, only if there are more
                            // provided than expected.
                            //
                            if argcnt > argexp as usize {
                                for _ in 0..argexp {
                                    self.stack.rotate(argcnt);
                                }
                            }
                            //
                            // Collect the remaining arguments into a list.
                            //
                            self.stack.list(argcnt - argexp as usize);
                            //
                            // Rotate the arguments.
                            //
                            self.stack.rotate(argexp as usize + 1);
                            //
                            // Push the link value.
                            //
                            self.stack.push(Value::Link(pc + 1).into());
                            pc = addr as usize;
                            continue;
                        }
                    }
                    Value::Immediate(Immediate::Syscall(index, argexp)) => {
                        //
                        // Pack in case of currying.
                        //
                        if argcnt < argexp as usize {
                            let imm = Immediate::Syscall(index, argexp);
                            self.stack.push(Value::Immediate(imm));
                            self.stack.pack(argcnt, argcnt + 1);
                        }
                        //
                        // Push the return link and go to the funcall address.
                        //
                        else {
                            let values = self.stack.slice_n(argexp as usize);
                            let res = syscalls::call(index, values);
                            self.stack.drop(argexp as usize);
                            self.stack.push(res);
                        }
                    }
                    _ => panic!("Expected a closure, funcall or syscall"),
                },
                OpCode::Ret => {
                    pc = self.stack.unlink().link();
                    continue;
                }
                //
                // self.stack operations.
                //
                OpCode::Dup(v) => self.stack.dup(v),
                OpCode::Get(v) => self.stack.get(v),
                OpCode::Lst(n) => self.stack.list(n),
                OpCode::Pak(v) => self.stack.pack(0, v),
                OpCode::Pop(v) => self.stack.drop(v),
                OpCode::Psh(v) => self.stack.push(Value::from(v).into()),
                OpCode::Rot(n) => self.stack.rotate(n),
                OpCode::Rtm(m, n) => self.stack.rotate_n(m, n),
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
