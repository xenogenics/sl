use crate::{error::Error, heap, opcodes::Immediate, stack};

pub fn call(index: u32, values: &[stack::Value]) -> stack::Value {
    match index {
        0 => {
            //
            // Get the first argument.
            //
            let stack::Value::Immediate(Immediate::Number(fd)) = values[1] else {
                return stack::Value::Immediate(Immediate::Nil);
            };
            //
            // Get the argument.
            //
            let stack::Value::Heap(value) = &values[0] else {
                return stack::Value::Immediate(Immediate::Nil);
            };
            //
            // Build the bytes.
            //
            let bytes: Vec<_> = value
                .iter()
                .filter_map(|v| match v.as_ref() {
                    heap::Value::Immediate(Immediate::Char(v)) => Some(*v),
                    _ => None,
                })
                .collect();
            //
            // Write.
            //
            let n = unsafe {
                libc::write(
                    fd as i32,
                    bytes.as_ptr() as *const libc::c_void,
                    bytes.len(),
                )
            };
            //
            // Done.
            //
            stack::Value::Immediate(Immediate::Number(n as i64))
        }
        _ => stack::Value::Immediate(Immediate::Nil),
    }
}

pub fn get(name: &str) -> Result<(u32, u32), Error> {
    match name {
        "WRITE" => Ok((0, 2)),
        _ => Err(Error::InvalidSystemCall(name.into())),
    }
}
