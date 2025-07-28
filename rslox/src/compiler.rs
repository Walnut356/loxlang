use std::rc::Rc;

use log::error;

use crate::{
    chunk::{Chunk, OpCode},
    scanner::{Scanner, Token, TokenKind},
    table::Table,
    value::{Function, Value},
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
    string_table: &'a mut Table,
    pub compiler: Compiler,
    curr: Token,
    prev: Token,
    pub scanner: Scanner,
    pub errors: bool,
    panic: bool,
}

impl<'a> Parser<'a> {
    pub fn new(source: Rc<str>, string_table: &'a mut Table) -> Self {
        let mut scanner = Scanner::new(source);
        Self {
            string_table,
            compiler: Compiler::default(),
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
        self.compiler.func.chunk.data.extend(bytes);
    }

    pub fn log_error(&self, token: &Token, message: &str) {
        match token.kind {
            TokenKind::Error => error!("[Line {}] Error: {message}", self.prev.line),
            TokenKind::EOF => error!("[Line {}] Unexpected EOF. {message}", self.prev.line),
            _ => error!(
                "[Line {}] Unexpected token: \"{}\". {message}",
                self.prev.line, token.data
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
            TokenKind::And => self.and(),
            TokenKind::Or => self.or(),
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
            self.compiler.func.chunk.push_opcode(OpCode::Nil, self.prev.line);
        }

        self.consume(
            TokenKind::Semicolon,
            "Expect ';' after variable declaration.",
        );

        self.var_def(global);
    }

    pub fn parse_var(&mut self, msg: &str) -> u16 {
        self.consume(TokenKind::Ident, msg);

        self.declare_variable();

        if !self.compiler.global_scope() {
            0
        } else {
            self.compiler.func.chunk
                .push_constant(Value::alloc_str(self.prev.data, self.string_table))
        }
    }

    pub fn var_def(&mut self, idx: u16) {
        if self.compiler.local_scope() {
            self.compiler.locals[idx as usize].depth = self.compiler.scope_depth;
            return;
        }

        let idx = idx.to_ne_bytes();
        if idx[1] != 0 {
            self.compiler.func.chunk.push_opcode(OpCode::DefGlobal16, self.prev.line);
            self.compiler.func.chunk.data.push(idx[0]);
            self.compiler.func.chunk.data.push(idx[1]);
        } else {
            self.compiler.func.chunk.push_opcode(OpCode::DefGlobal, self.prev.line);
            self.compiler.func.chunk.data.push(idx[0]);
        }
    }

    pub fn declare_variable(&mut self) {
        if self.compiler.global_scope() {
            return;
        }

        for local in self.compiler.locals[..self.compiler.count as usize]
            .iter()
            .rev()
        {
            if local.depth != u32::MAX && local.depth < self.compiler.scope_depth {
                break;
            }

            if local.name.data == self.prev.data {
                self.log_error(
                    &self.prev,
                    "There is already a variable with this name in this scope.",
                );
            }
        }

        self.add_local();
    }

    pub fn add_local(&mut self) {
        if self.compiler.count as usize > self.compiler.locals.len() {
            self.log_error(&self.prev, "Too many loal variables in function.");
            self.errors = true;
            self.panic = true;
            return;
        }

        self.compiler.locals[self.compiler.count as usize] = Local {
            name: self.prev.clone(),
            depth: self.compiler.scope_depth,
        };
        self.compiler.count += 1;
    }

    pub fn statement(&mut self) {
        match self.curr.kind {
            TokenKind::Print => {
                self.advance();
                self.print_statement();
            }
            TokenKind::LeftBrace => {
                self.advance();
                self.compiler.scope_depth += 1;
                let count = self.compiler.count;
                self.block();
                self.compiler.scope_depth -= 1;

                // there should always be exactly as many locals before and after a scope.
                // That means we don't need to iterate through like the book does, we can just
                // record that number and reset to it.
                let stack_pop = self.compiler.count - count;
                if stack_pop > 0 {
                    self.compiler.func.chunk.push_opcode(OpCode::StackSub, self.prev.line);
                    self.compiler.func.chunk.data.push(stack_pop as u8);
                }

                self.compiler.count = count;
            }
            TokenKind::If => {
                self.advance();
                self.if_statement();
            }
            TokenKind::While => {
                self.advance();
                self.while_statement();
            }
            TokenKind::For => {
                self.advance();
                self.for_statement();
            }
            _ => {
                self.expression_statement();
            }
        }
    }

    pub fn block(&mut self) {
        while !matches!(self.peek_next(), TokenKind::RightBrace | TokenKind::EOF) {
            self.declaration();
        }

        self.consume(TokenKind::RightBrace, "Expect '}' after block.");
    }

    pub fn print_statement(&mut self) {
        self.expression();
        self.consume(TokenKind::Semicolon, "Expect ';' after value.");
        self.compiler.func.chunk.push_opcode(OpCode::Print, self.prev.line);
    }

    pub fn patch_jump(&mut self, idx: usize) {
        let jump = self.compiler.func.chunk.data.len() - idx - 2;

        if jump > u16::MAX as usize {
            self.log_error(&self.prev, "Cannot jump more than 16::MAX bytes");
            self.errors = true;
            self.panic = true;
        }

        self.compiler.func.chunk
            .data
            .get_mut(idx..=idx + 1)
            .unwrap()
            .copy_from_slice(&(jump as u16).to_ne_bytes());
    }

    pub fn if_statement(&mut self) {
        self.consume(TokenKind::LeftParen, "Expect '(' after 'if'.");
        self.expression();
        self.consume(TokenKind::RightParen, "Expect ')' after condition.");

        let if_jump_idx = self.compiler.func.chunk.push_jump(OpCode::JumpFalsey, self.prev.line);

        self.compiler.func.chunk.push_opcode(OpCode::Pop, self.prev.line);
        self.statement();

        let else_jump_idx = self.compiler.func.chunk.push_jump(OpCode::Jump, self.prev.line);
        self.compiler.func.chunk.push_opcode(OpCode::Pop, self.prev.line);

        self.patch_jump(if_jump_idx);

        if self.advance_if(TokenKind::Else) {
            self.statement();
        }

        self.patch_jump(else_jump_idx);
    }

    pub fn while_statement(&mut self) {
        let loop_start = self.compiler.func.chunk.data.len();
        self.consume(TokenKind::LeftParen, "Expect '(' after 'while'");
        self.expression();
        self.consume(TokenKind::RightParen, "Expect ')' after condition.");

        let exit_jump = self.compiler.func.chunk.push_jump(OpCode::JumpFalsey, self.prev.line);
        self.compiler.func.chunk.push_opcode(OpCode::Pop, self.prev.line);
        self.statement();
        self.compiler.func.chunk.push_loop(loop_start, self.prev.line);

        self.patch_jump(exit_jump);
        self.compiler.func.chunk.push_opcode(OpCode::Pop, self.prev.line);
    }

    pub fn for_statement(&mut self) {
        self.compiler.scope_depth += 1;
        self.consume(TokenKind::LeftParen, "Expect '(' after 'for'");

        match self.peek_next() {
            TokenKind::Semicolon => self.advance(),
            TokenKind::Var => {
                self.advance();
                self.var_decl();
            }
            _ => self.expression_statement(),
        }

        let mut loop_start = self.compiler.func.chunk.data.len();
        let mut exit_jump = None;

        match self.peek_next() {
            TokenKind::Semicolon => self.advance(),
            _ => {
                self.expression();
                self.consume(TokenKind::Semicolon, "Expect ';' after for-loop condition");

                exit_jump = Some(self.compiler.func.chunk.push_jump(OpCode::JumpFalsey, self.prev.line));
                self.compiler.func.chunk.push_opcode(OpCode::Pop, self.prev.line);
            }
        }

        match self.peek_next() {
            TokenKind::RightParen => self.advance(),
            _ => {
                let body_jmp = self.compiler.func.chunk.push_jump(OpCode::Jump, self.prev.line);
                let incr_start = self.compiler.func.chunk.data.len();

                self.expression();
                self.compiler.func.chunk.push_opcode(OpCode::Pop, self.prev.line);
                self.consume(TokenKind::RightParen, "Expect ')' after for-loop clauses");

                self.compiler.func.chunk.push_loop(loop_start, self.prev.line);
                loop_start = incr_start;
                self.patch_jump(body_jmp);
            }
        }

        self.statement();
        self.compiler.func.chunk.push_loop(loop_start, self.prev.line);

        if let Some(jmp) = exit_jump {
            self.patch_jump(jmp);
            self.compiler.func.chunk.push_opcode(OpCode::Pop, self.prev.line);
        }
        self.compiler.scope_depth -= 1;
    }

    pub fn expression_statement(&mut self) {
        self.expression();
        self.consume(TokenKind::Semicolon, "Expect ';' after expression.");
        self.compiler.func.chunk.push_opcode(OpCode::Pop, self.prev.line);
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

        self.compiler.func.chunk.push_opcode(code, line);
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

        self.compiler.func.chunk.push_opcode(code, line);
    }

    pub fn and(&mut self) {
        let jump_idx = self.compiler.func.chunk.push_jump(OpCode::JumpFalsey, self.prev.line);

        self.compiler.func.chunk.push_opcode(OpCode::Pop, self.prev.line);
        self.parse_precedence(Precedence::And);

        self.patch_jump(jump_idx);
    }

    pub fn or(&mut self) {
        let jump_idx = self.compiler.func.chunk.push_jump(OpCode::JumpTruthy, self.prev.line);

        self.compiler.func.chunk.push_opcode(OpCode::Pop, self.prev.line);
        self.parse_precedence(Precedence::Or);

        self.patch_jump(jump_idx);
    }

    pub fn number(&mut self) {
        match self.prev.data.parse::<f64>() {
            Ok(x) => {
                self.compiler.func.chunk.insert_constant(Value::Float(x), self.prev.line);
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

        self.compiler.func.chunk.push_opcode(code, self.prev.line);
    }

    pub fn string(&mut self) {
        self.compiler.func.chunk.insert_constant(
            Value::alloc_str(
                &self.prev.data[1..self.prev.data.len() - 1],
                self.string_table,
            ),
            self.prev.line,
        );
    }

    pub fn variable(&mut self, can_assign: bool) {
        let mut local_idx = self.compiler.resolve_local(self.prev.data);

        let (get_op, set_op) = if let Some(idx) = local_idx {
            if self.compiler.locals[idx as usize].depth == UNINITIALIZED {
                self.log_error(
                    &self.prev,
                    "Cannot read local variable in its own initializer.",
                );
                self.errors = true;
                self.panic = true;
            }
            (OpCode::ReadLocal, OpCode::WriteLocal)
        } else {
            local_idx = Some(
                self.compiler.func.chunk
                    .push_constant(Value::alloc_str(self.prev.data, self.string_table)),
            );

            (OpCode::ReadGlobal, OpCode::WriteGlobal)
        };

        let arg = local_idx.unwrap();

        if can_assign && self.advance_if(TokenKind::Eq) {
            self.expression();
            self.compiler.func.chunk.push_opcode(set_op, self.prev.line);
        } else {
            self.compiler.func.chunk.push_opcode(get_op, self.prev.line);
        }

        if arg > u8::MAX as u16 {
            self.compiler.func.chunk.data.extend(arg.to_ne_bytes());
        } else {
            self.compiler.func.chunk.data.push(arg as u8);
        }
    }
}

#[derive(Debug, Clone)]
pub struct Local {
    name: Token,
    depth: u32,
}

impl Default for Local {
    fn default() -> Self {
        Self {
            name: Token {
                kind: TokenKind::Error,
                data: "",
                line: 0,
            },
            depth: 0,
        }
    }
}

pub const MAX_LOCALS: usize = 256;
pub const GLOBAL_SCOPE: u32 = 0;
pub const UNINITIALIZED: u32 = u32::MAX;

#[derive(Debug)]
enum FuncKind {
    Func,
    Script
}

#[derive(Debug)]
pub struct Compiler {
    pub func: &'static mut Function,
    pub kind: FuncKind,
    pub locals: [Local; MAX_LOCALS],
    pub count: u32,
    pub scope_depth: u32,
}

impl Default for Compiler {
    fn default() -> Self {
        let func = Box::leak(Box::new(Function::default()));
        Self {
            func,
            kind: FuncKind::Script,
            locals: std::array::from_fn(|_| Local::default()),
            count: 1,
            scope_depth: Default::default(),
        }
    }
}

impl Compiler {
    pub fn global_scope(&self) -> bool {
        self.scope_depth == GLOBAL_SCOPE
    }

    pub fn local_scope(&self) -> bool {
        self.scope_depth > GLOBAL_SCOPE
    }

    pub fn add_local(&mut self, token: Token) {
        self.locals[self.count as usize] = Local {
            name: token,
            depth: UNINITIALIZED,
        };
    }

    pub fn resolve_local(&self, name: &'static str) -> Option<u16> {
        self.locals[..self.count as usize]
            .iter()
            .rev()
            .position(|x| x.name.data == name)
            .map(|x| x as u16)
    }
}
