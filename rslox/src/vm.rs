use crate::{
    chunk::{Chunk, OpCode},
    compiler::Parser,
    scanner::{Scanner, Token, TokenKind},
    stack::Stack,
    table::Table,
    value::{Function, Value},
};
use log::{debug, error, log_enabled, trace, Level};
use std::{fmt::Write, rc::Rc};
use thiserror::Error;

const MAX_FRAMES: usize = 64;
const MAX_STACK: usize = MAX_FRAMES * u8::MAX as usize;

#[derive(Error, Debug)]
pub enum InterpretError {
    #[error("{0}")]
    CompileError(String),
    #[error("{0}")]
    RuntimeError(String),
}

#[derive(Debug, Default)]
pub struct CallFrame {
    func: *mut Function,
    ip: usize,
    sp: usize,
}

impl CallFrame {
    pub fn new(func: *mut Function, sp: usize) -> Self {
        Self {
            func,
            ip: 0,
            sp,
        }
    }
}



pub struct VM {
    // chunk: Option<Chunk>,
    heap_objects: Vec<Value>,
    strings: Table,
    globals: Table,
    frame_count: usize,
    frames: [CallFrame; MAX_FRAMES],
    stack: Stack<MAX_STACK>,
}

impl Default for VM {
    fn default() -> Self {
        Self {
            // chunk: Default::default(),
            heap_objects: Default::default(),
            strings: Default::default(),
            globals: Default::default(),
            frame_count: Default::default(),
            frames: std::array::from_fn(|_| CallFrame::default()),
            stack: Default::default(),
        }
    }
}

impl VM {
    pub fn interpret(&mut self, source: Rc<str>) -> Result<(), InterpretError> {
        let func = self.compile(source)?;


        self.frames[self.frame_count] = CallFrame::new(func, self.stack.cursor);
        self.frame_count += 1;

        self.stack.push(Value::Function(func))?;

        for val in unsafe { &func.as_ref().unwrap().chunk.constants } {
            match val {
                Value::String(s) => {
                    self.strings.insert(s, Value::Bool(true));
                }
                Value::Object(_) => todo!(),
                _ => (),
            }
        }

        let res = self.run();

        if res.is_err() {
            self.print_stack_trace();
        }

        res
    }

    pub fn compile(&mut self, source: Rc<str>) -> Result<*mut Function, InterpretError> {
        let mut parser = Parser::new(source, &mut self.strings);

        while !parser.eof() {
            parser.declaration();
        }

        if parser.errors {
            return Err(InterpretError::CompileError("".to_owned()));
        }

        debug!("{}", parser.compiler.func.chunk.disassemble(parser.compiler.func.name));

        Ok(parser.compiler.func as *mut _)
    }

    fn current_frame(&mut self) -> &mut CallFrame {
        &mut self.frames[self.frame_count - 1]
    }

    fn frame_ref(&self) -> &CallFrame {
        &self.frames[self.frame_count - 1]
    }

    fn chunk(&mut self) -> &Chunk {
        unsafe { &self.current_frame().func.as_ref().unwrap().chunk }
    }

    fn ip(&mut self) -> &mut usize {
        &mut self.current_frame().ip
    }

    fn ip_copied(&self) -> usize {
        self.frames[self.frame_count - 1].ip
    }

    fn sp(&self) -> usize {
        self.frames[self.frame_count - 1].sp
    }


    pub fn run(&mut self) -> Result<(), InterpretError> {
        // let frame = &mut self.frames[self.frame_count - 1];
        // let ip = &mut frame.ip;
        // let chunk = unsafe { &frame.func.as_ref().unwrap().chunk };


        let mut disasm_out = String::new();
        let mut cycles: usize = 0;

        while let ip_copy = self.ip_copied() && let Some(&op) = self.chunk().data.get(ip_copy) {
            cycles += 1;
            if log_enabled!(Level::Debug) {
                self.chunk()
                    .disassemble_instr(&mut disasm_out, ip_copy);
                trace!("cycle {cycles}:\n{disasm_out}");
                disasm_out.clear();
            }

            *self.ip() += 1;

            let Some(opcode) = OpCode::from_repr(op) else {
                return Err(InterpretError::RuntimeError(format!("Invalid Opcode {op}")));
            };

            match opcode {
                OpCode::Return => {
                    let result = self.stack.pop()?;
                    self.frame_count -= 1;

                    if self.frame_count == 0 {
                        self.stack.pop()?;
                        return Ok(());
                    }

                    self.stack.cursor = self.sp();
                    self.stack.push(result)?;
                }
                OpCode::Constant => {
                    let value = self.read_const()?;
                    self.stack.push(value).unwrap();
                }
                OpCode::Constant16 => {
                    let value = self.read_const_16()?;
                    self.stack.push(value).unwrap();
                }
                OpCode::DefGlobal => {
                    let name = self.read_const()?;
                    let Value::String(n) = name else {
                        return Err(InterpretError::RuntimeError(format!(
                            "Invalid type for global name. Expected string, got {name:?}"
                        )));
                    };

                    self.globals.insert(n, *self.stack.top());

                    self.stack.pop()?;
                }
                OpCode::DefGlobal16 => {
                    let name = self.read_const_16()?;
                    let n = name.try_as_string().unwrap();

                    self.globals.insert(n, *self.stack.top());

                    self.stack.pop()?;
                }
                OpCode::ReadGlobal => {
                    let name = self.read_const()?;
                    let n = name.try_as_string().unwrap();

                    match self.globals.get(n) {
                        Some(x) => self.stack.push(*x)?,
                        None => {
                            return Err(InterpretError::RuntimeError(format!(
                                "Undefined variable {n:?}"
                            )));
                        }
                    }
                }
                OpCode::ReadGlobal16 => {
                    let name = self.read_const_16()?;
                    let n = name.try_as_string().unwrap();

                    match self.globals.get(n) {
                        Some(x) => self.stack.push(*x)?,
                        None => {
                            return Err(InterpretError::RuntimeError(format!(
                                "Undefined variable {n:?}"
                            )));
                        }
                    }
                }
                OpCode::WriteGlobal => {
                    let name = self.read_const()?;

                    let n = name.try_as_string().unwrap();

                    if self.globals.insert(n, *self.stack.top()) {
                        self.globals.remove(n);
                        return Err(InterpretError::RuntimeError(format!(
                            "Undefined variable {n:?}"
                        )));
                    }
                }
                OpCode::WriteGlobal16 => {
                    let name = self.read_const_16()?;

                    let n = name.try_as_string().unwrap();

                    if self.globals.insert(n, *self.stack.top()) {
                        self.globals.remove(n);
                        return Err(InterpretError::RuntimeError(format!(
                            "Undefined variable {n:?}"
                        )));
                    }
                }
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
                    match self.stack.data[self.stack.cursor - 1 - arg_count] {
                        Value::Function(f) => {
                            let fun = unsafe {f.as_ref().unwrap()};
                            if fun.arg_count != arg_count as u8 {
                                return Err(InterpretError::RuntimeError(format!("{} expects {} args, got {}.", self.stack.data[self.stack.cursor - 1 - arg_count], fun.arg_count, arg_count)))
                            }
                            if self.frame_count == MAX_FRAMES {
                                return Err(InterpretError::RuntimeError("Stack overflow".to_owned()));
                            }

                            self.frames[self.frame_count] = CallFrame::new(f, (self.stack.cursor - 1) - arg_count);
                            self.frame_count += 1;

                            debug!("{}", fun.chunk.disassemble(fun.name));
                            debug!("{}", Self::print_stack(&self.stack, self.sp()));
                            continue;
                        }
                        x => return Err(InterpretError::RuntimeError(format!("Object '{x:?}' is not callable")))
                    }
                }
                // all ops that require 2 operands
                _ => {
                    let b = self.stack.pop()?;
                    let top = self.stack.top_mut();

                    match opcode {
                        OpCode::Add => {
                            top.add(&b, &mut self.strings)?;
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
                            top.equal(&b);
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

            trace!("{}", Self::print_stack(&self.stack, self.frame_ref().sp));
        }

        Ok(())
    }

    fn read_byte(&mut self) -> Result<u8, InterpretError> {
        let ip = *self.ip();
        let val = Ok(self.chunk()
            .data
            .get(ip)
            .copied()
            .ok_or_else(|| InterpretError::RuntimeError("Constant data missing".to_owned()))?);

        *self.ip() += 1;

        val
    }

    fn read_u16(&mut self) -> Result<u16, InterpretError> {
        let ip = *self.ip();
        if self.chunk().data.len() <= ip + 1 {
            return Err(InterpretError::RuntimeError(
                "Constant data missing".to_owned(),
            ));
        }

        let val = unsafe { Ok(self.chunk().data.as_ptr().byte_add(ip).cast::<u16>().read()) };

        *self.ip() += 2;

        val
    }

    fn read_const(&mut self) -> Result<Value, InterpretError> {
        let const_idx = self.read_byte()? as usize;

        Ok(self.chunk().constants[const_idx])
    }

    fn read_const_16(&mut self) -> Result<Value, InterpretError> {
        let const_idx_lo = self.read_byte()? as usize;

        let const_idx_hi = self.read_byte()? as usize;

        let const_idx = (const_idx_hi << 8) | const_idx_lo;

        Ok(self.chunk().constants[const_idx])
    }

    pub fn reset_stack(&mut self) {
        self.stack.clear();
    }

    pub fn print_stack(stack: &Stack<MAX_STACK>, sp: usize) -> String {
        let mut output = "".to_owned();

        let top = stack.cursor;

        writeln!(output, "   Stack:").unwrap();

        for (i, v) in stack.data.iter().enumerate().skip(sp) {
            if i >= top {
                break;
            }
            writeln!(output, "   # [{i:03}]: {v}").unwrap();
        }

        output
    }

    pub fn print_stack_trace(&self) {
        for frame in self.frames[0..self.frame_count].iter() {
            let func = unsafe { frame.func.as_ref().unwrap() };
            let name = if func.name.is_empty() {
                    "script"
            } else {
                func.name
            };


            error!("[line {}] in {name}", func.chunk.line_for_offset(frame.ip));
        }
    }
}
