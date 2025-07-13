use std::ops::Neg;

use strum_macros::*;

use crate::vm::InterpretError;

#[derive(Debug, Default, EnumTryAs, strum_macros::Display, Clone, Copy, PartialEq)]
pub enum Value {
    #[default]
    Nil,
    #[strum(to_string = "{0}")]
    Bool(bool),
    #[strum(to_string = "{0}")]
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
            _ => {
                return Err(InterpretError::RuntimeError(format!(
                    "Negate called with non-number operand: {self:?} "
                )));
            }
        }

        Ok(())
    }

    /// Subtracts the given value from `self` in-place
    #[inline(never)]
    pub fn add(&mut self, b: &Value) -> Result<(), InterpretError> {
        match (self, b) {
            (Value::Float(x), Value::Float(y)) => {
                *x += y;
                Ok(())
            }
            x => Err(InterpretError::RuntimeError(format!(
                "Add called with non-number operand: {x:?} "
            ))),
        }
    }

    /// Adds the given value to `self` in-place
    pub fn sub(&mut self, b: &Value) -> Result<(), InterpretError> {
        match (self, b) {
            (Value::Float(x), Value::Float(y)) => {
                *x -= y;
                Ok(())
            }
            x => Err(InterpretError::RuntimeError(format!(
                "Sub called on non-number operand: {x:?} "
            ))),
        }
    }

    /// Multiplies `self` by the given value in-place
    pub fn mul(&mut self, b: &Value) -> Result<(), InterpretError> {
        match (self, b) {
            (Value::Float(x), Value::Float(y)) => {
                *x *= y;
                Ok(())
            }
            x => Err(InterpretError::RuntimeError(format!(
                "Mul called on non-number operand: {x:?} "
            ))),
        }
    }

    /// Divides the given value by `b` in-place
    pub fn div(&mut self, b: &Value) -> Result<(), InterpretError> {
        match (self, b) {
            (Value::Float(x), Value::Float(y)) => {
                *x /= y;
                Ok(())
            }
            x => Err(InterpretError::RuntimeError(format!(
                "Div called on non-number operand: {x:?} "
            ))),
        }
    }

    pub fn not(&mut self) {
        *self = Self::Bool(self.is_falsey());
    }

    pub fn is_falsey(&self) -> bool {
        matches!(self, Value::Nil | Value::Bool(false))
    }

    pub fn is_truthy(&self) -> bool {
        !self.is_falsey()
    }

    pub fn equal(&mut self, b: &Value) {
        *self = Self::Bool(self == b);
    }

    pub fn greater(&mut self, b: &Value) -> Result<(), InterpretError> {
        if let &mut Value::Float(x) = self
            && let &Value::Float(y) = b
        {
            *self = Self::Bool(x > y);
            Ok(())
        } else {
            Err(InterpretError::RuntimeError(format!(
                "Greater-than called on non-number operand: {:?} ",
                (self, b)
            )))
        }
    }

    pub fn greater_equal(&mut self, b: &Value) -> Result<(), InterpretError> {
        if let &mut Value::Float(x) = self
            && let &Value::Float(y) = b
        {
            *self = Self::Bool(x >= y);
            Ok(())
        } else {
            Err(InterpretError::RuntimeError(format!(
                "Greater-than-or-equal called on non-number operand: {:?} ",
                (self, b)
            )))
        }
    }

    pub fn less(&mut self, b: &Value) -> Result<(), InterpretError> {
        if let &mut Value::Float(x) = self
            && let &Value::Float(y) = b
        {
            *self = Self::Bool(x < y);
            Ok(())
        } else {
            Err(InterpretError::RuntimeError(format!(
                "Less-than called on non-number operand: {:?} ",
                (self, b)
            )))
        }
    }

    pub fn less_equal(&mut self, b: &Value) -> Result<(), InterpretError> {
        if let &mut Value::Float(x) = self
            && let &Value::Float(y) = b
        {
            *self = Self::Bool(x <= y);
            Ok(())
        } else {
            Err(InterpretError::RuntimeError(format!(
                "Less-than-or-equal called on non-number operand: {:?} ",
                (self, b)
            )))
        }
    }
}
