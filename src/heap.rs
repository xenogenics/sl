use std::rc::Rc;

use crate::{opcodes::Immediate, stack};

//
// Value.
//

#[derive(Clone, Debug, Eq)]
pub enum Value {
    Closure(stack::Closure),
    Immediate(Immediate),
    Pair(Rc<Value>, Rc<Value>),
}

impl Value {
    pub fn iter(&self) -> ValueIterator {
        ValueIterator(Rc::new(self.clone()))
    }

    pub fn conc(a: Rc<Value>, b: Rc<Value>) -> Rc<Value> {
        match a.as_ref() {
            Value::Pair(car, cdr) => Self::cons(car.clone(), Self::conc(cdr.clone(), b)),
            _ => b,
        }
    }

    pub fn cons(a: Rc<Value>, b: Rc<Value>) -> Rc<Value> {
        Self::Pair(a, b).into()
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
            (Value::Immediate(a), Value::Immediate(b)) => a == b,
            (Value::Pair(car0, cdr0), Value::Pair(car1, cdr1)) => car0 == car1 && cdr0 == cdr1,
            //
            // Default.
            //
            _ => false,
        }
    }
}

//
// Value iterator.
//

pub struct ValueIterator(Rc<Value>);

impl std::iter::Iterator for ValueIterator {
    type Item = Rc<Value>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0.as_ref() {
            Value::Pair(car, cdr) => {
                let result = car.clone();
                self.0 = cdr.clone();
                Some(result)
            }
            _ => None,
        }
    }
}
