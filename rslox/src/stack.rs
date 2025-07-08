use std::ptr::null_mut;

use crate::{value::Value, vm::InterpretError};

/// Not 100% necessary as I could just use a Vec, but this should be a bit faster since we can stack
/// allocate it and we don't have to deal with the vec bookkeeping
#[derive(Debug)]
pub struct Stack<const N: usize> {
    pub cursor: usize,
    pub data: [Value; N],
}

impl<const N: usize> Default for Stack<N> {
    fn default() -> Self {
        Self {
            cursor: 0,
            data: [Value::Float(0.0); N],
        }
    }
}

impl<const N: usize> Stack<N> {
    pub fn clear(&mut self) {
        self.cursor = 0;
    }

    pub fn top(&self) -> &Value {
        &self.data[self.cursor - 1]
    }

    pub fn top_mut(&mut self) -> &mut Value {
        &mut self.data[self.cursor - 1]
    }

    pub fn push(&mut self, val: Value) -> Result<(), InterpretError> {
        if self.cursor > N {
            return Err(InterpretError::RuntimeError("Stack overflow".to_owned()));
        }

        self.data[self.cursor] = val;

        self.cursor += 1;

        Ok(())
    }

    pub fn pop(&mut self) -> Result<Value, InterpretError> {
        if self.cursor == 0 {
            return Err(InterpretError::RuntimeError("Stack underflow".to_owned()));
        }

        self.cursor -= 1;

        Ok(self.data[self.cursor])
    }
}
