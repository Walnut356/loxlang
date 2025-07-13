use std::rc::Rc;

use crate::compiler::Precedence;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Semicolon,

    // keep the unary and binary operators together for efficiency in the compiler
    Bang,
    Minus,
    Plus,
    Slash,
    Star,

    Eq,
    Gt,
    Lt,

    NotEq,
    EqEq,
    GtEq,
    LtEq,
    And,
    Or,

    Ident,
    String,
    Number,
    False,
    Nil,
    This,
    True,

    Class,
    Else,
    For,
    Fun,
    If,
    Print,
    Return,
    Super,
    Var,
    While,

    // misc
    Error,
    EOF,
}

impl TokenKind {
    pub const fn precedence(&self) -> Precedence {
        use Precedence as P;
        match self {
            TokenKind::Minus => P::Term,
            TokenKind::Plus => P::Term,
            TokenKind::Slash => P::Factor,
            TokenKind::Star => P::Factor,
            _ => P::None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub data: &'static str,
    pub line: u32,
}

#[derive(Debug, Clone)]
pub struct Scanner {
    pub source: Rc<str>,
    pub start: usize,
    pub pos: usize,
    pub line: u32,
}

impl Scanner {
    pub fn new(source: Rc<str>) -> Self {
        Self {
            source,
            start: 0,
            pos: 0,
            line: 1,
        }
    }

    fn new_token(&self, kind: TokenKind) -> Token {
        Token {
            kind,
            // safety: this should be fine? the Rc lifetime is "bound" to the scanner, which is
            // bound to the lifetime of the Parser, which is the only consumer of tokens.
            data: unsafe {
                (&raw const self.source[self.start..self.pos])
                    .as_ref()
                    .unwrap()
            },
            line: self.line,
        }
    }

    fn new_error(&self, message: &'static str) -> Token {
        Token {
            kind: TokenKind::Error,
            data: message,
            line: self.line,
        }
    }

    fn at_eof(&self) -> bool {
        self.pos >= self.source.len()
    }

    fn peek(&self) -> u8 {
        self.source.as_bytes()[self.pos]
    }

    fn peek_byte(&self, n: usize) -> u8 {
        self.source.as_bytes()[n]
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.source.len() {
            match self.peek() {
                b'\n' => {
                    self.line += 1;
                }
                b'\t' | b' ' | b'\r' => (),
                b'/' if self.pos + 1 < self.source.len()
                    && self.peek_byte(self.pos - 1) == b'/' =>
                {
                    self.pos += 2;
                    while !self.at_eof() && self.peek_byte(self.pos) != b'\n' {
                        self.pos += 1;
                    }
                }
                _ => return,
            }

            self.pos += 1;
        }
    }

    fn read(&mut self) -> u8 {
        self.pos += 1;
        self.source.as_bytes()[self.pos - 1]
    }

    fn read_if(&mut self, char: u8) -> bool {
        if !self.at_eof() && self.source.as_bytes()[self.pos] == char {
            self.pos += 1;
            return true;
        }

        false
    }

    fn consume_while<T: Fn(&u8) -> bool>(&mut self, cond: T) {
        while !self.at_eof() {
            let c = self.peek();
            if !cond(&c) {
                return;
            }
            if c == b'\n' {
                self.line += 1;
            }
            self.pos += 1;
        }
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();

        if self.at_eof() {
            return self.new_token(TokenKind::EOF);
        }

        self.start = self.pos;

        let c = self.read();

        match c {
            b'(' => self.new_token(TokenKind::LeftParen),
            b')' => self.new_token(TokenKind::RightParen),
            b'{' => self.new_token(TokenKind::LeftBrace),
            b'}' => self.new_token(TokenKind::RightBrace),
            b';' => self.new_token(TokenKind::Semicolon),
            b',' => self.new_token(TokenKind::Comma),
            b'.' => self.new_token(TokenKind::Dot),
            b'-' => self.new_token(TokenKind::Minus),
            b'+' => self.new_token(TokenKind::Plus),
            b'/' => self.new_token(TokenKind::Slash),
            b'*' => self.new_token(TokenKind::Star),
            b'!' => {
                if self.read_if(b'=') {
                    self.new_token(TokenKind::NotEq)
                } else {
                    self.new_token(TokenKind::Bang)
                }
            }
            b'=' => {
                if self.read_if(b'=') {
                    self.new_token(TokenKind::EqEq)
                } else {
                    self.new_token(TokenKind::Eq)
                }
            }
            b'>' => {
                if self.read_if(b'=') {
                    self.new_token(TokenKind::GtEq)
                } else {
                    self.new_token(TokenKind::Gt)
                }
            }
            b'<' => {
                if self.read_if(b'=') {
                    self.new_token(TokenKind::LtEq)
                } else {
                    self.new_token(TokenKind::Lt)
                }
            }
            b'"' => {
                self.consume_while(|c| *c != b'"');
                if self.at_eof() {
                    self.new_error("Unterminated String")
                } else {
                    self.new_token(TokenKind::String)
                }
            }
            c if c.is_ascii_digit() => {
                self.consume_while(u8::is_ascii_digit);
                if !self.at_eof() && self.peek() == b'.' {
                    self.consume_while(u8::is_ascii_digit);
                }

                self.new_token(TokenKind::Number)
            }
            c if c.is_ascii_alphabetic() => {
                self.consume_while(|c| c.is_ascii_alphanumeric() || *c == b'_');

                let mut token = self.new_token(TokenKind::Ident);

                match token.data.as_bytes()[0] {
                    b'a' => {
                        if &token.data[1..] == "nd" {
                            token.kind = TokenKind::And
                        }
                    }
                    b'c' => {
                        if &token.data[1..] == "lass" {
                            token.kind = TokenKind::Class
                        }
                    }
                    b'e' => {
                        if &token.data[1..] == "lse" {
                            token.kind = TokenKind::Else
                        }
                    }
                    b'i' => {
                        if &token.data[1..] == "f" {
                            token.kind = TokenKind::If
                        }
                    }
                    b'n' => {
                        if &token.data[1..] == "il" {
                            token.kind = TokenKind::Nil
                        }
                    }
                    b'o' => {
                        if &token.data[1..] == "r" {
                            token.kind = TokenKind::Or
                        }
                    }
                    b'p' => {
                        if &token.data[1..] == "rint" {
                            token.kind = TokenKind::Print
                        }
                    }
                    b'r' => {
                        if &token.data[1..] == "eturn" {
                            token.kind = TokenKind::Return
                        }
                    }
                    b's' => {
                        if &token.data[1..] == "uper" {
                            token.kind = TokenKind::Super
                        }
                    }
                    b'v' => {
                        if &token.data[1..] == "ar" {
                            token.kind = TokenKind::Var
                        }
                    }
                    b'w' => {
                        if &token.data[1..] == "hile" {
                            token.kind = TokenKind::While
                        }
                    }
                    b'f' if token.data.len() > 1 => match token.data.as_bytes()[1] {
                        b'a' => {
                            if &token.data[1..] == "lse" {
                                token.kind = TokenKind::False
                            }
                        }
                        b'o' => {
                            if &token.data[1..] == "r" {
                                token.kind = TokenKind::For
                            }
                        }
                        b'u' => {
                            if &token.data[1..] == "n" {
                                token.kind = TokenKind::Fun
                            }
                        }
                        _ => (),
                    },
                    b't' if token.data.len() > 1 => match token.data.as_bytes()[1] {
                        b'h' => {
                            if &token.data[1..] == "is" {
                                token.kind = TokenKind::This
                            }
                        }
                        b'r' => {
                            if &token.data[1..] == "ue" {
                                token.kind = TokenKind::True
                            }
                        }
                        _ => (),
                    },
                    _ => (),
                }

                token
            }
            _ => todo!(),
        }
    }
}
