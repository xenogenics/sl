use std::rc::Rc;

use crate::{opcodes::Immediate, stack};

//
// Value.
//

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Value {
    Closure(Rc<[stack::Value]>),
    Immediate(Immediate),
    Pair(Rc<Value>, Rc<Value>),
}
