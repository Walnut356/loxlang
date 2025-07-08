use crate::{
    chunk::{Chunk, OpCode},
    stack::Stack,
    value::Value,
};
use log::{Level, debug, log_enabled};
use std::fmt::Write;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum InterpretError {
    #[error("{0}")]
    CompileError(String),
    #[error("{0}")]
    RuntimeError(String),
}

#[derive(Default)]
pub struct VM<'a> {
    chunk: Option<&'a Chunk>,
    stack: Stack<256>,
}

impl<'a> VM<'a> {
    // pub fn new() -> Self {
    //     Self { chunk: None }
    // }

    pub fn interpret(&mut self, chunk: &'a Chunk) -> Result<(), InterpretError> {
        self.chunk = Some(chunk);
        self.run()
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
                debug!("cycle {cycles}:\n{disasm_out}");
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
                Some(OpCode::Negate) => {
                    self.stack.top_mut().negate();
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
