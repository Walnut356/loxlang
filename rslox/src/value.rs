use std::{ops::Neg, ptr::NonNull, time::UNIX_EPOCH};

use strum::VariantNames;
use strum_macros::*;
use tracing::{Level, debug, instrument};

use crate::{chunk::Chunk, table::Table, vm::InterpretError};

#[derive(Debug, Clone, Copy)]
pub struct Object {
    pub marked: bool,
}

#[derive(Debug, Default)]
pub struct Function {
    pub name: &'static str,
    pub chunk: Chunk,
    pub upval_count: u8,
    pub arg_count: u8,
    pub marked: bool,
}

impl std::fmt::Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = if self.name.is_empty() {
            "script"
        } else {
            self.name
        };

        write!(f, "<fn {name}>")
    }
}

#[derive(Debug)]
pub struct Closure {
    pub func: NonNull<Function>,
    /// Stores pointers to Value::Upvalue
    pub upvals: Vec<NonNull<UpVal>>,
    pub marked: bool,
}

impl Default for Closure {
    fn default() -> Self {
        Self {
            func: NonNull::dangling(),
            upvals: Default::default(),
            marked: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum UpVal {
    Open(NonNull<Value>, bool),
    Closed(Value, bool),
}

// Copy is implemented instead of a bespoke Clone that properly reallocates the string because we
// don't want to reallocate the string when popping it off the stack
#[derive(Debug, EnumTryAs, VariantNames, Clone, Copy)]
#[repr(u8)]
pub enum Value {
    Nil,
    // #[strum(to_string = "{0}")]
    Bool(bool),
    // #[strum(to_string = "{0}")]
    Float(f64),
    NativeFn(fn(&[Value]) -> Value),
    // #[strum(to_string = "{0}")]
    String(&'static str),
    // #[strum(to_string = "{0}")]
    Function(NonNull<Function>),
    Closure(NonNull<Closure>),
    UpValue(NonNull<UpVal>),
    Object(NonNull<Object>),
}

impl Default for Value {
    fn default() -> Self {
        Self::Nil
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Nil => write!(f, "Nil"),
            Value::Bool(x) => write!(f, "{}", *x),
            Value::Float(x) => write!(f, "{}", *x),
            Value::NativeFn(_) => write!(f, "<native function>"),
            Value::String(x) => write!(f, "\"{}\"", *x),
            Value::Function(x) => write!(f, "Function({})", unsafe { x.as_ref() }.name),
            Value::Object(x) => write!(f, "Object({:?})", *x),
            Value::Closure(x) => write!(f, "Closure(<fn {}>)", unsafe {
                x.as_ref().func.as_ref().name
            }),
            Value::UpValue(_) => write!(f, "<upval>"),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Bool(l0), Self::Bool(r0)) => l0 == r0,
            (Self::Float(l0), Self::Float(r0)) => l0 == r0,
            (Self::String(l0), Self::String(r0)) => l0.as_ptr() == r0.as_ptr(),
            (Self::Object(l0), Self::Object(r0)) => (*l0).addr() == (*r0).addr(),
            _ => false,
        }
    }
}

impl Value {
    pub const TRUE: Self = Value::Bool(true);
    pub const FALSE: Self = Value::Bool(false);

    pub const CLOCK: Self = Value::NativeFn(|_| {
        Value::Float(
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
        )
    });

    pub fn alloc_str(src: &str, string_table: &mut Table) -> Self {
        let val = match string_table.get_key(src) {
            Some(s) => s,
            None => {
                let str = Box::leak(Box::<str>::from(src));
                string_table.insert(
                    unsafe { (str as *const str).as_ref().unwrap() },
                    Value::Bool(true),
                );

                str
            }
        };

        Self::String(val)
    }

    pub fn alloc_string(src: String, string_table: &mut Table) -> Self {
        let val = match string_table.get_key(&src) {
            Some(s) => s,
            None => Box::leak(Box::<str>::from(src)),
        };

        Self::String(val)
    }

    #[instrument(level = Level::DEBUG, skip(heap_objects))]
    pub fn alloc_func(heap_objects: &mut Vec<Value>) -> NonNull<Function> {
        let func = Box::leak(Box::new(Function::default()));
        let func = unsafe { NonNull::new_unchecked(func) };

        heap_objects.push(Value::Function(func));

        func
    }

    #[instrument(level = Level::DEBUG, skip(heap_objects))]
    pub fn alloc_closure(
        func: NonNull<Function>,
        heap_objects: &mut Vec<Value>,
    ) -> NonNull<Closure> {
        let closure = Box::leak(Box::new(Closure {
            func,
            upvals: Vec::new(),
            marked: false,
        }));

        let closure = unsafe { NonNull::new_unchecked(closure) };

            heap_objects.push(Value::Closure(closure));

        closure
    }

    #[instrument(level = Level::DEBUG, skip(heap_objects))]
    pub fn alloc_upval(val: NonNull<Value>, heap_objects: &mut Vec<Value>) -> NonNull<UpVal> {
        let upval = Box::leak(Box::new(UpVal::Open(val, false)));
        let upval = unsafe { NonNull::new_unchecked(upval) };
            heap_objects.push(Value::UpValue(upval));

        upval
    }

    #[instrument(level = Level::DEBUG)]
    pub fn dealloc(self) {
        match self {
            Value::String(s) => unsafe {
                let _ = Box::from_raw(s as *const str as *mut str);
            },
            Value::Object(o) => unsafe {
                let _ = Box::from_raw(o.as_ptr());
            },
            Value::Function(f) => unsafe {
                let _ = Box::from_raw(f.as_ptr());
            },
            Value::Closure(c) => unsafe {
                let _ = Box::from_raw(c.as_ptr());
            },
            Value::UpValue(v) => unsafe {
                let _ = Box::from_raw(v.as_ptr());
            },
            _ => (),
        }
    }

    pub fn mark(&mut self) {
        unsafe {
            match self {
                Value::String(_) => (),
                Value::Function(f) => f.as_mut().marked = true,
                Value::Closure(c) => c.as_mut().marked = true,
                Value::UpValue(u) => match u.as_mut() {
                    UpVal::Open(_, marked) => *marked = true,
                    UpVal::Closed(_, marked) => *marked = true,
                },
                Value::Object(o) => o.as_mut().marked = true,
                _ => (),
            }
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
    pub fn add(&mut self, b: &Value, string_table: &mut Table) -> Result<(), InterpretError> {
        match (self, b) {
            (Value::Float(x), Value::Float(y)) => {
                *x += y;
                Ok(())
            }
            (Value::String(s1), Value::String(s2)) => {
                let mut concat = s1.to_owned();
                concat.push_str(s2);

                // this leaks the old string, but it should be interned and "owned" by the string
                // table so that's fine
                *s1 = Value::alloc_string(concat, string_table)
                    .try_as_string()
                    .unwrap();

                Ok(())
            }
            x => Err(InterpretError::RuntimeError(format!(
                "Add called with non-number: {x:?} "
            ))),
        }
    }
    /*
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
    } */

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
