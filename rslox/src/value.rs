use std::ops::Neg;

use strum_macros::*;

#[derive(Debug, EnumTryAs, strum_macros::Display, Clone, Copy, PartialEq)]
pub enum Value {
    // #[default]
    // Null,
    #[strum(to_string = "Float({0})")]
    Float(f64),
}

impl Value {
    /// negates `self` in-place
    pub fn negate(&mut self) {
        match self {
            Value::Float(x) => *x = -(*x),
        }
    }

    /// Subtracts the given value from `self` in-place
    pub fn add(&mut self, b: &Value) {
        match (self, b) {
            (Value::Float(x), Value::Float(y)) => *x += y,
        }
    }

    /// Subtracts the given value from `self` in-place
    pub fn sub(&mut self, b: &Value) {
        match (self, b) {
            (Value::Float(x), Value::Float(y)) => *x -= y,
        }
    }

    /// Subtracts the given value from `self` in-place
    pub fn mul(&mut self, b: &Value) {
        match (self, b) {
            (Value::Float(x), Value::Float(y)) => *x *= y,
        }
    }

    /// Subtracts the given value from `self` in-place
    pub fn div(&mut self, b: &Value) {
        match (self, b) {
            (Value::Float(x), Value::Float(y)) => *x /= y,
        }
    }
}
