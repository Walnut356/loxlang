use crate::{
    chunk::{Chunk, OpCode},
    compiler::Parser,
    scanner::{Scanner, Token, TokenKind},
    stack::Stack,
    table::Table,
    value::{Function, Value},
};
use log::{Level, debug, log_enabled, trace};
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

        self.stack.push(Value::Function(func))?;

        self.frames[self.frame_count] = CallFrame::new(func, self.stack.cursor);
        self.frame_count += 1;


        for val in unsafe { &func.as_ref().unwrap().chunk.constants } {
            match val {
                Value::String(s) => {
                    self.strings.insert(s, Value::Bool(true));
                }
                Value::Object(_) => todo!(),
                _ => (),
            }
        }

        self.run()
    }

    pub fn compile(&mut self, source: Rc<str>) -> Result<*mut Function, InterpretError> {
        let mut parser = Parser::new(source, &mut self.strings);

        while !parser.eof() {
            parser.declaration();
        }

        if parser.errors {
            return Err(InterpretError::CompileError("".to_owned()));
        }

        debug!("{}", parser.compiler.func.chunk.disassemble("chunk"));

        Ok(parser.compiler.func as *mut _)
    }

    pub fn run(&mut self) -> Result<(), InterpretError> {
        let frame = &mut self.frames[self.frame_count - 1];
        let ip = &mut frame.ip;
        let chunk = unsafe { &frame.func.as_ref().unwrap().chunk };


        let mut disasm_out = String::new();
        let mut cycles: usize = 0;


        while let Some(&op) = chunk.data.get(*ip) {
            cycles += 1;
            if log_enabled!(Level::Debug) {
                chunk
                    .disassemble_instr(&mut disasm_out, *ip);
                trace!("cycle {cycles}:\n{disasm_out}");
                disasm_out.clear();
            }

            *ip += 1;

            let Some(opcode) = OpCode::from_repr(op) else {
                return Err(InterpretError::RuntimeError(format!("Invalid Opcode {op}")));
            };

            match opcode {
                OpCode::Return => {
                    return Ok(());
                }
                OpCode::Constant => {
                    let value = Self::read_const(chunk, ip)?;
                    self.stack.push(value).unwrap();
                }
                OpCode::Constant16 => {
                    let value = Self::read_const_16(chunk, ip)?;
                    self.stack.push(value).unwrap();
                }
                OpCode::DefGlobal => {
                    let name = Self::read_const(chunk, ip)?;
                    let Value::String(n) = name else {
                        return Err(InterpretError::RuntimeError(format!(
                            "Invalid type for global name. Expected string, got {name:?}"
                        )));
                    };

                    self.globals.insert(n, *self.stack.top());

                    self.stack.pop()?;
                }
                OpCode::DefGlobal16 => {
                    let name = Self::read_const_16(chunk, ip)?;
                    let n = name.try_as_string().unwrap();

                    self.globals.insert(n, *self.stack.top());

                    self.stack.pop()?;
                }
                OpCode::ReadGlobal => {
                    let name = Self::read_const(chunk, ip)?;
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
                    let name = Self::read_const_16(chunk, ip)?;
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
                    let name = Self::read_const(chunk, ip)?;

                    let n = name.try_as_string().unwrap();

                    if self.globals.insert(n, *self.stack.top()) {
                        self.globals.remove(n);
                        return Err(InterpretError::RuntimeError(format!(
                            "Undefined variable {n:?}"
                        )));
                    }
                }
                OpCode::WriteGlobal16 => {
                    let name = Self::read_const_16(chunk, ip)?;

                    let n = name.try_as_string().unwrap();

                    if self.globals.insert(n, *self.stack.top()) {
                        self.globals.remove(n);
                        return Err(InterpretError::RuntimeError(format!(
                            "Undefined variable {n:?}"
                        )));
                    }
                }
                OpCode::ReadLocal => {
                    let slot = Self::read_byte(chunk, ip)? as usize;
                    self.stack.push(self.stack.data[frame.sp + slot])?;
                }
                OpCode::WriteLocal => {
                    let slot = Self::read_byte(chunk, ip)? as usize;
                    self.stack.data[frame.sp + slot] = *self.stack.top();
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
                    self.stack.cursor -= Self::read_byte(chunk, ip)? as usize;
                }
                OpCode::Jump => {
                    let offset = Self::read_u16(chunk, ip)?;
                    *ip += offset as usize;
                }
                OpCode::JumpFalsey => {
                    let offset = Self::read_u16(chunk, ip)?;
                    if self.stack.top().is_falsey() {
                        *ip += offset as usize;
                    }
                }
                OpCode::JumpTruthy => {
                    let offset = Self::read_u16(chunk, ip)?;
                    if self.stack.top().is_truthy() {
                        *ip += offset as usize;
                    }
                }
                OpCode::JumpBack => {
                    let offset = Self::read_u16(chunk, ip)?;
                    *ip -= offset as usize;
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

            trace!("{}", Self::trace_stack(&self.stack, frame.sp));
        }

        Ok(())
    }

    fn read_byte(chunk: &Chunk, ip: &mut usize) -> Result<u8, InterpretError> {
        let val = Ok(chunk
            .data
            .get(*ip)
            .copied()
            .ok_or_else(|| InterpretError::RuntimeError("Constant data missing".to_owned()))?);

        *ip += 1;

        val
    }

    fn read_u16(chunk: &Chunk, ip: &mut usize) -> Result<u16, InterpretError> {
        if chunk.data.len() <= *ip + 1 {
            return Err(InterpretError::RuntimeError(
                "Constant data missing".to_owned(),
            ));
        }

        let val = unsafe { Ok(chunk.data.as_ptr().byte_add(*ip).cast::<u16>().read()) };

        *ip += 2;

        val
    }

    fn read_const(chunk: &Chunk, ip: &mut usize) -> Result<Value, InterpretError> {
        let const_idx = Self::read_byte(chunk, ip)? as usize;

        Ok(chunk.constants[const_idx])
    }

    fn read_const_16(chunk: &Chunk, ip: &mut usize) -> Result<Value, InterpretError> {
        let const_idx_lo = Self::read_byte(chunk, ip)? as usize;

        let const_idx_hi = Self::read_byte(chunk, ip)? as usize;

        let const_idx = (const_idx_hi << 8) | const_idx_lo;

        Ok(chunk.constants[const_idx])
    }

    pub fn reset_stack(&mut self) {
        self.stack.clear();
    }

    pub fn trace_stack(stack: &Stack<MAX_STACK>, sp: usize) -> String {
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
}
