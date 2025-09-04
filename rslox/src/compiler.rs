use std::rc::Rc;

use tracing::error;

use crate::{
    chunk::OpCode,
    scanner::{Scanner, Token, TokenKind},
    table::Table,
    value::{Function, Value},
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
    heap_objects: &'a mut Vec<Value>,
    pub compiler: Compiler,
    pub class_compiler: Option<ClassCompiler>,
    curr: Token,
    prev: Token,
    pub scanner: Scanner,
    pub errors: bool,
    panic: bool,
}

impl<'a> Parser<'a> {
    pub fn new(
        source: Rc<str>,
        string_table: &'a mut Table,
        heap_objects: &'a mut Vec<Value>,
    ) -> Self {
        let mut scanner = Scanner::new(source.clone());
        let compiler = Compiler::new(heap_objects);
        let res = Self {
            string_table,
            heap_objects,
            compiler,
            class_compiler: Default::default(),
            curr: scanner.next_token(),
            prev: Token {
                kind: TokenKind::EOF,
                data: "",
                line: 0,
            },
            scanner,
            errors: false,
            panic: false,
        };

        res.compiler.func.chunk.source = source;

        res
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

    pub fn log_error(&self, token: &Token, message: &str) {
        match token.kind {
            TokenKind::Error => error!("[Line {}] Error: {message}", token.line),
            TokenKind::EOF => error!("[Line {}] Unexpected EOF. {message}", token.line),
            _ => error!(
                "[Line {}] Unexpected token: \'{}\'. {message}",
                token.line, token.data
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
            TokenKind::This => self.this(),
            TokenKind::Super => self.super_(),
            _ => (),
        }
    }

    pub fn infix_rule(&mut self, token_kind: TokenKind, can_assign: bool) {
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
            TokenKind::LeftParen => self.call(),
            TokenKind::Dot => self.dot(can_assign),
            _ => (),
        }
    }

    pub fn parse_precedence(&mut self, p: Precedence) {
        self.advance();

        let can_assign = p <= Precedence::Assignment;
        self.prefix_rule(self.prev.kind, can_assign);

        while p <= self.curr.kind.precedence() {
            self.advance();

            self.infix_rule(self.prev.kind, can_assign);
        }

        if can_assign && self.advance_if(TokenKind::Eq) {
            self.log_error(&self.prev, "Invalid assignment target");
            self.errors = true;
            self.panic = true;
        }
    }

    pub fn declaration(&mut self) {
        match self.peek_next() {
            TokenKind::Class => {
                self.advance();
                self.class_decl();
            }
            TokenKind::Fun => {
                self.advance();
                self.func_decl();
            }
            TokenKind::Var => {
                self.advance();
                self.var_decl();
            }
            _ => {
                self.statement();
            }
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

    pub fn end_scope(&mut self) {
        self.compiler.scope_depth -= 1;

        let mut stack_pop: u8 = 0;

        for l in self.compiler.locals[1..self.compiler.local_count as usize]
            .iter()
            .rev()
        {
            if l.depth <= self.compiler.scope_depth {
                break;
            }

            if l.captured {
                // try to batch pops when possible
                match stack_pop {
                    0 => {}
                    1 => {
                        self.compiler
                            .func
                            .chunk
                            .push_opcode(OpCode::Pop, self.prev.line);
                    }
                    _ => {
                        self.compiler
                            .func
                            .chunk
                            .push_opcode(OpCode::StackSub, self.prev.line);
                        self.compiler.func.chunk.push_bytes(&[stack_pop]);
                    }
                }

                stack_pop = 0;

                self.compiler
                    .func
                    .chunk
                    .push_opcode(OpCode::CloseUpVal, self.prev.line);
            } else {
                stack_pop += 1;
            }

            self.compiler.local_count -= 1;
        }

        match stack_pop {
            0 => {}
            1 => {
                self.compiler
                    .func
                    .chunk
                    .push_opcode(OpCode::Pop, self.prev.line);
            }
            _ => {
                self.compiler
                    .func
                    .chunk
                    .push_opcode(OpCode::StackSub, self.prev.line);
                self.compiler.func.chunk.push_bytes(&[stack_pop]);
            }
        }
    }

    pub fn class_decl(&mut self) {
        self.consume(TokenKind::Ident, "Expect class name");
        let class_name = self.prev.data;

        let name_constant = self.compiler.func.chunk.push_constant(Value::alloc_str(
            self.prev.data,
            self.string_table,
            self.heap_objects,
        ));

        self.declare_variable();

        self.compiler
            .func
            .chunk
            .push_opcode(OpCode::Class, self.prev.line);

        self.compiler.func.chunk.push_bytes(&[name_constant]);

        self.var_def(name_constant);

        let mut cc = Some(ClassCompiler::default());

        std::mem::swap(&mut cc, &mut self.class_compiler);

        self.class_compiler.as_mut().unwrap().parent = cc.as_mut().map(|x| x as *mut _);

        if self.advance_if(TokenKind::Lt) {
            self.consume(TokenKind::Ident, "Expect superclass name.");
            self.variable(false);

            if class_name == self.prev.data {
                self.log_error(&self.prev, "Class cannot inheret itself");
                self.panic = true;
                self.errors = true;
            }

            self.compiler.scope_depth += 1;

            // super dumb, but it works. self.add_local() operates on self.prev without advancing,
            // so since we're adding a synthetic token, we just replace self.prev temporarily
            let temp = self.prev.clone();
            self.prev = Token {
                kind: TokenKind::Super,
                data: "super",
                line: temp.line,
            };
            self.add_local();
            self.var_def(0);

            self.prev = temp;

            self.named_variable(class_name, false);
            self.compiler
                .func
                .chunk
                .push_opcode(OpCode::Inherit, self.prev.line);

            self.class_compiler.as_mut().unwrap().has_superclass = true;
        }

        self.named_variable(class_name, false);

        self.consume(TokenKind::LeftBrace, "Expect '{' before class body.");

        while !self.eof() && self.curr.kind != TokenKind::RightBrace {
            self.method();
        }

        self.consume(TokenKind::RightBrace, "Expect '}' after class body.");

        self.compiler
            .func
            .chunk
            .push_opcode(OpCode::Pop, self.prev.line);

        if self
            .class_compiler
            .as_ref()
            .is_some_and(|x| x.has_superclass)
        {
            self.end_scope();
        }

        std::mem::swap(&mut cc, &mut self.class_compiler);
    }

    pub fn method(&mut self) {
        self.consume(TokenKind::Ident, "Expect method name");

        let constant = self.compiler.func.chunk.push_constant(Value::alloc_str(
            self.prev.data,
            self.string_table,
            self.heap_objects,
        ));

        let kind = if self.prev.data == "init" {
            FuncKind::Initializer
        } else {
            FuncKind::Method
        };

        self.function(kind);
        self.compiler
            .func
            .chunk
            .push_opcode(OpCode::Method, self.prev.line);
        self.compiler.func.chunk.push_bytes(&[constant]);
    }

    pub fn super_(&mut self) {
        self.consume(TokenKind::Dot, "Expect '.' after 'super'.");
        self.consume(TokenKind::Ident, "Expect superclass method name.");

        let name = self.compiler.func.chunk.push_constant(Value::alloc_str(
            self.prev.data,
            self.string_table,
            self.heap_objects,
        ));

        self.named_variable("this", false);

        if self.advance_if(TokenKind::LeftParen) {
            let arg_count = self.argument_list();
            self.named_variable("super", false);
            self.compiler
                .func
                .chunk
                .push_opcode(OpCode::SuperInvoke, self.prev.line);
            self.compiler.func.chunk.push_bytes(&[name, arg_count]);
        } else {
            self.named_variable("super", false);
            self.compiler
                .func
                .chunk
                .push_opcode(OpCode::Super, self.prev.line);
            self.compiler.func.chunk.push_bytes(&[name]);
        }

        if self.class_compiler.is_none() {
            self.log_error(&self.prev, "Can't use 'super' outside of a class.");
            self.errors = true;
            self.panic = true;
        } else if self
            .class_compiler
            .as_ref()
            .is_some_and(|x| !x.has_superclass)
        {
            self.log_error(
                &self.prev,
                "Can't use 'super' in a class with no superclass.",
            );
            self.errors = true;
            self.panic = true;
        }
    }

    pub fn this(&mut self) {
        if self.class_compiler.is_none() {
            self.log_error(&self.prev, "Can't use 'this' outside of a class.");
            self.panic = true;
            self.errors = true;
            return;
        }
        self.variable(false);
    }

    pub fn func_decl(&mut self) {
        let global = self.parse_var("Expect function name.");

        if self.compiler.local_scope() {
            self.compiler.locals[global as usize].depth = self.compiler.scope_depth;
        }

        self.function(FuncKind::Func);
        self.var_def(global);
    }

    pub fn function(&mut self, kind: FuncKind) {
        let line = self.prev.line;

        let mut inner_compiler = Compiler::new(self.heap_objects);

        inner_compiler.kind = kind;
        inner_compiler.scope_depth = 1;
        inner_compiler.func.chunk.source = self.scanner.source.clone();

        if kind != FuncKind::Script {
            inner_compiler.func.name = self.prev.data;
            if matches!(kind, FuncKind::Method | FuncKind::Initializer) {
                inner_compiler.locals[0].name = Token {
                    kind: TokenKind::This,
                    data: "this",
                    line: self.prev.line,
                };
                inner_compiler.locals[0].depth = inner_compiler.scope_depth;
            }
        }

        std::mem::swap(&mut self.compiler, &mut inner_compiler);

        self.compiler.parent = Some(&mut inner_compiler as *mut _);

        self.consume(TokenKind::LeftParen, "Expect '(' after function name.");

        if self.peek_next() != TokenKind::RightParen {
            loop {
                if self.compiler.func.arg_count == 255 {
                    self.log_error(&self.prev, "Can't have more than 255 parameters.");
                    self.errors = true;
                    self.panic = true;
                    return;
                }

                self.compiler.func.arg_count += 1;

                let constant = self.parse_var("Expect parameter name");
                self.var_def(constant);

                if !self.advance_if(TokenKind::Comma) {
                    break;
                }
            }
        }

        self.consume(TokenKind::RightParen, "Expect ')' after parameters.");
        self.consume(TokenKind::LeftBrace, "Expect '{' before function body.");

        self.block();

        if kind == FuncKind::Initializer {
            self.compiler
                .func
                .chunk
                .push_opcode(OpCode::ReadLocal, self.prev.line);
            self.compiler.func.chunk.push_bytes(&[0]);
        } else {
            self.compiler
                .func
                .chunk
                .push_opcode(OpCode::Nil, self.prev.line);
        }
        self.compiler
            .func
            .chunk
            .push_opcode(OpCode::Return, self.prev.line);

        std::mem::swap(&mut self.compiler, &mut inner_compiler);

        // self.compiler.func.chunk.push_opcode(OpCode::Constant, line);
        self.compiler.func.chunk.push_opcode(OpCode::Closure, line);

        let idx = self
            .compiler
            .func
            .chunk
            .push_constant(Value::Function(inner_compiler.func.into()));
        self.compiler.func.chunk.push_bytes(&[idx]);

        for i in 0..inner_compiler.func.upval_count {
            let val = &inner_compiler.upvalues[i as usize];
            self.compiler
                .func
                .chunk
                .push_bytes(&[val.local as u8, val.idx]);
        }
    }

    pub fn var_decl(&mut self) {
        let line = self.curr.line;
        let global = self.parse_var("Expect variable name.");

        if self.advance_if(TokenKind::Eq) {
            self.expression();
        } else {
            self.compiler.func.chunk.push_opcode(OpCode::Nil, line);
        }

        self.consume(
            TokenKind::Semicolon,
            "Expect ';' after variable declaration.",
        );

        self.var_def(global);
    }

    pub fn parse_var(&mut self, msg: &str) -> u8 {
        self.consume(TokenKind::Ident, msg);

        self.declare_variable();

        if !self.compiler.global_scope() {
            0
        } else {
            self.compiler.func.chunk.push_constant(Value::alloc_str(
                self.prev.data,
                self.string_table,
                self.heap_objects,
            ))
        }
    }

    pub fn var_def(&mut self, idx: u8) {
        if self.compiler.local_scope() {
            self.compiler.locals[self.compiler.local_count as usize - 1].depth =
                self.compiler.scope_depth;
            return;
        }

        // let idx = idx;
        // if idx[1] != 0 {
        //     self.compiler
        //         .func
        //         .chunk
        //         .push_opcode(OpCode::DefGlobal16, self.prev.line);
        //     self.compiler.func.chunk.push_bytes(&idx);
        // } else {
        self.compiler
            .func
            .chunk
            .push_opcode(OpCode::DefGlobal, self.prev.line);
        self.compiler.func.chunk.push_bytes(&[idx]);
        // }
    }

    pub fn declare_variable(&mut self) {
        if self.compiler.global_scope() {
            return;
        }

        for local in self.compiler.locals[..self.compiler.local_count as usize]
            .iter()
            .rev()
        {
            if local.depth != UNINITIALIZED && local.depth < self.compiler.scope_depth {
                break;
            }

            if local.name.data == self.prev.data {
                self.log_error(
                    &self.prev,
                    "There is already a variable with this name in this scope.",
                );
                self.errors = true;
                self.panic = true;
            }
        }

        self.add_local();
    }

    pub fn add_local(&mut self) {
        if self.compiler.local_count as usize >= MAX_LOCALS {
            self.log_error(&self.prev, "Too many loal variables in function.");
            self.errors = true;
            self.panic = true;
            return;
        }

        self.compiler.locals[self.compiler.local_count as usize] = Local {
            name: self.prev.clone(),
            depth: UNINITIALIZED,
            captured: false,
        };
        self.compiler.local_count += 1;
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
                // let count = self.compiler.local_count;
                self.block();
                self.end_scope();
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
            TokenKind::Return => {
                self.advance();
                self.return_statement();
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
        let line = self.prev.line;
        self.expression();
        self.consume(TokenKind::Semicolon, "Expect ';' after value.");
        self.compiler.func.chunk.push_opcode(OpCode::Print, line);
    }

    pub fn patch_jump(&mut self, idx: usize) {
        let jump = (self.compiler.func.chunk.data.len()) - idx - 2;

        if jump > u16::MAX as usize {
            self.log_error(&self.prev, "Cannot jump more than 16::MAX bytes");
            self.errors = true;
            self.panic = true;
        }

        self.compiler
            .func
            .chunk
            .data
            .get_mut(idx..=idx + 1)
            .unwrap()
            .copy_from_slice(&(jump as u16).to_ne_bytes());
    }

    pub fn if_statement(&mut self) {
        self.consume(TokenKind::LeftParen, "Expect '(' after 'if'.");
        self.expression();
        self.consume(TokenKind::RightParen, "Expect ')' after condition.");

        let if_jump_idx = self
            .compiler
            .func
            .chunk
            .push_jump(OpCode::JumpFalsey, self.prev.line);

        self.compiler
            .func
            .chunk
            .push_opcode(OpCode::Pop, self.prev.line);

        self.statement();

        let else_jump_idx = self
            .compiler
            .func
            .chunk
            .push_jump(OpCode::Jump, self.prev.line);

        self.patch_jump(if_jump_idx);

        self.compiler
            .func
            .chunk
            .push_opcode(OpCode::Pop, self.prev.line);

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

        let exit_jump = self
            .compiler
            .func
            .chunk
            .push_jump(OpCode::JumpFalsey, self.prev.line);
        self.compiler
            .func
            .chunk
            .push_opcode(OpCode::Pop, self.prev.line);
        self.statement();
        self.compiler
            .func
            .chunk
            .push_loop(loop_start, self.prev.line);

        self.patch_jump(exit_jump);
        self.compiler
            .func
            .chunk
            .push_opcode(OpCode::Pop, self.prev.line);
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

                exit_jump = Some(
                    self.compiler
                        .func
                        .chunk
                        .push_jump(OpCode::JumpFalsey, self.prev.line),
                );
                self.compiler
                    .func
                    .chunk
                    .push_opcode(OpCode::Pop, self.prev.line);
            }
        }

        match self.peek_next() {
            TokenKind::RightParen => self.advance(),
            _ => {
                let body_jmp = self
                    .compiler
                    .func
                    .chunk
                    .push_jump(OpCode::Jump, self.prev.line);
                let incr_start = self.compiler.func.chunk.data.len();

                self.expression();
                self.compiler
                    .func
                    .chunk
                    .push_opcode(OpCode::Pop, self.prev.line);
                self.consume(TokenKind::RightParen, "Expect ')' after for-loop clauses");

                self.compiler
                    .func
                    .chunk
                    .push_loop(loop_start, self.prev.line);

                loop_start = incr_start;

                self.patch_jump(body_jmp);
            }
        }

        self.statement();
        self.compiler
            .func
            .chunk
            .push_loop(loop_start, self.prev.line);

        if let Some(jmp) = exit_jump {
            self.patch_jump(jmp);
            self.compiler
                .func
                .chunk
                .push_opcode(OpCode::Pop, self.prev.line);
        }

        // self.compiler.scope_depth -= 1;
        self.end_scope();
    }

    pub fn expression_statement(&mut self) {
        self.expression();
        self.consume(TokenKind::Semicolon, "Expect ';' after expression.");
        self.compiler
            .func
            .chunk
            .push_opcode(OpCode::Pop, self.prev.line);
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

    pub fn return_statement(&mut self) {
        if self.compiler.kind == FuncKind::Script {
            self.log_error(&self.prev, "Can't return from top-level code.");
            self.errors = true;
            self.panic = true;
        }

        if self.advance_if(TokenKind::Semicolon) {
            if self.compiler.kind == FuncKind::Initializer {
                self.compiler
                    .func
                    .chunk
                    .push_opcode(OpCode::ReadLocal, self.prev.line);
                self.compiler.func.chunk.push_bytes(&[0]);
            } else {
                self.compiler
                    .func
                    .chunk
                    .push_opcode(OpCode::Nil, self.prev.line);
            }
        } else {
            if self.compiler.kind == FuncKind::Initializer {
                self.log_error(&self.prev, "Can't reutrn a value from an initializer.");
                self.panic = true;
                self.errors = true;
            }
            self.expression();
            self.consume(TokenKind::Semicolon, "Expect ';' after return value.");
        }

        self.compiler
            .func
            .chunk
            .push_opcode(OpCode::Return, self.prev.line);
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
        let jump_idx = self
            .compiler
            .func
            .chunk
            .push_jump(OpCode::JumpFalsey, self.prev.line);

        self.compiler
            .func
            .chunk
            .push_opcode(OpCode::Pop, self.prev.line);
        self.parse_precedence(Precedence::And);

        self.patch_jump(jump_idx);
    }

    pub fn or(&mut self) {
        let jump_idx = self
            .compiler
            .func
            .chunk
            .push_jump(OpCode::JumpTruthy, self.prev.line);

        self.compiler
            .func
            .chunk
            .push_opcode(OpCode::Pop, self.prev.line);
        self.parse_precedence(Precedence::Or);

        self.patch_jump(jump_idx);
    }

    pub fn call(&mut self) {
        let line = self.prev.line;
        let arg_count = self.argument_list();
        self.compiler.func.chunk.push_opcode(OpCode::Call, line);
        self.compiler.func.chunk.push_bytes(&[arg_count]);
    }

    pub fn argument_list(&mut self) -> u8 {
        let mut count = 0;
        if self.peek_next() != TokenKind::RightParen {
            loop {
                self.expression();
                if count == 255 {
                    self.log_error(&self.prev, "Can't hvae more than 255 arguments.");
                    self.errors = true;
                    self.panic = true;
                    return 0;
                }
                count += 1;
                if !self.advance_if(TokenKind::Comma) {
                    break;
                }
            }
        }

        self.consume(TokenKind::RightParen, "Expect ')' after argument list.");

        count
    }

    pub fn number(&mut self) {
        match self.prev.data.parse::<f64>() {
            Ok(x) => {
                self.compiler
                    .func
                    .chunk
                    .insert_constant(Value::Float(x), self.prev.line);
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
                self.heap_objects,
            ),
            self.prev.line,
        );
    }

    pub fn named_variable(&mut self, name: &'static str, can_assign: bool) {
        let mut local_idx = self.compiler.resolve_local(name);

        let (get_op, set_op) = match local_idx {
            Some(idx) => {
                if self.compiler.locals[idx as usize].depth == UNINITIALIZED {
                    self.log_error(
                        &self.prev,
                        "Cannot read local variable in its own initializer.",
                    );
                    self.errors = true;
                    self.panic = true;
                }
                (OpCode::ReadLocal, OpCode::WriteLocal)
            }
            None => match self.compiler.resolve_upvalue(name) {
                Some(idx) => {
                    local_idx = Some(idx);
                    (OpCode::ReadUpval, OpCode::WriteUpval)
                }
                None => {
                    local_idx = Some(self.compiler.func.chunk.push_constant(Value::alloc_str(
                        name,
                        self.string_table,
                        self.heap_objects,
                    )));

                    (OpCode::ReadGlobal, OpCode::WriteGlobal)
                }
            },
        };

        let arg = local_idx.unwrap();

        if can_assign && self.advance_if(TokenKind::Eq) {
            self.expression();
            self.compiler.func.chunk.push_opcode(set_op, self.prev.line);
        } else {
            self.compiler.func.chunk.push_opcode(get_op, self.prev.line);
        }

        // if arg > u8::MAX as u16 {
        //     self.compiler.func.chunk.push_bytes(&arg.to_ne_bytes());
        // } else {
        self.compiler.func.chunk.push_bytes(&[arg]);
        // }
    }

    pub fn variable(&mut self, can_assign: bool) {
        self.named_variable(self.prev.data, can_assign);
    }

    pub fn dot(&mut self, can_assign: bool) {
        self.consume(TokenKind::Ident, "Expect property name after '.'.");
        let line = self.prev.line;

        let name = self.compiler.func.chunk.push_constant(Value::alloc_str(
            self.prev.data,
            self.string_table,
            self.heap_objects,
        ));

        if can_assign && self.advance_if(TokenKind::Eq) {
            self.expression();
            self.compiler
                .func
                .chunk
                .push_opcode(OpCode::WriteProperty, line);
        } else if self.advance_if(TokenKind::LeftParen) {
            let arg_count = self.argument_list();
            self.compiler
                .func
                .chunk
                .push_opcode(OpCode::Invoke, self.prev.line);
            self.compiler.func.chunk.push_bytes(&[name, arg_count]);
            return;
        } else {
            self.compiler
                .func
                .chunk
                .push_opcode(OpCode::ReadProperty, line);
        }

        self.compiler.func.chunk.push_bytes(&[name])
    }
}

#[derive(Debug, Clone)]
pub struct Local {
    name: Token,
    depth: u32,
    captured: bool,
}

impl Default for Local {
    fn default() -> Self {
        Self {
            name: Token {
                kind: TokenKind::Error,
                data: "",
                line: 0,
            },
            depth: UNINITIALIZED,
            captured: false,
        }
    }
}

pub const MAX_LOCALS: usize = 256;
pub const MAX_UPVAL: usize = 256;
pub const GLOBAL_SCOPE: u32 = 0;
pub const UNINITIALIZED: u32 = u32::MAX;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FuncKind {
    Func,
    Method,
    Initializer,
    Script,
}

#[derive(Debug, Default)]
pub struct CompUpVal {
    idx: u8,
    local: bool,
}

#[derive(Debug)]
pub struct Compiler {
    pub func: &'static mut Function,
    pub kind: FuncKind,
    pub locals: [Local; MAX_LOCALS],
    pub local_count: u32,
    pub upvalues: [CompUpVal; MAX_UPVAL],
    pub upval_count: u32,
    pub scope_depth: u32,
    pub parent: Option<*mut Compiler>,
}

impl Compiler {
    pub fn new(heap_objects: &mut Vec<Value>) -> Self {
        let func = unsafe { Value::alloc_func(heap_objects).as_mut() };
        Self {
            func,
            kind: FuncKind::Script,
            locals: std::array::from_fn(|_| Default::default()),
            local_count: 1,
            upvalues: std::array::from_fn(|_| Default::default()),
            upval_count: 0,
            scope_depth: Default::default(),
            parent: None,
        }
    }

    pub fn global_scope(&self) -> bool {
        self.scope_depth == GLOBAL_SCOPE
    }

    pub fn local_scope(&self) -> bool {
        self.scope_depth > GLOBAL_SCOPE
    }

    pub fn resolve_local(&self, name: &'static str) -> Option<u8> {
        for i in (0..self.local_count as usize).rev() {
            if self.locals[i].name.data == name {
                return Some(i as u8);
            }
        }
        None
        // self.locals[..self.local_count as usize]
        //     .iter()
        //     .enumerate()
        //     .rev()
        //     .find(|x| x.1.name.data == name)
        //     .map(|x| x.0 as u8)
    }

    pub fn resolve_upvalue(&mut self, name: &'static str) -> Option<u8> {
        if let Some(p) = self.parent {
            let p = unsafe { p.as_mut().unwrap() };
            let mut res = p.resolve_local(name);

            let local = if let Some(i) = res {
                p.locals[i as usize].captured = true;
                true
            } else {
                false
            };

            res = res.or_else(|| p.resolve_upvalue(name));

            return res.map(|x| self.add_upvalue(x as u8, local));
        }

        None
    }

    pub fn add_upvalue(&mut self, idx: u8, local: bool) -> u8 {
        let count = self.func.upval_count as usize;

        match self.upvalues[..count]
            .iter()
            .position(|x| x.idx == idx && x.local == local)
        {
            Some(v) => v as u8,
            None => {
                self.upvalues[count] = CompUpVal { idx, local };

                // todo there's a better way to handle this but it's so rare i'm putting it off
                if count + 1 == MAX_UPVAL {
                    panic!("too many closure variables");
                }
                self.func.upval_count += 1;

                count as u8
            }
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct ClassCompiler {
    pub parent: Option<*mut ClassCompiler>,
    pub has_superclass: bool,
}
