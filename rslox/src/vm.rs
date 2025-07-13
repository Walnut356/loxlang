use crate::{
    chunk::{Chunk, OpCode}, compiler::Parser, scanner::{Scanner, Token, TokenKind}, stack::Stack, value::Value
};
use log::{debug, log_enabled, trace, Level};
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
}

impl VM {
    pub fn interpret(&mut self, source: Rc<str>) -> Result<(), InterpretError> {
        self.compile(source)?;

        self.run()
    }

    pub fn compile(&mut self, source: Rc<str>) -> Result<(), InterpretError> {
        if self.chunk.is_none() {
            self.chunk = Some(Chunk::default());
        } else {
            self.chunk.as_mut().unwrap().reset();
        }

        let mut parser = Parser::new(source, self.chunk.as_mut().unwrap());
        parser.expression();
        parser.consume(TokenKind::EOF, "Expect EOF");
        parser.chunk.push_opcode(OpCode::Return, parser.scanner.line);

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
            match OpCode::from_repr(op) {
                Some(OpCode::Constant) => {
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
                Some(OpCode::Constant16) => {
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

                    let const_idx = (const_idx_hi << 8)  | const_idx_lo;

                    let value = self.chunk.as_ref().unwrap().constants[const_idx];
                    self.stack.push(value).unwrap();
                }
                Some(OpCode::Negate) => {
                    self.stack.top_mut().negate()?;
                }

                Some(OpCode::Add) => {
                    let b = self.stack.pop()?;
                    self.stack.top_mut().add(&b);
                }
                Some(OpCode::Subtract) => {
                    let b = self.stack.pop()?;
                    self.stack.top_mut().sub(&b);
                }
                Some(OpCode::Multiply) => {
                    let b = self.stack.pop()?;
                    self.stack.top_mut().mul(&b);
                }
                Some(OpCode::Divide) => {
                    let b = self.stack.pop()?;
                    self.stack.top_mut().div(&b);
                }

                Some(OpCode::Return) => {
                    println!("return {:?}", self.stack.pop()?);
                    break;
                }
                None => return Err(InterpretError::RuntimeError(format!("Invalid Opcode {op}"))),
            }
        }

        Ok(())
    }

    pub fn reset_stack(&mut self) {
        self.stack.clear();
    }

    pub fn trace_stack(&self) -> String {
        let mut output = "".to_owned();

        for (i, v) in self.stack.data.iter().enumerate() {
            writeln!(output, "[{i}]: {v:?}").unwrap();
        }

        output
    }
}
