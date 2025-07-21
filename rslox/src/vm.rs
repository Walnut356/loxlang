use crate::{
    chunk::{Chunk, OpCode},
    compiler::Parser,
    scanner::{Scanner, Token, TokenKind},
    stack::Stack,
    table::Table,
    value::Value,
};
use log::{Level, debug, log_enabled, trace};
use std::{fmt::Write, rc::Rc};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum InterpretError {
    #[error("{0}")]
    CompileError(String),
    #[error("{0}")]
    RuntimeError(String),
}

#[derive(Default)]
pub struct VM {
    chunk: Option<Chunk>,
    stack: Stack<256>,
    heap_objects: Vec<Value>,
    strings: Table,
    globals: Table,
}

impl VM {
    pub fn interpret(&mut self, source: Rc<str>) -> Result<(), InterpretError> {
        self.compile(source)?;

        for val in &self.chunk.as_ref().unwrap().constants {
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

    pub fn compile(&mut self, source: Rc<str>) -> Result<(), InterpretError> {
        if self.chunk.is_none() {
            self.chunk = Some(Chunk::default());
        } else {
            self.chunk.as_mut().unwrap().reset();
        }

        let mut parser = Parser::new(source, self.chunk.as_mut().unwrap(), &mut self.strings);

        while !parser.eof() {
            parser.declaration();
        }

        if parser.errors {
            return Err(InterpretError::CompileError("".to_owned()));
        }
        /*         parser.expression();
        parser.consume(TokenKind::EOF, "Expect EOF");
        parser
            .chunk
            .push_opcode(OpCode::Return, parser.scanner.line); */

        debug!("{}", parser.chunk.disassemble("chunk"));

        Ok(())
    }

    pub fn run(&mut self) -> Result<(), InterpretError> {
        let mut ip: std::iter::Enumerate<std::slice::Iter<'_, u8>> =
            self.chunk.as_ref().unwrap().data.iter().enumerate();
        let mut disasm_out = String::new();
        let mut cycles: usize = 0;

        while let Some((idx, &op)) = ip.next() {
            cycles += 1;
            if log_enabled!(Level::Debug) {
                self.chunk
                    .as_ref()
                    .unwrap()
                    .disassemble_instr(&mut disasm_out, idx);
                trace!("cycle {cycles}:\n{disasm_out}");
                disasm_out.clear();
            }

            let Some(opcode) = OpCode::from_repr(op) else {
                return Err(InterpretError::RuntimeError(format!("Invalid Opcode {op}")));
            };

            match opcode {
                OpCode::Return => {
                    return Ok(());
                }
                OpCode::Constant => {
                    /* let const_idx = *ip
                        .next()
                        .ok_or_else(|| {
                            InterpretError::RuntimeError("Constant data missing".to_owned())
                        })?
                        .1 as usize;

                    let value = self.chunk.as_ref().unwrap().constants[const_idx]; */

                    let value = Self::read_const(self.chunk.as_ref().unwrap(), &mut ip)?;
                    self.stack.push(value).unwrap();
                }
                OpCode::Constant16 => {
                    /* let const_idx_lo = *ip
                        .next()
                        .ok_or_else(|| {
                            InterpretError::RuntimeError("Constant data missing".to_owned())
                        })?
                        .1 as usize;

                    let const_idx_hi = *ip
                        .next()
                        .ok_or_else(|| {
                            InterpretError::RuntimeError("Constant data missing".to_owned())
                        })?
                        .1 as usize;

                    let const_idx = (const_idx_hi << 8) | const_idx_lo;

                    let value = self.chunk.as_ref().unwrap().constants[const_idx]; */

                    let value = Self::read_const_16(self.chunk.as_ref().unwrap(), &mut ip)?;
                    self.stack.push(value).unwrap();
                }
                OpCode::DefGlobal => {
                    let name = Self::read_const(self.chunk.as_ref().unwrap(), &mut ip)?;
                    let n = name.try_as_string().unwrap();

                    self.globals
                        .insert(n, *self.stack.top());

                    self.stack.pop()?;
                }
                OpCode::DefGlobal16 => {
                    let name = Self::read_const_16(self.chunk.as_ref().unwrap(), &mut ip)?;
                    let n = name.try_as_string().unwrap();

                    self.globals
                        .insert(n, *self.stack.top());

                    self.stack.pop()?;
                }
                OpCode::ReadGlobal => {
                    let name = Self::read_const(self.chunk.as_ref().unwrap(), &mut ip)?;
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
                    let name = Self::read_const_16(self.chunk.as_ref().unwrap(), &mut ip)?;
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
                    let name = Self::read_const(self.chunk.as_ref().unwrap(), &mut ip)?;

                    let n = name.try_as_string().unwrap();

                    if self.globals.insert(n, *self.stack.top()) {
                        self.globals.remove(n);
                        return Err(InterpretError::RuntimeError(format!(
                            "Undefined variable {n:?}"
                        )));
                    }
                }
                OpCode::WriteGlobal16 => {
                    let name = Self::read_const_16(self.chunk.as_ref().unwrap(), &mut ip)?;

                    let n = name.try_as_string().unwrap();

                    if self.globals.insert(n, *self.stack.top()) {
                        self.globals.remove(n);
                        return Err(InterpretError::RuntimeError(format!(
                            "Undefined variable {n:?}"
                        )));
                    }
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

            trace!("{}", self.trace_stack());
        }

        Ok(())
    }

    fn read_const(
        chunk: &Chunk,
        ip: &mut std::iter::Enumerate<std::slice::Iter<'_, u8>>,
    ) -> Result<Value, InterpretError> {
        let const_idx = *ip
            .next()
            .ok_or_else(|| InterpretError::RuntimeError("Constant data missing".to_owned()))?
            .1 as usize;

        Ok(chunk.constants[const_idx])
    }

    fn read_const_16(
        chunk: &Chunk,
        ip: &mut std::iter::Enumerate<std::slice::Iter<'_, u8>>,
    ) -> Result<Value, InterpretError> {
        let const_idx_lo = *ip
            .next()
            .ok_or_else(|| InterpretError::RuntimeError("Constant data missing".to_owned()))?
            .1 as usize;

        let const_idx_hi = *ip
            .next()
            .ok_or_else(|| InterpretError::RuntimeError("Constant data missing".to_owned()))?
            .1 as usize;

        let const_idx = (const_idx_hi << 8) | const_idx_lo;

        Ok(chunk.constants[const_idx])
    }

    pub fn reset_stack(&mut self) {
        self.stack.clear();
    }

    pub fn trace_stack(&self) -> String {
        let mut output = "".to_owned();

        let top = self.stack.cursor;

        writeln!(output, "   Stack:").unwrap();

        for (i, v) in self.stack.data.iter().enumerate() {
            if i >= top {
                break;
            }
            writeln!(output, "   # [{i:03}]: {v}").unwrap();
        }

        output
    }
}
