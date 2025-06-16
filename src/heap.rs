use std::rc::Rc;

use crate::{opcodes::Immediate, stack};

//
// Value.
//

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Value {
    Closure(stack::Closure),
    Immediate(Immediate),
    Pair(Rc<Value>, Rc<Value>),
}

impl Value {
    pub fn iter(&self) -> ValueIterator {
        ValueIterator(Rc::new(self.clone()))
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
