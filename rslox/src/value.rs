use std::ops::Neg;

use strum_macros::*;

use crate::vm::InterpretError;

#[derive(Debug, EnumTryAs, strum_macros::Display, Clone, Copy, PartialEq)]
pub enum Object {
    String(&'static str),
    Object(f64)
}

// Copy is implemented instead of a bespoke Clone that properly reallocates the string because we
// don't want to reallocate the string when popping it off the stack
#[derive(Debug, Default, EnumTryAs, strum_macros::Display, Clone, Copy, PartialEq)]
pub enum Value {
    #[default]
    Nil,
    #[strum(to_string = "{0}")]
    Bool(bool),
    #[strum(to_string = "{0}")]
    Float(f64),
    String(&'static str),
    // #[strum(to_string = "{0}")]
    Object(*mut Object),
}

impl Value {
    pub const TRUE: Self = Value::Bool(true);
    pub const FALSE: Self = Value::Bool(false);

    pub fn alloc_string(src: &str) -> Self {
        let val: &mut str = Box::leak(Box::<str>::from(src));
        Self::String(val)
    }

    pub fn dealloc(self) {
        match self {
            Value::String(s) => unsafe {
                let _ = Box::from_raw(s as *const str as *mut str);
            },
            Value::Object(o) => unsafe {
                let _ = Box::from_raw(o);
            },
            _ => (),
        }
    }

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

    /// Adds the given value to `self` in-place
    pub fn add(&mut self, b: &Value) -> Result<(), InterpretError> {
        match (self, b) {
            (Value::Float(x), Value::Float(y)) => {
                *x += y;
                Ok(())
            }
            (Value::String(s1), Value::String(s2)) => {
                let mut concat = s1.to_owned();
                concat.push_str(s2);
                let new = concat.into_boxed_str();

                // this leaks the old string
                *s1 = Box::leak(new);

                Ok(())
            }
            x => Err(InterpretError::RuntimeError(format!(
                "Add called with non-number: {x:?} "
            ))),
        }
    }

    /// Creates a new string, format!("{self}{b}")
    pub fn concat(&self, b: &Value) -> Result<Value, InterpretError> {
        match (self, b) {
            (Value::String(s1), Value::String(s2)) => {
                let mut concat: String = (*s1).to_owned();
                concat.push_str(s2);

                Ok(Self::alloc_string(&concat))
            }
            x => Err(InterpretError::RuntimeError(format!(
                "Add called with non-string operands: {x:?} "
            ))),
        }
    }


    /// Subtracts the given value from `self` in-place
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
