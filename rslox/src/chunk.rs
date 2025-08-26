use crate::value::Value;
use strum::VariantNames;
use strum_macros::*;
// use std::io::Write;
use std::{fmt::Write, rc::Rc};

/// SAFETY: opcodes with 16-bit operands must have a discr 1 greater than their 8-bit counterpart
/// (e.g. `Constant as u8 == 1`, `Constant16 as u8 == 2`)
#[derive(Debug, FromRepr, VariantNames)]
#[repr(u8)]
pub enum OpCode {
    Return,
    Constant,
    Constant16,
    DefGlobal,
    DefGlobal16,
    ReadGlobal,
    ReadGlobal16,
    WriteGlobal,
    WriteGlobal16,
    // no 16 bit variants for Read/Write local
    ReadLocal,
    WriteLocal,
    ReadUpval,
    WriteUpval,

    Negate,
    Add,
    Subtract,
    Multiply,
    Divide,
    Nil,
    False,
    True,
    Not,
    Eq,
    Neq,
    Gt,
    GtEq,
    Lt,
    LtEq,
    Print,
    Pop,
    // Pops N
    StackSub,
    Jump,
    JumpFalsey,
    JumpTruthy,
    JumpBack,
    Call,
    Closure,
    CloseUpVal,
}

impl OpCode {
    /// Returns the byte-size of the opcode + its operand
    pub fn total_size(&self) -> usize {
        match self {
            OpCode::Constant
            | OpCode::ReadLocal
            | OpCode::WriteLocal
            | OpCode::DefGlobal
            | OpCode::ReadGlobal
            | OpCode::WriteGlobal
            | OpCode::StackSub
            | OpCode::Call
            | OpCode::ReadUpval
            | OpCode::WriteUpval => 2,
            OpCode::Constant16
            | OpCode::DefGlobal16
            | OpCode::ReadGlobal16
            | OpCode::WriteGlobal16
            | OpCode::Jump
            | OpCode::JumpFalsey
            | OpCode::JumpTruthy
            | OpCode::JumpBack => 3,
            OpCode::Return
            | OpCode::Negate
            | OpCode::Add
            | OpCode::Subtract
            | OpCode::Multiply
            | OpCode::Divide
            | OpCode::Nil
            | OpCode::False
            | OpCode::True
            | OpCode::Not
            | OpCode::Eq
            | OpCode::Neq
            | OpCode::Gt
            | OpCode::GtEq
            | OpCode::Lt
            | OpCode::LtEq
            | OpCode::Print
            | OpCode::Pop
            | OpCode::CloseUpVal => 1,
            // variable sized
            OpCode::Closure => usize::MAX,
        }
    }
}

/// Run-length encoded line number
#[derive(Debug, Default)]
pub struct LineRun {
    pub line: u32,
    pub len: u32,
}

#[derive(Debug, Default)]
pub struct Chunk {
    pub data: Vec<u8>,
    pub constants: Vec<Value>,
    pub lines: Vec<LineRun>,
    pub(crate) source: Rc<str>,
}

impl Chunk {
    pub fn reset(&mut self) {
        self.data.clear();
        self.constants.clear();
        self.lines.clear();
    }

    pub fn line_for_offset(&self, offset: usize) -> u32 {
        let mut curr = 0;
        for l in &self.lines {
            curr += l.len;
            if curr as usize > offset {
                return l.line;
            }
        }

        self.lines.last().map(|x| x.line).unwrap_or_default()
    }

    pub fn disassemble(&self, name: &str) -> String {
        let mut output = String::new();
        writeln!(output, "-- {name} --").unwrap();

        let mut offset = 0;
        while offset < self.data.len() {
            offset = self.disassemble_instr(&mut output, offset);
        }

        output
    }

    pub fn disassemble_instr(&self, output: &mut String, offset: usize) -> usize {
        let line_num = self.line_for_offset(offset);
        if offset == 0 || (self.line_for_offset(offset - 1) != line_num) {
            writeln!(output, "Line {line_num}:").unwrap();
        }
        write!(output, " | {offset:04x} ").unwrap();

        let opcode = self.data[offset];
        let op = OpCode::from_repr(opcode);
        match op {
            Some(OpCode::Jump | OpCode::JumpBack | OpCode::JumpFalsey | OpCode::JumpTruthy) => {
                let idx = unsafe { self.data.as_ptr().byte_add(offset + 1).cast::<u16>().read() }
                    as usize;

                let jmp = if opcode == OpCode::JumpBack as u8 {
                    offset + 3 - idx
                } else {
                    offset + 3 + idx
                };

                writeln!(output, "{}: {:04x}", OpCode::VARIANTS[opcode as usize], jmp).unwrap();
            }
            Some(OpCode::StackSub)
            | Some(OpCode::ReadLocal)
            | Some(OpCode::WriteLocal)
            | Some(OpCode::ReadUpval)
            | Some(OpCode::WriteUpval) => {
                let idx = self.data[offset + 1] as usize;
                writeln!(output, "{}: {idx:03}", OpCode::VARIANTS[opcode as usize]).unwrap();
            }
            Some(OpCode::Constant)
            | Some(OpCode::DefGlobal)
            | Some(OpCode::ReadGlobal)
            | Some(OpCode::WriteGlobal) => {
                let idx = self.data[offset + 1] as usize;
                writeln!(
                    output,
                    "{}: ({idx:03}) {}",
                    OpCode::VARIANTS[opcode as usize],
                    self.constants[idx]
                )
                .unwrap();
            }
            Some(OpCode::Constant16)
            | Some(OpCode::DefGlobal16)
            | Some(OpCode::ReadGlobal16)
            | Some(OpCode::WriteGlobal16) => {
                let idx = unsafe { self.data.as_ptr().byte_add(offset + 1).cast::<u16>().read() }
                    as usize;

                if idx < self.constants.len() {
                    writeln!(
                        output,
                        "{}: ({idx:05}) {}",
                        OpCode::VARIANTS[opcode as usize],
                        self.constants[idx]
                    )
                    .unwrap();
                } else {
                    writeln!(output, "<error reading opcode>").unwrap()
                }
            }
            Some(OpCode::Call) => {
                writeln!(output, "Call ({} args)", self.data[offset + 1]).unwrap();
            }
            Some(OpCode::Closure) => {
                let func = unsafe {
                    self.constants[self.data[offset + 1] as usize]
                        .try_as_function()
                        .unwrap()
                        .as_ref()
                };
                writeln!(output, "Closure({func})").unwrap();

                let mut res = offset + 2;
                for _ in 0..func.upval_count {
                    let kind = if self.data[res] == 0 {
                        "upval"
                    } else {
                        "local"
                    };
                    writeln!(output, " | {res:04x} | {kind} {}", self.data[res + 1]).unwrap();

                    res += 2;
                }

                return res;
            }
            Some(_) => {
                writeln!(output, "{}", OpCode::VARIANTS[opcode as usize]).unwrap();
            }
            None => {
                writeln!(output, "Unknown opcode: {opcode}").unwrap();
                return offset + 1;
            }
        }

        op.unwrap().total_size() + offset
    }

    pub fn push_opcode(&mut self, code: OpCode, line: u32) {
        self.data.push(code as u8);

        // absolutely gorgeous
        match self.lines.last_mut() {
            Some(l) if l.line == line => l.len += 1,
            _ => self.lines.push(LineRun { line, len: 1 }),
        }
    }

    /// assumed to occur on the same line as the the previous instruction
    pub fn push_bytes(&mut self, bytes: &[u8]) {
        self.data.extend(bytes);

        match self.lines.last_mut() {
            Some(l) => l.len += bytes.len() as u32,
            _ => panic!("push byte with no prior opcode"),
        }
    }

    /// Adds a constant to the constant table. Repeat constants are only stored once
    pub fn push_constant(&mut self, value: Value) -> u16 {
        if let Some(i) = self.constants.iter().position(|x| *x == value) {
            i as u16
        } else {
            self.constants.push(value);
            (self.constants.len() - 1) as u16
        }
    }

    /// Adds a constant to the constant table, then pushes an OpCode::Constant/OpCode::Constant16
    /// to the bytecode that reads the newly inserted constant
    pub fn insert_constant(&mut self, value: Value, line: u32) -> u16 {
        let idx = self.push_constant(value).to_ne_bytes();
        if idx[1] != 0 {
            self.push_opcode(OpCode::Constant16, line);
            self.push_bytes(&idx);
        } else {
            self.push_opcode(OpCode::Constant, line);
            self.push_bytes(&idx[0..1]);
        }

        u16::from_ne_bytes(idx)
    }

    pub fn push_jump(&mut self, opcode: OpCode, line: u32) -> usize {
        self.push_opcode(opcode, line);
        self.push_bytes(&u16::MAX.to_ne_bytes());
        self.data.len() - 2
    }

    pub fn push_loop(&mut self, idx: usize, line: u32) {
        self.push_opcode(OpCode::JumpBack, line);

        let offset = self.data.len() - idx + 2;
        if offset > u16::MAX as usize {
            // fix this some day
            panic!();
        }

        self.push_bytes(&(offset as u16).to_ne_bytes());
    }

    pub fn push_return(&mut self, line: u32) {
        self.push_opcode(OpCode::Nil, line);
        self.push_opcode(OpCode::Return, line);
    }
}

#[cfg(test)]
mod tests {
    use crate::chunk::*;

    #[test]
    fn disasm_ret() {
        let mut chunk = Chunk::default();
        chunk.push_opcode(OpCode::Return, 0);
        assert_eq!(
            chunk.disassemble("test"),
            "-- test --\n\tLine 0:\n0000 Return\n"
        );
    }

    #[test]
    fn disasm_const() {
        let mut chunk = Chunk::default();
        chunk.insert_constant(Value::Float(10.0), 0);

        assert_eq!(
            chunk.disassemble("test"),
            "-- test --\n\tLine 0:\n0000 Constant: (000) Float(10)\n"
        )
    }
}
