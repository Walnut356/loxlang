use crate::value::Value;
use strum::VariantNames;
use strum_macros::*;
// use std::io::Write;
use std::fmt::Write;

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
}

impl OpCode {
    /// Returns the byte-size of the opcode + its operand
    pub fn total_size(&self) -> usize {
        match self {
            OpCode::Constant
            | OpCode::DefGlobal
            | OpCode::ReadGlobal
            | OpCode::WriteGlobal
            | OpCode::StackSub => 2,
            OpCode::Constant16
            | OpCode::DefGlobal16
            | OpCode::ReadGlobal16
            | OpCode::WriteGlobal16 => 3,
            _ => 1,
        }
    }
}

/// Run-length encoded line number
#[derive(Debug, Default)]
pub struct LineRun {
    line: u32,
    len: u32,
}

#[derive(Debug, Default)]
pub struct Chunk {
    pub data: Vec<u8>,
    pub constants: Vec<Value>,
    pub lines: Vec<LineRun>,
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
            if curr as usize >= offset {
                return l.line;
            }
        }

        curr
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
        match OpCode::from_repr(opcode) {
            Some(OpCode::StackSub) | Some(OpCode::ReadLocal) | Some(OpCode::WriteLocal) => {
                let idx = self.data[offset + 1] as usize;
                writeln!(output, "{}: {idx:03}", OpCode::VARIANTS[opcode as usize]).unwrap();

                offset + 2
            }
            Some(OpCode::Constant)
            | Some(OpCode::DefGlobal)
            | Some(OpCode::ReadGlobal)
            | Some(OpCode::WriteGlobal) => {
                let idx = self.data[offset + 1] as usize;
                writeln!(
                    output,
                    "{}: ({idx:03}) {:?}",
                    OpCode::VARIANTS[opcode as usize],
                    self.constants[idx]
                )
                .unwrap();

                offset + 2
            }
            Some(OpCode::Constant16)
            | Some(OpCode::DefGlobal16)
            | Some(OpCode::ReadGlobal16)
            | Some(OpCode::WriteGlobal16) => {
                let idx = unsafe { self.data.as_ptr().byte_add(offset + 1).cast::<u16>().read() }
                    as usize;
                writeln!(
                    output,
                    "{}: ({idx:05}) {:?}",
                    OpCode::VARIANTS[opcode as usize],
                    self.constants[idx]
                )
                .unwrap();

                offset + 2
            }
            Some(_) => {
                writeln!(output, "{}", OpCode::VARIANTS[opcode as usize]).unwrap();
                offset + 1
            }
            None => {
                writeln!(output, "Unknown opcode: {opcode}").unwrap();
                offset + 1
            }
        }
    }

    pub fn push_opcode(&mut self, code: OpCode, line: u32) {
        self.data.push(code as u8);

        // absolutely gorgeous
        match self.lines.last_mut() {
            Some(l) if l.line == line => l.len += 1,
            _ => self.lines.push(LineRun { line, len: 1 }),
        }
    }

    pub fn push_constant(&mut self, value: Value) -> u16 {
        if let Some(i) = self.constants.iter().position(|x| *x == value) {
            i as u16
        } else {
            self.constants.push(value);
            (self.constants.len() - 1) as u16
        }
    }

    pub fn insert_constant(&mut self, value: Value, line: u32) -> u16 {
        let idx = self.push_constant(value).to_ne_bytes();
        if idx[1] != 0 {
            self.push_opcode(OpCode::Constant16, line);
            self.data.push(idx[0]);
            self.data.push(idx[1]);
        } else {
            self.push_opcode(OpCode::Constant, line);
            self.data.push(idx[0]);
        }

        u16::from_ne_bytes(idx)
    }

    pub fn push_jump(&mut self, opcode: OpCode, line: u32) -> usize {
        self.push_opcode(opcode, line);
        self.data.extend(u16::MAX.to_ne_bytes());
        self.data.len() - 2
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
