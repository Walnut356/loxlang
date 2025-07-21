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
        parser.expression();
        parser.consume(TokenKind::EOF, "Expect EOF");
        parser
            .chunk
            .push_opcode(OpCode::Return, parser.scanner.line);

        debug!("{}", parser.chunk.disassemble("chunk"));

        Ok(())
    }

    pub fn run(&mut self) -> Result<(), InterpretError> {
        let mut ip = self.chunk.as_ref().unwrap().data.iter().enumerate();
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
                    println!("return {:?}", self.stack.pop()?);
                    break;
                }
                OpCode::Constant => {
                    let const_idx = *ip
                        .next()
                        .ok_or_else(|| {
                            InterpretError::RuntimeError("Constant data missing".to_owned())
                        })?
                        .1 as usize;

                    let value = self.chunk.as_ref().unwrap().constants[const_idx];
                    self.stack.push(value).unwrap();
                    // println!("{value}");
                }
                OpCode::Constant16 => {
                    let const_idx_lo = *ip
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

                    let value = self.chunk.as_ref().unwrap().constants[const_idx];
                    self.stack.push(value).unwrap();
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
                // all ops that require 2 operands
                _ => {
                    let b = self.stack.pop()?;
                    let top = self.stack.top_mut();

                    match opcode {
                        OpCode::Add => {
                            top.add(&b)?;
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
