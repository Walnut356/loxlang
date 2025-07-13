use crate::value::Value;
use strum::VariantNames;
use strum_macros::*;
// use std::io::Write;
use std::fmt::Write;

#[derive(Debug, FromRepr, VariantNames)]
#[repr(u8)]
pub enum OpCode {
    Return,
    Constant,
    Constant16,
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
}

impl OpCode {
    /// Returns the byte-size of the opcode + its operand
    pub fn total_size(&self) -> usize {
        match self {
            OpCode::Constant => 2,
            OpCode::Constant16 => 3,
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
            Some(OpCode::Constant) => {
                let idx = self.data[offset + 1] as usize;
                writeln!(output, "Constant: ({idx:03}) {}", self.constants[idx]).unwrap();

                offset + 2
            }
            Some(OpCode::Constant16) => {
                let idx = unsafe { self.data.as_ptr().byte_add(offset + 1).cast::<u16>().read() }
                    as usize;
                writeln!(output, "Constant16: ({idx:05}) {}", self.constants[idx]).unwrap();

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

    pub fn insert_constant(&mut self, value: Value, line: u32) {
        self.push_opcode(OpCode::Constant, line);
        let idx = self.push_constant(value).to_ne_bytes();
        self.data.push(idx[0]);
        if idx[1] != 0 {
            self.data.push(idx[1]);
        }
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
