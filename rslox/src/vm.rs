use crate::{
    chunk::{Chunk, OpCode},
    compiler::Parser,
    stack::Stack,
    table::{Entry, Table},
    value::{Closure, UpVal, Value},
};
// use log::{Level, debug, error, log_enabled, trace};
use std::{cmp::Ordering, collections::BTreeMap, fmt::Write, ptr::NonNull, rc::Rc};
use thiserror::Error;
use tracing::{Level, debug, error, instrument, trace};

const MAX_FRAMES: usize = 64;
const MAX_STACK: usize = MAX_FRAMES * u8::MAX as usize;

#[derive(Error, Debug)]
pub enum InterpretError {
    #[error("{0}")]
    CompileError(String),
    #[error("{0}")]
    RuntimeError(String),
}

pub enum VMState {
    Running,
    Done,
}

#[derive(Debug)]
pub struct CallFrame {
    closure: NonNull<Closure>,
    ip: usize,
    sp: usize,
}

impl CallFrame {
    pub fn new(closure: NonNull<Closure>, sp: usize) -> Self {
        Self { closure, ip: 0, sp }
    }

    pub fn closure(&self) -> &Closure {
        unsafe { self.closure.as_ref() }
    }

    pub fn closure_mut(&mut self) -> &mut Closure {
        unsafe { self.closure.as_mut() }
    }
}

impl Default for CallFrame {
    fn default() -> Self {
        Self {
            closure: NonNull::dangling(),
            ip: 0,
            sp: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GCStats {
    pub bytes_allocated: usize,
    pub next_gc: usize,
}

impl Default for GCStats {
    fn default() -> Self {
        Self {
            bytes_allocated: Default::default(),
            next_gc: 1024 * 1024,
        }
    }
}

pub struct VM {
    // chunk: Option<Chunk>,
    clock: usize,
    heap_objects: Vec<Value>,
    strings: Table,
    globals: Table,
    upvalues: BTreeMap<usize, NonNull<UpVal>>,
    frame_count: usize,
    frames: [CallFrame; MAX_FRAMES],
    pub(crate) stack: Box<Stack<MAX_STACK>>,
    grey_stack: Vec<Value>,
    gc_stats: GCStats,
}

// impl Drop for VM {
//     fn drop(&mut self) {
//         for o in &self.heap_objects {
//             o.dealloc();
//         }
//     }
// }

impl Default for VM {
    fn default() -> Self {
        Self {
            // chunk: Default::default(),
            clock: 0,
            heap_objects: Default::default(),
            strings: Default::default(),
            globals: Default::default(),
            frame_count: Default::default(),
            frames: std::array::from_fn(|_| CallFrame::default()),
            stack: Default::default(),
            upvalues: Default::default(),
            grey_stack: Default::default(),
            gc_stats: GCStats::default(),
        }
    }
}

impl VM {
    pub const GC_HEAP_GROW_FACTOR: usize = 2;
    /// Deallocates everything necessary and resets the VM back to a default state
    #[instrument(skip_all)]
    pub fn reset(&mut self) {
        for o in &self.heap_objects {
            o.dealloc();
        }
        self.heap_objects.clear();
        self.globals.clear();
        self.strings.clear();
        self.upvalues.clear();
        // no point rewriting the bits to default values since they can't be read
        // without being overwritten first
        self.stack.cursor = 0;
        self.grey_stack.clear();
        self.frame_count = 0;
    }

    /// Shortcut for:
    /// ```ignore
    /// self.compile()?;
    /// self.run()?;
    /// ```
    pub fn interpret(&mut self, source: Rc<str>) -> Result<(), InterpretError> {
        self.compile(source)?;

        let res = self.run();

        if res.is_err() {
            self.print_stack_trace();
        }

        res
    }

    pub fn compile(&mut self, source: Rc<str>) -> Result<(), InterpretError> {
        let mut parser = Parser::new(source, &mut self.strings, &mut self.heap_objects);

        while !parser.eof() {
            parser.declaration();
        }

        if parser.errors {
            return Err(InterpretError::CompileError("".to_owned()));
        }

        parser.compiler.func.chunk.push_return(
            parser
                .compiler
                .func
                .chunk
                .lines
                .last()
                .map(|x| x.line + 1)
                .unwrap_or_default(),
        );

        debug!(
            "{}",
            parser
                .compiler
                .func
                .chunk
                .disassemble(parser.compiler.func.name)
        );

        self.stack
            .push(Value::Function(parser.compiler.func.into()))?;

        if self.time_to_gc() {
            self.collect_garbage();
        }
        let closure = Value::alloc_closure(
            self.stack.pop()?.try_as_function().unwrap(),
            &mut self.heap_objects,
        );

        self.frames[self.frame_count] = CallFrame::new(closure, self.stack.cursor);
        self.frame_count += 1;

        self.stack.push(Value::Closure(closure))?;

        // for val in unsafe { &closure.as_ref().func.as_ref().chunk.constants } {
        //     match val {
        //         Value::String(s) => {
        //             self.strings.insert(*s, Value::Bool(true));
        //         }
        //         Value::Object(_) => todo!(),
        //         _ => (),
        //     }
        // }

        self.init_natives();

        // we do this here just to prevent passing the gc stats everywhere
        // it's also convenient because we don't garbage collect during the compile phase at all
        for o in &self.heap_objects {
            self.gc_stats.bytes_allocated += o.size();
        }

        trace!("{}", {
            let mut output = "Globals:\n".to_owned();

            for v in self.globals.entries.iter().flatten() {
                output.push_str(&format!("    {}: {}", v.key, v.val));
            }

            output
        });

        Ok(())
    }

    fn init_natives(&mut self) {
        let clock = Value::alloc_str("clock", &mut self.strings, &mut self.heap_objects);
        self.globals
            .insert(clock.try_as_string().unwrap(), Value::CLOCK);
    }

    pub fn current_frame(&mut self) -> &mut CallFrame {
        &mut self.frames[self.frame_count - 1]
    }

    // fn frame_ref(&self) -> &CallFrame {
    //     &self.frames[self.frame_count - 1]
    // }

    pub fn chunk(&mut self) -> &Chunk {
        unsafe { &self.current_frame().closure.as_ref().func.as_ref().chunk }
    }

    pub fn ip(&mut self) -> &mut usize {
        &mut self.current_frame().ip
    }

    pub fn ip_copied(&self) -> usize {
        self.frames[self.frame_count - 1].ip
    }

    pub fn sp(&self) -> usize {
        self.frames[self.frame_count - 1].sp
    }

    // fn slot(&mut self, n: usize) -> &mut Value {
    //     &mut self.stack.data[self.sp() + 1 + n]
    // }

    pub fn run(&mut self) -> Result<(), InterpretError> {
        loop {
            match self.step() {
                Ok(VMState::Running) => (),
                Ok(VMState::Done) => {
                    self.reset();

                    return Ok(());
                }
                Err(e) => {
                    self.reset();
                    return Err(e);
                }
            }
        }
    }

    pub fn step_n(&mut self, mut n: usize) -> Result<(), InterpretError> {
        while n > 0 {
            match self.step() {
                Ok(VMState::Running) => n -= 1,
                Ok(VMState::Done) => {
                    self.reset();
                    return Ok(());
                }
                Err(e) => {
                    self.reset();
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    pub fn step(&mut self) -> Result<VMState, InterpretError> {
        // let frame = &mut self.frames[self.frame_count - 1];
        // let ip = &mut frame.ip;
        // let chunk = unsafe { &frame.func.as_ref().unwrap().chunk };
        let ip_copy = self.ip_copied();

        let Some(&op) = self.chunk().data.get(ip_copy) else {
            return Err(InterpretError::RuntimeError(format!(
                "[cycle: {}] No instruction at ip {ip_copy}",
                self.clock
            )));
        };

        self.clock += 1;

        trace!(target: "cycle", "{}\n{}", self.clock.clone(), {
            let mut disasm_out = String::new();
            self.chunk().disassemble_instr(&mut disasm_out, ip_copy);
            indent::indent_all_by(9, disasm_out)
        });

        *self.ip() += 1;

        let opcode = unsafe { std::mem::transmute::<u8, OpCode>(op) };

        match opcode {
            OpCode::Return => {
                let result = self.stack.pop()?;
                self.close_upval(self.sp() + 1);
                self.frame_count -= 1;

                if self.frame_count == 0 {
                    self.stack.pop()?;
                    return Ok(VMState::Done);
                }

                self.stack.cursor = self.frames[self.frame_count].sp;
                self.stack.push(result)?;
            }
            OpCode::Constant => {
                let value = self.read_const()?;
                self.stack.push(value).unwrap();
            }
            // OpCode::Constant16 => {
            //     let value = self.read_const_16()?;
            //     self.stack.push(value).unwrap();
            // }
            OpCode::DefGlobal => {
                let name = self.read_const()?;
                let Value::String(n) = name else {
                    return Err(InterpretError::RuntimeError(format!(
                        "[cycle: {}] Invalid type for global name. Expected string, got {name:?}",
                        self.clock
                    )));
                };

                self.globals.insert(n, *self.stack.top());

                self.stack.pop()?;
            }
            // OpCode::DefGlobal16 => {
            //     let name = self.read_const_16()?;
            //     let n = name.try_as_string().unwrap();

            //     self.globals.insert(n, *self.stack.top());

            //     self.stack.pop()?;
            // }
            OpCode::ReadGlobal => {
                let name = self.read_const()?;
                let n = name.try_as_string().unwrap();
                let tmp = n.str();

                match self.globals.get(n.str()) {
                    Some(x) => self.stack.push(*x)?,
                    None => {
                        return Err(InterpretError::RuntimeError(format!(
                            "[cycle: {}] Undefined variable '{n}'.",
                            self.clock
                        )));
                    }
                }
            }
            // OpCode::ReadGlobal16 => {
            //     let name = self.read_const_16()?;
            //     let n = name.try_as_string().unwrap();

            //     match self.globals.get(n.str()) {
            //         Some(x) => self.stack.push(*x)?,
            //         None => {
            //             return Err(InterpretError::RuntimeError(format!(
            //                 "Undefined variable '{n}'."
            //             )));
            //         }
            //     }
            // }
            OpCode::WriteGlobal => {
                let name = self.read_const()?;

                let n = name.try_as_string().unwrap();

                if self.globals.insert(n, *self.stack.top()) {
                    self.globals.remove(n.str());
                    return Err(InterpretError::RuntimeError(format!(
                        "[cycle: {}] Undefined variable '{n}'.",
                        self.clock
                    )));
                }
            }
            // OpCode::WriteGlobal16 => {
            //     let name = self.read_const_16()?;

            //     let n = name.try_as_string().unwrap();

            //     if self.globals.insert(n, *self.stack.top()) {
            //         self.globals.remove(n.str());
            //         return Err(InterpretError::RuntimeError(format!(
            //             "Undefined variable '{n}'."
            //         )));
            //     }
            // }
            OpCode::ReadLocal => {
                let slot = self.read_byte()? as usize;
                self.stack.push(self.stack.data[self.sp() + slot])?;
            }
            OpCode::WriteLocal => {
                let slot = self.read_byte()? as usize;
                self.stack.data[self.sp() + slot] = *self.stack.top();
            }
            OpCode::Nil => {
                self.stack.push(Value::Nil)?;
            }
            OpCode::True => {
                self.stack.push(Value::TRUE)?;
            }
            OpCode::False => {
                self.stack.push(Value::FALSE)?;
            }
            OpCode::Negate => {
                self.stack.top_mut().negate()?;
            }
            OpCode::Not => {
                self.stack.top_mut().not();
            }
            OpCode::Print => {
                println!("{}", self.stack.pop()?);
            }
            OpCode::Pop => {
                self.stack.pop()?;
            }
            OpCode::StackSub => {
                self.stack.cursor -= self.read_byte()? as usize;
            }
            OpCode::Jump => {
                let offset = self.read_u16()?;
                *self.ip() += offset as usize;
            }
            OpCode::JumpFalsey => {
                let offset = self.read_u16()?;
                if self.stack.top().is_falsey() {
                    *self.ip() += offset as usize;
                }
            }
            OpCode::JumpTruthy => {
                let offset = self.read_u16()?;
                if self.stack.top().is_truthy() {
                    *self.ip() += offset as usize;
                }
            }
            OpCode::JumpBack => {
                let offset = self.read_u16()?;
                *self.ip() -= offset as usize;
            }
            OpCode::Call => {
                let arg_count = self.read_byte()? as usize;
                match self.stack.peek(arg_count) {
                    Value::Closure(c) => {
                        let f = unsafe { c.as_ref().func };
                        let fun = unsafe { f.as_ref() };
                        if fun.arg_count != arg_count as u8 {
                            return Err(InterpretError::RuntimeError(format!(
                                "[cycle: {}] Function({}) expects {} args, got {}.",
                                self.clock, fun.name, fun.arg_count, arg_count
                            )));
                        }
                        if self.frame_count == MAX_FRAMES {
                            return Err(InterpretError::RuntimeError(format!(
                                "[cycle: {}] Stack overflow",
                                self.clock
                            )));
                        }

                        self.frames[self.frame_count] =
                            CallFrame::new(*c, self.stack.cursor - arg_count - 1);
                        self.frame_count += 1;

                        debug!("{}", fun.chunk.disassemble(fun.name));
                        // debug!("{}", Self::print_stack(&self.stack, self.sp(), false));
                        // return Ok(VMState::Running);
                    }
                    Value::NativeFn(func) => {
                        let result = func(
                            &self.stack.data[self.stack.cursor - arg_count..self.stack.cursor],
                        );
                        self.stack.cursor -= arg_count;
                        *self.stack.top_mut() = result;
                    }
                    x => {
                        return Err(InterpretError::RuntimeError(format!(
                            "[cycle: {}] Object '{x:?}' is not callable",
                            self.clock
                        )));
                    }
                }
            }
            OpCode::Closure => {
                let func = self.read_const()?;
                match func {
                    Value::Function(f) => {
                        if self.time_to_gc() {
                            self.collect_garbage();
                        }
                        let mut closure_ptr = Value::alloc_closure(f, &mut self.heap_objects);
                        let closure = Value::Closure(closure_ptr);
                        self.gc_stats.bytes_allocated += closure.size();

                        self.stack.push(closure)?;

                        for _ in 0..unsafe { f.as_ref().upval_count } {
                            let local = self.read_byte()? != 0;
                            let idx = self.read_byte()? as usize;

                            let upval = if local {
                                let val: NonNull<Value> =
                                    (&mut self.stack.data[self.sp() + idx]).into();
                                self.capture_upval(self.sp() + idx, val)
                            } else {
                                unsafe { self.current_frame().closure.as_ref().upvals[idx] }
                            };

                            unsafe { closure_ptr.as_mut().upvals.push(upval) };
                        }
                    }
                    x => {
                        return Err(InterpretError::RuntimeError(format!(
                            "[cycle: {}] Expected function, got {x:?}",
                            self.clock
                        )));
                    }
                }
            }
            OpCode::ReadUpval => {
                let slot = self.read_byte()? as usize;
                let val = unsafe { self.current_frame().closure.as_ref().upvals[slot] };

                match unsafe { val.as_ref() } {
                    UpVal::Open(v, _) => self.stack.push(unsafe { v.read() })?,
                    UpVal::Closed(v, _) => self.stack.push(*v)?,
                }
            }
            OpCode::WriteUpval => {
                let slot = self.read_byte()? as usize;
                match unsafe { self.current_frame().closure_mut().upvals[slot].as_mut() } {
                    UpVal::Open(v, _) => unsafe { v.write(*self.stack.peek(0)) },
                    UpVal::Closed(value, _) => *value = *self.stack.peek(0),
                }
            }
            OpCode::CloseUpVal => {
                // let val = *self.stack.peek(0);
                self.close_upval(self.stack.cursor - 1);
                self.stack.pop()?;
            }
            // all ops that require 2 operands
            _ => {
                let b = self.stack.pop()?;
                let top = self.stack.top_mut();

                match opcode {
                    OpCode::Add => {
                        if self.time_to_gc() {
                            self.collect_garbage();
                        }
                        let top = self.stack.top_mut();
                        top.add(&b, &mut self.strings, &mut self.heap_objects)?;
                        if matches!(top, Value::String(_)) {
                            self.gc_stats.bytes_allocated += top.size();
                        }
                    }
                    OpCode::Subtract => {
                        top.sub(&b)?;
                    }
                    OpCode::Multiply => {
                        top.mul(&b)?;
                    }
                    OpCode::Divide => {
                        top.div(&b)?;
                    }
                    OpCode::Eq => {
                        top.equal(&b);
                    }
                    OpCode::Neq => {
                        top.not_equal(&b);
                    }
                    OpCode::Gt => {
                        top.greater(&b)?;
                    }
                    OpCode::GtEq => {
                        top.greater_equal(&b)?;
                    }
                    OpCode::Lt => {
                        top.less(&b)?;
                    }
                    OpCode::LtEq => {
                        top.less_equal(&b)?;
                    }
                    _ => unreachable!(),
                }
            }
        }

        trace!(target: "Stack", "\n{}", indent::indent_all_by(9, Self::print_stack(&self.stack, self.sp(), true)));

        Ok(VMState::Running)
    }

    fn read_byte(&mut self) -> Result<u8, InterpretError> {
        let ip = *self.ip();
        let val = Ok(self.chunk().data.get(ip).copied().ok_or_else(|| {
            InterpretError::RuntimeError(format!("[cycle: {}] Constant data missing", self.clock))
        })?);

        *self.ip() += 1;

        val
    }

    fn read_u16(&mut self) -> Result<u16, InterpretError> {
        let ip = *self.ip();
        if self.chunk().data.len() <= ip + 1 {
            return Err(InterpretError::RuntimeError(format!(
                "[cycle: {}] Constant data missing",
                self.clock
            )));
        }

        let val = unsafe { Ok(self.chunk().data.as_ptr().byte_add(ip).cast::<u16>().read()) };

        *self.ip() += 2;

        val
    }

    fn read_const(&mut self) -> Result<Value, InterpretError> {
        let const_idx = self.read_byte()? as usize;

        Ok(self.chunk().constants[const_idx])
    }
    // fn read_const_16(&mut self) -> Result<Value, InterpretError> {
    //     let const_idx_lo = self.read_byte()? as usize;

    //     let const_idx_hi = self.read_byte()? as usize;

    //     let const_idx = (const_idx_hi << 8) | const_idx_lo;

    //     Ok(self.chunk().constants[const_idx])
    // }

    fn capture_upval(&mut self, slot: usize, val: NonNull<Value>) -> NonNull<UpVal> {
        match self.upvalues.get(&slot) {
            Some(v) => *v,
            None => {
                if self.time_to_gc() {
                    self.collect_garbage();
                }
                let upval = Value::alloc_upval(val, &mut self.heap_objects);
                self.gc_stats.bytes_allocated += Value::UpValue(upval).size();
                self.upvalues.insert(slot, upval);

                upval
            }
        }
    }

    fn close_upval(&mut self, last: usize) {
        let mut remove = Vec::new();
        for (loc, uv) in self.upvalues.iter().rev() {
            if *loc < last {
                break;
            }

            let val = match unsafe { uv.as_ref() } {
                UpVal::Open(v, _) => unsafe { v.read() },
                UpVal::Closed(_, _) => panic!("Closed upval in open upval list"),
            };

            unsafe { uv.write(UpVal::Closed(val, false)) };
            remove.push(*loc);
        }

        for r in remove {
            self.upvalues.remove(&r);
        }
    }

    pub fn reset_stack(&mut self) {
        self.stack.clear();
    }

    #[instrument(skip_all, level = Level::DEBUG)]
    pub fn collect_garbage(&mut self) {
        let before = self.gc_stats.bytes_allocated;
        debug!("bytes before: {before}");
        self.mark_roots();
        self.trace_references();

        for e in self.strings.entries.iter_mut() {
            if e.as_ref().is_some_and(|e| !e.key.is_marked()) {
                *e = None;
            }
        }

        self.sweep();
        self.gc_stats.next_gc = self.gc_stats.bytes_allocated * Self::GC_HEAP_GROW_FACTOR;

        let diff = before - self.gc_stats.bytes_allocated;
        debug!(
            "bytes after: {} (swept: {diff}, next: {})",
            self.gc_stats.bytes_allocated, self.gc_stats.next_gc
        );
    }

    pub fn mark_roots(&mut self) {
        for val in self.stack.data[0..self.stack.cursor].iter_mut() {
            if !val.is_marked() {
                val.mark();
                if val.has_child_allocs() {
                    self.grey_stack.push(*val);
                }
            }
        }

        for frame in self.frames[..self.frame_count].iter_mut() {
            let c = unsafe { frame.closure.as_mut() };
            if !c.marked {
                c.marked = true;
                // no need to check since closures always have children
                self.grey_stack.push(Value::Closure(frame.closure));
            }
        }

        for upval in self.upvalues.values_mut() {
            match unsafe { upval.as_mut() } {
                UpVal::Open(_, marked) | UpVal::Closed(_, marked) => {
                    if !*marked {
                        *marked = true;
                        self.grey_stack.push(Value::UpValue(*upval));
                    }
                }
            }
        }

        for entry in self.globals.entries.iter_mut().flatten() {
            if !entry.key.is_marked() {
                entry.key.mark();
                // no point adding strings to the grey stack since they're terminal nodes anyway
            }
            if !entry.val.is_marked() {
                entry.val.mark();
                if entry.val.has_child_allocs() {
                    self.grey_stack.push(entry.val);
                }
            }
        }
    }

    pub fn trace_references(&mut self) {
        while let Some(val) = self.grey_stack.pop() {
            match val {
                Value::Function(mut non_null) => {
                    for c in unsafe { non_null.as_mut().chunk.constants.iter_mut() } {
                        if !c.is_marked() {
                            c.mark();
                            if c.has_child_allocs() {
                                self.grey_stack.push(*c);
                            }
                        }
                    }
                }
                Value::Closure(mut non_null) => {
                    let clos = unsafe { non_null.as_mut() };
                    let mut func = Value::Function(clos.func);
                    if !func.is_marked() {
                        func.mark();
                        self.grey_stack.push(func);
                    }

                    for v in &clos.upvals {
                        let mut uv = Value::UpValue(*v);
                        if !uv.is_marked() {
                            uv.mark();
                            if uv.has_child_allocs() {
                                self.grey_stack.push(uv);
                            }
                        }
                    }
                }
                Value::UpValue(mut non_null) => {
                    if let UpVal::Closed(value, _) = unsafe { non_null.as_mut() }
                        && !value.is_marked()
                    {
                        value.mark();
                        if value.has_child_allocs() {
                            self.grey_stack.push(*value);
                        }
                    }
                }
                _ => (),
            }
        }
    }

    pub fn sweep(&mut self) {
        let mut i = 0;

        while i < self.heap_objects.len() {
            if self.heap_objects[i].is_marked() {
                self.heap_objects[i].unmark();
                i += 1;
                continue;
            }

            let val = self.heap_objects.swap_remove(i);
            self.gc_stats.bytes_allocated -= val.size();
            val.dealloc();
        }
    }

    pub fn time_to_gc(&self) -> bool {
        self.gc_stats.bytes_allocated > self.gc_stats.next_gc
    }

    pub fn print_stack(stack: &Stack<MAX_STACK>, sp: usize, full: bool) -> String {
        let mut output = "".to_owned();

        let top = stack.cursor;

        // writeln!(output, "Stack:").unwrap();

        let skip = if full { 0 } else { sp };

        for (i, v) in stack.data.iter().enumerate().skip(skip) {
            if i >= top {
                break;
            }
            let delim = match i.cmp(&sp) {
                Ordering::Less => "░",
                Ordering::Equal => "-",
                Ordering::Greater => "█",
            };

            writeln!(output, "{delim} [{i:03}]: {v}").unwrap();
        }

        output
    }

    pub fn print_stack_trace(&self) {
        for frame in self.frames[0..self.frame_count].iter() {
            let func = unsafe { frame.closure.as_ref().func.as_ref() };
            let name = if func.name.is_empty() {
                "script"
            } else {
                func.name
            };

            error!("[line {}] in {name}", func.chunk.line_for_offset(frame.ip));
        }
    }
}
