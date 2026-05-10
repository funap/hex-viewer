use std::collections::HashMap;

pub struct ExprEvaluator;

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(i64),
    Identifier(String),
    String(String),
    Plus, Minus, Star, Slash, Percent,
    Equal, NotEqual, GreaterEqual, LessEqual, Greater, Less,
    And, Or, Amp, Pipe, Bang,
    LParen, RParen, Dot,
    EOF,
}

struct Lexer<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn next_token(&mut self) -> Token {
        self.skip_whitespace();
        if self.pos >= self.input.len() {
            return Token::EOF;
        }

        let ch = self.current_char();
        if ch.is_ascii_digit() {
            return self.lex_number();
        }
        if ch.is_alphabetic() || ch == '_' {
            return self.lex_identifier();
        }
        if ch == '\'' || ch == '"' {
            return self.lex_string();
        }

        self.pos += 1;
        match ch {
            '+' => Token::Plus,
            '-' => Token::Minus,
            '*' => Token::Star,
            '/' => Token::Slash,
            '%' => Token::Percent,
            '(' => Token::LParen,
            ')' => Token::RParen,
            '.' => Token::Dot,
            '!' => {
                if self.current_char() == '=' {
                    self.pos += 1;
                    Token::NotEqual
                } else {
                    Token::Bang
                }
            }
            '=' => {
                if self.current_char() == '=' {
                    self.pos += 1;
                    Token::Equal
                } else {
                    Token::Equal // Default to equal for single = too?
                }
            }
            '>' => {
                if self.current_char() == '=' {
                    self.pos += 1;
                    Token::GreaterEqual
                } else {
                    Token::Greater
                }
            }
            '<' => {
                if self.current_char() == '=' {
                    self.pos += 1;
                    Token::LessEqual
                } else {
                    Token::Less
                }
            }
            '&' => {
                if self.current_char() == '&' {
                    self.pos += 1;
                    Token::And
                } else {
                    Token::Amp
                }
            }
            '|' => {
                if self.current_char() == '|' {
                    self.pos += 1;
                    Token::Or
                } else {
                    Token::Pipe
                }
            }
            _ => Token::EOF,
        }
    }

    fn current_char(&self) -> char {
        self.input[self.pos..].chars().next().unwrap_or('\0')
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() && self.current_char().is_whitespace() {
            self.pos += 1;
        }
    }

    fn lex_number(&mut self) -> Token {
        let start = self.pos;
        if self.current_char() == '0' {
            self.pos += 1;
            if self.current_char() == 'x' || self.current_char() == 'X' {
                self.pos += 1;
                let hex_start = self.pos;
                while self.pos < self.input.len() && self.current_char().is_ascii_hexdigit() {
                    self.pos += 1;
                }
                let val = i64::from_str_radix(&self.input[hex_start..self.pos], 16).unwrap_or(0);
                return Token::Number(val);
            }
        }
        while self.pos < self.input.len() && self.current_char().is_ascii_digit() {
            self.pos += 1;
        }
        let val = self.input[start..self.pos].parse().unwrap_or(0);
        Token::Number(val)
    }

    fn lex_identifier(&mut self) -> Token {
        let start = self.pos;
        while self.pos < self.input.len() && (self.current_char().is_alphanumeric() || self.current_char() == '_') {
            self.pos += 1;
        }
        Token::Identifier(self.input[start..self.pos].to_string())
    }

    fn lex_string(&mut self) -> Token {
        let quote = self.current_char();
        self.pos += 1;
        let start = self.pos;
        while self.pos < self.input.len() && self.current_char() != quote {
            self.pos += 1;
        }
        let s = self.input[start..self.pos].to_string();
        if self.pos < self.input.len() {
            self.pos += 1;
        }
        Token::String(s)
    }
}

impl ExprEvaluator {
    pub fn evaluate(expr: &str, context: &HashMap<String, i64>, base_path: &[String]) -> i64 {
        let mut parser = Parser::new(expr, context, base_path);
        parser.parse_expr()
    }

    pub fn evaluate_bool(expr: &str, context: &HashMap<String, i64>, base_path: &[String]) -> bool {
        Self::evaluate(expr, context, base_path) != 0
    }
}

struct Parser<'a> {
    lexer: Lexer<'a>,
    current_token: Token,
    context: &'a HashMap<String, i64>,
    base_path: &'a [String],
}

impl<'a> Parser<'a> {
    fn new(input: &'a str, context: &'a HashMap<String, i64>, base_path: &'a [String]) -> Self {
        let mut lexer = Lexer::new(input);
        let current_token = lexer.next_token();
        Self { lexer, current_token, context, base_path }
    }

    fn advance(&mut self) {
        self.current_token = self.lexer.next_token();
    }

    fn parse_expr(&mut self) -> i64 {
        let mut val = self.parse_comparison();
        while matches!(self.current_token, Token::And | Token::Or) {
            let op = self.current_token.clone();
            self.advance();
            let right = self.parse_comparison();
            match op {
                Token::And => val = if val != 0 && right != 0 { 1 } else { 0 },
                Token::Or => val = if val != 0 || right != 0 { 1 } else { 0 },
                _ => {}
            }
        }
        val
    }

    fn parse_comparison(&mut self) -> i64 {
        let mut val = self.parse_term();
        while matches!(self.current_token, Token::Equal | Token::NotEqual | Token::Greater | Token::GreaterEqual | Token::Less | Token::LessEqual) {
            let op = self.current_token.clone();
            self.advance();
            let right = self.parse_term();
            match op {
                Token::Equal => val = if val == right { 1 } else { 0 },
                Token::NotEqual => val = if val != right { 1 } else { 0 },
                Token::Greater => val = if val > right { 1 } else { 0 },
                Token::GreaterEqual => val = if val >= right { 1 } else { 0 },
                Token::Less => val = if val < right { 1 } else { 0 },
                Token::LessEqual => val = if val <= right { 1 } else { 0 },
                _ => {}
            }
        }
        val
    }

    fn parse_term(&mut self) -> i64 {
        let mut val = self.parse_factor();
        while matches!(self.current_token, Token::Plus | Token::Minus) {
            let op = self.current_token.clone();
            self.advance();
            let right = self.parse_factor();
            match op {
                Token::Plus => val += right,
                Token::Minus => val -= right,
                _ => {}
            }
        }
        val
    }

    fn parse_factor(&mut self) -> i64 {
        let mut val = self.parse_primary();
        while matches!(self.current_token, Token::Star | Token::Slash | Token::Percent) {
            let op = self.current_token.clone();
            self.advance();
            let right = self.parse_primary();
            match op {
                Token::Star => val *= right,
                Token::Slash => val = if right != 0 { val / right } else { 0 },
                Token::Percent => val = if right != 0 { val % right } else { 0 },
                _ => {}
            }
        }
        val
    }

    fn parse_primary(&mut self) -> i64 {
        match self.current_token.clone() {
            Token::Number(n) => {
                self.advance();
                n
            }
            Token::Identifier(id) => {
                self.advance();
                
                let mut path_parts;
                let current_id = id;
                
                if current_id == "_root" {
                    path_parts = Vec::new();
                } else if current_id == "_parent" {
                    let mut p = self.base_path.to_vec();
                    if !p.is_empty() { p.pop(); }
                    path_parts = p;
                } else {
                    path_parts = self.base_path.to_vec();
                    path_parts.push(current_id);
                }

                while self.current_token == Token::Dot {
                    self.advance();
                    if let Token::Identifier(sub) = &self.current_token {
                        if sub == "_parent" {
                            if !path_parts.is_empty() { path_parts.pop(); }
                        } else if sub == "_root" {
                            path_parts = Vec::new();
                        } else {
                            path_parts.push(sub.clone());
                        }
                        self.advance();
                    } else {
                        break;
                    }
                }
                
                // Try to resolve the path
                // First try absolute path from parts
                let full_id = path_parts.join(".");
                if let Some(val) = self.context.get(&full_id) {
                    return *val;
                }
                
                // If not found, and it was a simple identifier (no dots),
                // try to find it as a sibling of the current base_path
                if path_parts.len() == 1 {
                    let mut sibling_path = self.base_path.to_vec();
                    sibling_path.push(path_parts[0].clone());
                    let sibling_id = sibling_path.join(".");
                    if let Some(val) = self.context.get(&sibling_id) {
                        return *val;
                    }
                }

                // If not found, try as a simple global identifier (top-level sibling)
                if let Some(val) = self.context.get(&path_parts.last().unwrap_or(&"".to_string()).clone()) {
                    return *val;
                }

                0
            }
            Token::String(s) => {
                self.advance();
                let mut bytes = [0u8; 8];
                let s_bytes = s.as_bytes();
                let len = s_bytes.len().min(8);
                bytes[..len].copy_from_slice(&s_bytes[..len]);
                i64::from_le_bytes(bytes)
            }
            Token::LParen => {
                self.advance();
                let val = self.parse_expr();
                if self.current_token == Token::RParen {
                    self.advance();
                }
                val
            }
            Token::Bang => {
                self.advance();
                if self.parse_primary() == 0 { 1 } else { 0 }
            }
            _ => {
                self.advance();
                0
            }
        }
    }
}
