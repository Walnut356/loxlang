use std::rc::Rc;

use log::error;

use crate::{
    chunk::{Chunk, OpCode},
    scanner::{Scanner, Token, TokenKind},
    table::Table,
    value::Value,
    vm::InterpretError,
};

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precedence {
    #[default]
    None,
    Assignment,
    Or,
    And,
    Equality,
    Comparison,
    Term,
    Factor,
    Unary,
    Call,
    Primary,
}

impl Precedence {
    pub fn incr(self) -> Self {
        unsafe { std::mem::transmute(self as u8 + 1) }
    }
}

#[derive(Debug)]
pub struct Parser<'a> {
    pub chunk: &'a mut Chunk,
    string_table: &'a mut Table,
    curr: Token,
    prev: Token,
    pub scanner: Scanner,
    pub errors: bool,
    panic: bool,
}

impl<'a> Parser<'a> {
    pub fn new(source: Rc<str>, chunk: &'a mut Chunk, string_table: &'a mut Table) -> Self {
        let mut scanner = Scanner::new(source);
        Self {
            chunk,
            string_table,
            curr: scanner.next_token(),
            prev: Token {
                kind: TokenKind::EOF,
                data: "",
                line: 0,
            },
            scanner,
            errors: false,
            panic: false,
        }
    }

    pub fn eof(&self) -> bool {
        self.curr.kind == TokenKind::EOF
    }

    pub fn peek_next(&self) -> TokenKind {
        self.curr.kind
    }

    pub fn advance_if(&mut self, token: TokenKind) -> bool {
        if token == self.peek_next() {
            self.advance();
            return true;
        }

        false
    }

    pub fn write<const N: usize>(&mut self, bytes: [u8; N]) {
        self.chunk.data.extend(bytes);
    }

    pub fn log_error(&self, token: &Token, message: &str) {
        match token.kind {
            TokenKind::Error => error!("[Line {}] Error: {message}", self.scanner.line),
            TokenKind::EOF => error!("[Line {}] Unexpected EOF. {message}", self.scanner.line),
            _ => error!(
                "[Line {}] Unexpected token: \"{}\". {message}",
                self.scanner.line, token.data
            ),
        };
    }

    pub fn advance(&mut self) {
        self.prev = self.curr.clone();

        loop {
            self.curr = self.scanner.next_token();
            if self.curr.kind == TokenKind::Error && !self.panic {
                self.log_error(&self.curr, self.curr.data);
                self.errors = true;
                self.panic = true;
            } else {
                break;
            }
        }
    }

    pub fn consume(&mut self, kind: TokenKind, error_msg: &str) {
        if self.curr.kind == kind {
            self.advance();
        } else {
            self.log_error(&self.curr, error_msg);
            self.errors = true;
            self.panic = true;
        }
    }

    pub fn prefix_rule(&mut self, token_kind: TokenKind, can_assign: bool) {
        match token_kind {
            TokenKind::LeftParen => self.grouping(),
            TokenKind::Minus | TokenKind::Bang => self.unary(),
            TokenKind::Number => self.number(),
            TokenKind::False | TokenKind::True | TokenKind::Nil => self.literal(),
            TokenKind::String => self.string(),
            TokenKind::Ident => self.variable(can_assign),
            _ => (),
        }
    }

    pub fn infix_rule(&mut self, token_kind: TokenKind) {
        match token_kind {
            TokenKind::Minus
            | TokenKind::Plus
            | TokenKind::Slash
            | TokenKind::Star
            | TokenKind::NotEq
            | TokenKind::EqEq
            | TokenKind::Gt
            | TokenKind::GtEq
            | TokenKind::Lt
            | TokenKind::LtEq => self.binary(),
            _ => (),
        }
    }

    pub fn parse_precedence(&mut self, p: Precedence) {
        self.advance();

        let can_assign = p <= Precedence::Assignment;
        self.prefix_rule(self.prev.kind, can_assign);

        while p <= self.curr.kind.precedence() {
            self.advance();

            self.infix_rule(self.prev.kind);
        }

        if can_assign && self.advance_if(TokenKind::Eq) {
            self.log_error(&self.prev, "Invalid assignment target");
            self.errors = true;
            self.panic = true;
        }
    }

    pub fn declaration(&mut self) {
        if self.advance_if(TokenKind::Var) {
            self.var_decl();
        } else {
            self.statement();
        }
        if self.panic {
            self.resync();
        }
    }

    pub fn resync(&mut self) {
        self.panic = false;

        while self.curr.kind != TokenKind::EOF {
            if self.prev.kind == TokenKind::Semicolon
                || matches!(
                    self.curr.kind,
                    TokenKind::Class
                        | TokenKind::Fun
                        | TokenKind::Var
                        | TokenKind::For
                        | TokenKind::If
                        | TokenKind::While
                        | TokenKind::Print
                        | TokenKind::Return
                )
            {
                return;
            }

            self.advance();
        }
    }

    pub fn var_decl(&mut self) {
        let global = self.parse_var("Expect variable name.");

        if self.advance_if(TokenKind::Eq) {
            self.expression();
        } else {
            self.chunk.push_opcode(OpCode::Nil, self.scanner.line);
        }

        self.consume(
            TokenKind::Semicolon,
            "Expect ';' after variable declaration.",
        );

        self.var_def(global);
    }

    pub fn parse_var(&mut self, msg: &str) -> u16 {
        self.consume(TokenKind::Ident, msg);

        self.chunk
            .push_constant(Value::alloc_str(self.prev.data, self.string_table))
    }

    pub fn var_def(&mut self, idx: u16) {
        let idx = idx.to_ne_bytes();
        if idx[1] != 0 {
            self.chunk
                .push_opcode(OpCode::DefGlobal16, self.scanner.line);
            self.chunk.data.push(idx[0]);
            self.chunk.data.push(idx[1]);
        } else {
            self.chunk.push_opcode(OpCode::DefGlobal, self.scanner.line);
            self.chunk.data.push(idx[0]);
        }
    }

    pub fn statement(&mut self) {
        match self.curr.kind {
            TokenKind::Print => {
                self.advance();
                self.print_statement();
            }
            _ => {
                self.expression_statement();
            }
        }
    }

    pub fn print_statement(&mut self) {
        self.expression();
        self.consume(TokenKind::Semicolon, "Expect ';' after value.");
        self.chunk.push_opcode(OpCode::Print, self.scanner.line);
    }

    pub fn expression_statement(&mut self) {
        self.expression();
        self.consume(TokenKind::Semicolon, "Expect ';' after expression.");
        self.chunk.push_opcode(OpCode::Pop, self.scanner.line);
    }

    pub fn expression(&mut self) {
        self.parse_precedence(Precedence::Assignment);
    }

    pub fn grouping(&mut self) {
        self.expression();
        self.consume(TokenKind::RightParen, "Expect ')' after expression.");
    }

    pub fn unary(&mut self) {
        let kind = self.prev.kind;
        let line = self.prev.line;

        self.parse_precedence(Precedence::Unary);

        let code = match kind {
            TokenKind::Minus => OpCode::Negate,
            TokenKind::Bang => OpCode::Not,
            _ => unreachable!(),
        };

        self.chunk.push_opcode(code, line);
    }

    pub fn binary(&mut self) {
        let kind = self.prev.kind;
        let line = self.prev.line;

        self.parse_precedence(kind.precedence().incr());

        let code = match kind {
            TokenKind::Plus => OpCode::Add,
            TokenKind::Minus => OpCode::Subtract,
            TokenKind::Star => OpCode::Multiply,
            TokenKind::Slash => OpCode::Divide,
            TokenKind::NotEq => OpCode::Neq,
            TokenKind::EqEq => OpCode::Eq,
            TokenKind::Gt => OpCode::Gt,
            TokenKind::GtEq => OpCode::GtEq,
            TokenKind::Lt => OpCode::Lt,
            TokenKind::LtEq => OpCode::LtEq,

            _ => unreachable!(),
        };

        self.chunk.push_opcode(code, line);
    }

    pub fn number(&mut self) {
        match self.prev.data.parse::<f64>() {
            Ok(x) => {
                self.chunk.insert_constant(Value::Float(x), self.prev.line);
            }
            Err(x) => {
                self.log_error(&self.prev, &format!("{x:?}"));
                self.errors = true;
                self.panic = true;
            }
        }
    }

    pub fn literal(&mut self) {
        let code = match self.prev.kind {
            TokenKind::True => OpCode::True,
            TokenKind::False => OpCode::False,
            TokenKind::Nil => OpCode::Nil,
            _ => unreachable!(),
        };

        self.chunk.push_opcode(code, self.prev.line);
    }

    pub fn string(&mut self) {
        self.chunk.insert_constant(
            Value::alloc_str(
                &self.prev.data[1..self.prev.data.len() - 1],
                self.string_table,
            ),
            self.prev.line,
        );
    }

    pub fn variable(&mut self, can_assign: bool) {
        let arg = Value::alloc_str(self.prev.data, self.string_table);
        if can_assign && self.advance_if(TokenKind::Eq) {
            self.expression();
            self.chunk.insert_write_global(arg, self.scanner.line);
        } else {
            self.chunk.insert_read_global(arg, self.scanner.line);
        }
    }
}
