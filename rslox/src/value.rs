use std::ops::Neg;

use strum_macros::*;

use crate::vm::InterpretError;

#[derive(Debug, Default, EnumTryAs, strum_macros::Display, Clone, Copy, PartialEq)]
pub enum Value {
    #[default]
    Nil,
    #[strum(to_string = "{0}")]
    Bool(bool),
    #[strum(to_string = "Float({0})")]
    Float(f64),
    // #[strum(to_string = "{0}")]
    // String(String)
}

impl Value {
    pub const TRUE: Self = Value::Bool(true);
    pub const FALSE: Self = Value::Bool(false);


    /// negates `self` in-place
    pub fn negate(&mut self) -> Result<(), InterpretError> {
        match self {
            Value::Float(x) => *x = -(*x),
            _ => return Err(InterpretError::RuntimeError(format!("Negate called on non-number operand: {self:?} "))),
        }

        Ok(())
    }

    /// Subtracts the given value from `self` in-place
    pub fn add(&mut self, b: &Value) {
        match (self, b) {
            (Value::Float(x), Value::Float(y)) => *x += y,
            _ => ()
        }
    }

    /// Subtracts the given value from `self` in-place
    pub fn sub(&mut self, b: &Value) {
        match (self, b) {
            (Value::Float(x), Value::Float(y)) => *x -= y,
            _ => ()
        }
    }

    /// Subtracts the given value from `self` in-place
    pub fn mul(&mut self, b: &Value) {
        match (self, b) {
            (Value::Float(x), Value::Float(y)) => *x *= y,
            _ => ()
        }
    }

    /// Subtracts the given value from `self` in-place
    pub fn div(&mut self, b: &Value) {
        match (self, b) {
            (Value::Float(x), Value::Float(y)) => *x /= y,
            _ =>()
        }
    }
}
