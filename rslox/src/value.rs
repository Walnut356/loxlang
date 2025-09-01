use std::{
    alloc::{self, Layout, handle_alloc_error},
    ptr::{self, NonNull},
    time::UNIX_EPOCH,
};

use strum_macros::*;
use tracing::{Level, instrument};

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

// impl Default for Function {
//     fn default() -> Self {
//         Self {
//             name: Default::default(),
//             chunk: Default::default(),
//             upval_count: Default::default(),
//             arg_count: Default::default(),
//             marked: true,
//         }
//     }
// }

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

#[derive(Debug)]
#[repr(C)]
pub struct LoxStrInner {
    marked: bool,
    data: str,
}

impl LoxStrInner {
    /// returns an uninitialized LoxStr that **is not zeroed**, though `self.marked` is set to false
    pub fn new(data: &str) -> NonNull<Self> {
        let layout = Layout::new::<bool>();
        let layout = layout
            .extend(Layout::array::<u8>(data.len()).unwrap())
            .unwrap()
            .0;
        let layout = layout.pad_to_align();

        let addr = match layout.size() {
            0 => ptr::NonNull::dangling().as_ptr(),
            _ => {
                let addr = unsafe { alloc::alloc(layout) };
                if addr.is_null() {
                    handle_alloc_error(layout);
                }
                addr
            }
        };

        let result = ptr::slice_from_raw_parts_mut(addr, data.len()) as *mut LoxStrInner;

        unsafe {
            ptr::copy_nonoverlapping(
                data.as_ptr(),
                result.as_mut().unwrap().data.as_mut_ptr(),
                data.len(),
            )
        };

        let mut result = unsafe { NonNull::new_unchecked(result) };

        unsafe { result.as_mut().marked = false };

        result
    }

    /// returns an uninitialized LoxStr that **is not zeroed**, though `self.marked` is set to false
    fn new_sized(len: usize) -> NonNull<Self> {
        let layout = Layout::new::<bool>();
        let layout = layout.extend(Layout::array::<u8>(len).unwrap()).unwrap().0;
        let layout = layout.pad_to_align();

        let addr = match layout.size() {
            0 => ptr::NonNull::dangling().as_ptr(),
            _ => {
                let addr = unsafe { alloc::alloc(layout) };
                if addr.is_null() {
                    handle_alloc_error(layout);
                }
                addr
            }
        };

        let result = ptr::slice_from_raw_parts_mut(addr, len) as *mut LoxStrInner;
        let mut result = unsafe { NonNull::new_unchecked(result) };

        unsafe { result.as_mut().marked = false };

        result
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct LoxStr(pub NonNull<LoxStrInner>);

impl LoxStr {
    // next level stupid, but saves allocations
    pub const EMPTY: Self = {
        const TEMP: (bool, [u8; 0]) = const { (true, []) };

        let t: *const (bool, [u8; 0]) = const { &TEMP as *const _ };

        let ptr = ptr::slice_from_raw_parts_mut(t as *mut u8, 0) as *mut LoxStrInner;

        Self(NonNull::new(ptr).unwrap())
    };

    pub fn new(data: &str) -> Self {
        Self(LoxStrInner::new(data))
    }

    pub fn str(&self) -> &'static str {
        unsafe { &self.0.as_ref().data }
    }

    fn str_mut(&mut self) -> &'static mut str {
        unsafe { &mut self.0.as_mut().data }
    }

    pub fn new_concat(s1: &str, s2: &str) -> Self {
        let mut res = Self(LoxStrInner::new_sized(s1.len() + s2.len()));

        unsafe {
            ptr::copy_nonoverlapping(s1.as_ptr(), res.str_mut().as_mut_ptr(), s1.len());
        }
        unsafe {
            ptr::copy_nonoverlapping(
                s2.as_ptr(),
                res.str_mut().as_mut_ptr().add(s1.len()),
                s2.len(),
            );
        }

        res
    }

    pub fn mark(&mut self) {
        unsafe {
            self.0.as_mut().marked = true;
        }
    }

    pub fn unmark(&mut self) {
        unsafe {
            self.0.as_mut().marked = false;
        }
    }

    pub fn is_marked(&self) -> bool {
        unsafe { self.0.as_ref().marked }
    }
}

impl Default for LoxStr {
    fn default() -> Self {
        Self::EMPTY
    }
}

impl AsRef<str> for LoxStr {
    fn as_ref(&self) -> &str {
        self.str()
    }
}

impl std::fmt::Display for LoxStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.str())
    }
}

impl std::fmt::Debug for LoxStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "LoxStr(\"{}\", marked: {})",
            self.str(),
            self.is_marked()
        )
    }
}

// Copy is implemented instead of a bespoke Clone that properly reallocates the string because we
// don't want to reallocate the string when popping it off the stack
#[derive(EnumTryAs, VariantNames, Clone, Copy)]
#[repr(u8)]
pub enum Value {
    Nil,
    // #[strum(to_string = "{0}")]
    Bool(bool),
    // #[strum(to_string = "{0}")]
    Float(f64),
    NativeFn(fn(&[Value]) -> Value),
    // #[strum(to_string = "{0}")]
    String(LoxStr),
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
            Value::Nil => write!(f, "nil"),
            Value::Bool(x) => write!(f, "{}", *x),
            Value::Float(x) => write!(f, "{}", *x),
            Value::NativeFn(_) => write!(f, "<native fn>"),
            Value::String(x) => write!(f, "{}", x.str()),
            Value::Function(x) => write!(f, "Function({})", unsafe { x.as_ref() }.name),
            Value::Object(x) => write!(f, "Object({:?})", *x),
            Value::Closure(x) => write!(f, "Closure(<fn {}>)", unsafe {
                x.as_ref().func.as_ref().name
            }),
            Value::UpValue(_) => write!(f, "<upval>"),
        }
    }
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Nil => write!(f, "Nil"),
            Self::Bool(arg0) => f.debug_tuple("Bool").field(arg0).finish(),
            Self::Float(arg0) => f.debug_tuple("Float").field(arg0).finish(),
            Self::NativeFn(arg0) => f.debug_tuple("NativeFn").field(arg0).finish(),
            Self::String(arg0) => f.debug_tuple("String").field(&format!("{}", arg0)).finish(),
            Self::Function(arg0) => f.debug_tuple("Function").field(arg0).finish(),
            Self::Closure(arg0) => f.debug_tuple("Closure").field(arg0).finish(),
            Self::UpValue(arg0) => f.debug_tuple("UpValue").field(arg0).finish(),
            Self::Object(arg0) => f.debug_tuple("Object").field(arg0).finish(),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Nil, Self::Nil) => true,
            (Self::Bool(l0), Self::Bool(r0)) => l0 == r0,
            (Self::Float(l0), Self::Float(r0)) => l0 == r0,
            (Self::String(l0), Self::String(r0)) => std::ptr::addr_eq(l0.0.as_ptr(), r0.0.as_ptr()),
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

    pub fn size(&self) -> usize {
        match self {
            Value::Nil | Value::Bool(_) | Value::Float(_) | Value::NativeFn(_) => {
                size_of::<Value>()
            }
            Value::String(lox_str) => lox_str.str().len() + 1,
            Value::Function(_) => size_of::<Function>(),
            Value::Closure(_) => size_of::<Closure>(),
            Value::UpValue(_) => size_of::<UpVal>(),
            Value::Object(_) => size_of::<Object>(),
        }
    }

    pub fn alloc_str(src: &str, string_table: &mut Table, heap_objects: &mut Vec<Value>) -> Self {
        if src.is_empty() {
            // we intentionally don't add the empty string to the heap object or string table
            // because we cannot deallocate LoxStr::EMPTY
            return Value::String(LoxStr::EMPTY);
        }
        match string_table.get_key(src) {
            Some(s) => Self::String(s),
            None => {
                let str = LoxStr::new(src);
                string_table.insert(str, Value::Bool(true));

                let res = Self::String(str);
                heap_objects.push(res);

                res
            }
        }
    }

    pub fn alloc_string(
        src: String,
        string_table: &mut Table,
        heap_objects: &mut Vec<Value>,
    ) -> Self {
        match string_table.get_key(&src) {
            Some(s) => Self::String(s),
            None => {
                let str = LoxStr::new(&src);
                string_table.insert(str, Value::Bool(true));

                let res = Self::String(str);
                heap_objects.push(res);

                res
            }
        }
    }

    #[instrument(level = Level::DEBUG, skip(heap_objects))]
    pub fn alloc_func(heap_objects: &mut Vec<Value>) -> NonNull<Function> {
        let func = Box::leak(Box::new(Function::default()));
        let func = unsafe { NonNull::new_unchecked(func) };

        heap_objects.push(Value::Function(func));

        func
    }

    #[instrument(level = Level::DEBUG, skip(heap_objects), fields(deref=unsafe{func.as_ref().to_string()}))]
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
                let _ = Box::from_raw(s.0.as_ptr());
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
                Value::String(s) => s.mark(),
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

    pub fn unmark(&mut self) {
        unsafe {
            match self {
                Value::String(s) => s.unmark(),
                Value::Function(f) => f.as_mut().marked = false,
                Value::Closure(c) => c.as_mut().marked = false,
                Value::UpValue(u) => match u.as_mut() {
                    UpVal::Open(_, marked) => *marked = false,
                    UpVal::Closed(_, marked) => *marked = false,
                },
                Value::Object(o) => o.as_mut().marked = false,
                _ => (),
            }
        }
    }

    pub fn is_marked(&self) -> bool {
        unsafe {
            match self {
                Value::String(s) => s.is_marked(),
                Value::Function(f) => f.as_ref().marked,
                Value::Closure(c) => c.as_ref().marked,
                Value::UpValue(u) => match u.as_ref() {
                    UpVal::Open(_, marked) => *marked,
                    UpVal::Closed(_, marked) => *marked,
                },
                Value::Object(o) => o.as_ref().marked,
                _ => true,
            }
        }
    }

    /// returns true if this value has child allocations, thus if the value should be added to the
    /// grey stack when garbage collecting
    pub fn has_child_allocs(&self) -> bool {
        matches!(
            self,
            Value::Function(_) | Value::Closure(_) | Value::UpValue(_)
        )
    }

    /// negates `self` in-place
    pub fn negate(&mut self) -> Result<(), InterpretError> {
        match self {
            Value::Float(x) => *x = -(*x),
            _ => {
                return Err(InterpretError::RuntimeError(format!(
                    "Negate called with non-number operand: {self:?}"
                )));
            }
        }

        Ok(())
    }

    /// Adds the given value to `self` in-place
    pub fn add(
        &mut self,
        b: &Value,
        string_table: &mut Table,
        heap_objects: &mut Vec<Value>,
    ) -> Result<(), InterpretError> {
        match (self, b) {
            (Value::Float(x), Value::Float(y)) => {
                *x += y;
                Ok(())
            }
            (Value::String(s1), Value::String(s2)) => {
                let res = LoxStr::new_concat(s1.str(), s2.str());
                let val = match string_table.get_key(res.str()) {
                    Some(s) => {
                        Value::String(res).dealloc();
                        s
                    }
                    None => {
                        string_table.insert(res, Value::Bool(true));
                        heap_objects.push(Value::String(res));

                        res
                    }
                };

                *s1 = val;

                Ok(())
            }
            x => Err(InterpretError::RuntimeError(format!(
                "Add called with non-number/non-string operands: {x:?}"
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
                "Sub called on non-number operand(s): {x:?}"
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
                "Mul called on non-number operand(s): {x:?}"
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
                "Div called with non-number operand(s): {x:?}"
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

    pub fn not_equal(&mut self, b: &Value) {
        *self = Self::Bool(self != b);
    }

    pub fn greater(&mut self, b: &Value) -> Result<(), InterpretError> {
        if let &mut Value::Float(x) = self
            && let &Value::Float(y) = b
        {
            *self = Self::Bool(x > y);
            Ok(())
        } else {
            Err(InterpretError::RuntimeError(format!(
                "Greater-than called on non-number operand: {:?}",
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
                "Greater-than-or-equal called on non-number operand: {:?}",
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
                "Less-than called on non-number operand: {:?}",
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
                "Less-than-or-equal called on non-number operand: {:?}",
                (self, b)
            )))
        }
    }
}
