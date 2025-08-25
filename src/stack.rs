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

#[derive(Clone, Debug, Eq)]
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

    pub fn as_link(&self) -> usize {
        match self {
            Value::Link(v) => *v,
            _ => panic!("Expected a return link"),
        }
    }

    pub fn as_mut_ptr(&self) -> *mut u8 {
        match self {
            Value::Heap(value) => value.as_mut_ptr(),
            _ => panic!("Expected a heap value"),
        }
    }

    pub fn as_raw_cstr(&self) -> *mut i8 {
        match self {
            Value::Heap(value) => value.as_raw_cstr(),
            _ => panic!("Expected a heap value"),
        }
    }
}

impl From<Immediate> for Value {
    fn from(value: Immediate) -> Self {
        Self::Immediate(value)
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            //
            // Handle wildcards.
            //
            (Value::Immediate(Immediate::Wildcard), _) => true,
            (_, Value::Immediate(Immediate::Wildcard)) => true,
            //
            // Handle member equality.
            //
            (Value::Closure(a), Value::Closure(b)) => a == b,
            (Value::Heap(a), Value::Heap(b)) => a == b,
            (Value::Immediate(a), Value::Immediate(b)) => a == b,
            (Value::Link(a), Value::Link(b)) => a == b,
            //
            // Default.
            //
            _ => false,
        }
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

    pub fn slice_n(&mut self, n: usize) -> &[Value] {
        &self.0[self.0.len() - n..]
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

    pub fn list(&mut self, n: usize) {
        //
        // Return NIL on empty list.
        //
        if n == 0 {
            self.0.push(Value::Immediate(Immediate::Nil));
        }
        //
        // Combine the N elements into a list.
        else {
            //
            // Build the list from the N elements of the stack.
            //
            let result = self.0[self.0.len() - n..].iter().fold(
                Rc::new(heap::Value::Immediate(Immediate::Nil)),
                |acc, v| {
                    //
                    // Get the next element.
                    //
                    let w = match v {
                        Value::Closure(v) => heap::Value::Closure(v.clone()).into(),
                        Value::Heap(v) => v.clone(),
                        Value::Immediate(v) => heap::Value::Immediate(*v).into(),
                        Value::Link(_) => panic!(),
                    };
                    //
                    // Build the pair.
                    //
                    Rc::new(heap::Value::Pair(w, acc))
                },
            );
            //
            // Drop the N element from the stack.
            //
            for _ in 0..n {
                self.0.pop();
            }
            //
            // Push the result.
            //
            self.0.push(Value::Heap(result));
        }
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

    pub fn rotate_n(&mut self, m: usize, n: usize) {
        let offst = self.0.len() - m;
        self.0[offst..].rotate_right(n);
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
