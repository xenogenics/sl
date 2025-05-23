use crate::opcodes::Immediate;

pub struct Stack {
    head: usize,
    data: Box<[Immediate]>,
}

impl Stack {
    pub fn new(size: usize) -> Self {
        Self {
            head: 0,
            data: vec![Immediate::Nil; size].into_boxed_slice(),
        }
    }

    pub const fn push(&mut self, v: Immediate) {
        unsafe {
            self.data.as_mut_ptr().add(self.head).write(v);
            self.head = self.head.unchecked_add(1);
        }
    }

    pub const fn pop(&mut self) -> Immediate {
        unsafe {
            self.head = self.head.unchecked_sub(1);
            self.data.as_ptr().add(self.head).read()
        }
    }

    pub const fn drop(&mut self, n: usize) {
        unsafe {
            self.head = self.head.unchecked_sub(n);
        }
    }

    pub const fn dup(&mut self, n: usize) {
        unsafe {
            let p = self.data.as_mut_ptr().add(self.head - n);
            p.copy_to_nonoverlapping(p.add(n), n);
            self.head = self.head.unchecked_add(n);
        }
    }

    pub const fn pick(&mut self, n: usize) {
        unsafe {
            let v = self.data.as_ptr().add(self.head - n).read();
            self.data.as_mut_ptr().add(self.head).write(v);
            self.head = self.head.unchecked_add(1);
        }
    }

    pub const fn rotate(&mut self, n: usize) {
        unsafe {
            let cur = self.data.as_ptr().add(self.head - 1).read();
            let p = self.data.as_mut_ptr().add(self.head - n);
            p.copy_to(p.add(1), n - 1);
            p.write(cur);
        }
    }

    pub const fn swap(&mut self) {
        unsafe {
            let a = self.data.as_mut_ptr().add(self.head - 1);
            let b = self.data.as_mut_ptr().add(self.head - 2);
            let v = a.read();
            a.write(b.read());
            b.write(v);
        }
    }

    pub const fn unlink(&mut self) -> Immediate {
        unsafe {
            let a = self.data.as_mut_ptr().add(self.head - 1);
            let b = self.data.as_mut_ptr().add(self.head - 2);
            let v = b.read();
            b.write(a.read());
            self.head = self.head.unchecked_sub(1);
            v
        }
    }
}

impl std::fmt::Debug for Stack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        for i in 0..self.head {
            write!(f, "{:?}", self.data[i])?;
            if i != self.head - 1 {
                write!(f, ", ")?;
            }
        }
        write!(f, "]")
    }
}
