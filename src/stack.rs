use std::rc::Rc;

use crate::{heap, opcodes::Immediate};

//
// Closure.
//

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Closure {
    args: usize,
    vals: Rc<[Value]>,
}

impl Closure {
    fn new(args: usize, vals: Rc<[Value]>) -> Self {
        Self { args, vals }
    }
}

//
// Value.
//

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Value {
    Closure(Closure),
    Heap(Rc<heap::Value>),
    Immediate(Immediate),
    Link(usize),
}

impl Value {
    pub fn as_immediate(&self) -> Immediate {
        match self {
            Value::Immediate(v) => *v,
            _ => panic!("Expected an immediate value"),
        }
    }

    pub fn link(&self) -> usize {
        match self {
            Value::Link(v) => *v,
            _ => panic!("Expected a return link"),
        }
    }
}

impl From<Immediate> for Value {
    fn from(value: Immediate) -> Self {
        Self::Immediate(value)
    }
}

//
// Stack.
//

#[derive(Debug)]
pub struct Stack(Vec<Value>);

impl Stack {
    pub fn new(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    pub fn push(&mut self, v: Value) {
        self.0.push(v);
    }

    pub fn peek(&self) -> &Value {
        unsafe { self.0.last().unwrap_unchecked() }
    }

    pub fn pop(&mut self) -> Value {
        unsafe { self.0.pop().unwrap_unchecked() }
    }

    pub fn drop(&mut self, n: usize) {
        for _ in 0..n {
            self.0.pop();
        }
    }

    pub fn dup(&mut self, n: usize) {
        let len = self.0.len();
        for i in len - n..len {
            self.push(self.0[i].clone());
        }
    }

    pub fn get(&mut self, n: usize) {
        self.push(self.0[self.0.len() - n].clone());
    }

    pub const fn rotate(&mut self, n: usize) {
        unsafe {
            let len = self.0.len();
            let cur = self.0.as_ptr().add(len - 1).read();
            let p = self.0.as_mut_ptr().add(len - n);
            p.copy_to(p.add(1), n - 1);
            p.write(cur);
        }
    }

    pub const fn swap(&mut self) {
        unsafe {
            let n = self.0.len();
            let a = self.0.as_mut_ptr().add(n - 1);
            let b = self.0.as_mut_ptr().add(n - 2);
            let v = a.read();
            a.write(b.read());
            b.write(v);
        }
    }

    pub fn pack(&mut self, args: usize, total: usize) {
        let v: Rc<[Value]> = self.0[self.0.len() - total..].into();
        self.drop(total);
        self.push(Value::Closure(Closure::new(args, v)))
    }

    pub fn unpack(&mut self, closure: Closure) -> (usize, usize) {
        let result = (closure.args, closure.vals.len());
        self.0.extend_from_slice(closure.vals.as_ref());
        result
    }

    pub fn unlink(&mut self) -> Value {
        self.0.remove(self.0.len() - 2)
    }
}
