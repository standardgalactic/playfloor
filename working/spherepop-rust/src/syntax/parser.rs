use crate::core::event::{CollapseRule, RefusalReason, Symbol};
use crate::syntax::ast::Term;
use crate::types::ty::Type;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Ident(String), Number(u32),
    Lambda, Arrow, ThinArrow, Dot, Colon, LParen, RParen,
    LBracket, RBracket, Semi, Eq,
    KwLet, KwIn, KwPop, KwRefuse, KwCollapse, KwBind, KwSeq,
    KwType, KwPi, KwAdmissible, KwProcess, KwUnit, KwNever,
    // Collapse rule keywords
    KwLastWrite, KwAccumulate, KwProj,
    // Refusal reason keywords
    KwViolation, KwExplicit,
    Eof,
}

pub struct Lexer { input: Vec<char>, pos: usize }

impl Lexer {
    pub fn new(src: &str) -> Self { Self { input: src.chars().collect(), pos: 0 } }

    fn peek(&self) -> Option<char> { self.input.get(self.pos).copied() }

    fn advance(&mut self) -> Option<char> {
        let c = self.input.get(self.pos).copied();
        self.pos += 1;
        c
    }

    fn skip_ws(&mut self) {
        loop {
            while self.peek().map_or(false, |c| c.is_whitespace()) { self.advance(); }
            if self.input.get(self.pos..self.pos+2) == Some(&['-','-']) {
                while self.peek().map_or(false, |c| c != '\n') { self.advance(); }
            } else { break; }
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            self.skip_ws();
            match self.peek() {
                None => { tokens.push(Token::Eof); break; }
                Some(c) => {
                    let tok = match c {
                        '(' => { self.advance(); Token::LParen }
                        ')' => { self.advance(); Token::RParen }
                        '[' => { self.advance(); Token::LBracket }
                        ']' => { self.advance(); Token::RBracket }
                        '.' => { self.advance(); Token::Dot }
                        ':' => { self.advance(); Token::Colon }
                        ';' => { self.advance(); Token::Semi }
                        '=' => { self.advance(); Token::Eq }
                        'λ' | '\\' => { self.advance(); Token::Lambda }
                        '-' => {
                            self.advance();
                            if self.peek() == Some('>') { self.advance(); Token::Arrow }
                            else { Token::Ident("-".into()) }
                        }
                        '~' => {
                            self.advance();
                            if self.peek() == Some('>') { self.advance(); Token::ThinArrow }
                            else { Token::Ident("~".into()) }
                        }
                        c if c.is_ascii_digit() => {
                            let mut s = String::new();
                            while self.peek().map_or(false, |d| d.is_ascii_digit()) {
                                s.push(self.advance().unwrap());
                            }
                            Token::Number(s.parse().unwrap_or(0))
                        }
                        c if c.is_alphabetic() || c == '_' => {
                            let mut s = String::new();
                            while self.peek().map_or(false, |d| d.is_alphanumeric() || d == '_') {
                                s.push(self.advance().unwrap());
                            }
                            match s.as_str() {
                                "let" => Token::KwLet, "in" => Token::KwIn,
                                "pop" => Token::KwPop, "refuse" => Token::KwRefuse,
                                "collapse" => Token::KwCollapse, "bind" => Token::KwBind,
                                "seq" => Token::KwSeq, "Type" => Token::KwType,
                                "Pi" => Token::KwPi, "Admissible" => Token::KwAdmissible,
                                "Process" => Token::KwProcess, "Unit" => Token::KwUnit,
                                "Never" => Token::KwNever,
                                "last_write" => Token::KwLastWrite,
                                "accumulate" => Token::KwAccumulate,
                                "proj" => Token::KwProj,
                                "violation" => Token::KwViolation,
                                "explicit" => Token::KwExplicit,
                                _ => Token::Ident(s),
                            }
                        }
                        _ => { self.advance(); continue; }
                    };
                    tokens.push(tok);
                }
            }
        }
        tokens
    }
}

pub struct Parser { tokens: Vec<Token>, pos: usize }

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self { Self { tokens, pos: 0 } }

    fn peek(&self) -> &Token { self.tokens.get(self.pos).unwrap_or(&Token::Eof) }

    fn advance(&mut self) -> Token {
        let t = self.tokens.get(self.pos).cloned().unwrap_or(Token::Eof);
        if self.pos < self.tokens.len() { self.pos += 1; }
        t
    }

    fn expect_ident(&mut self) -> Result<Symbol, String> {
        match self.advance() {
            Token::Ident(s) => Ok(Symbol::new(s)),
            other => Err(format!("expected identifier, got {:?}", other)),
        }
    }

    /// Parse a collapse rule (after the `collapse` keyword or `via`).
    fn parse_collapse_rule(&mut self) -> CollapseRule {
        match self.peek() {
            Token::KwLastWrite => { self.advance(); CollapseRule::LastWrite }
            Token::KwAccumulate => { self.advance(); CollapseRule::Accumulate }
            Token::KwProj => {
                self.advance();
                if self.peek() == &Token::LParen { self.advance(); }
                let label = if let Token::Ident(_) = self.peek() {
                    self.expect_ident().unwrap_or(Symbol::new("unnamed"))
                } else { Symbol::new("unnamed") };
                if self.peek() == &Token::RParen { self.advance(); }
                CollapseRule::Projection(label)
            }
            Token::Ident(_) => {
                let name = self.expect_ident().unwrap_or(Symbol::new("id"));
                if name.0 == "id" { CollapseRule::Identity }
                else { CollapseRule::Named(name) }
            }
            _ => CollapseRule::Identity,
        }
    }

    /// Parse a refusal reason (after `because` / `b/c` / just a string).
    fn parse_refusal_reason(&mut self) -> RefusalReason {
        match self.peek() {
            Token::KwViolation => {
                self.advance();
                if self.peek() == &Token::LParen { self.advance(); }
                let name = self.expect_ident().unwrap_or(Symbol::new("constraint"));
                if self.peek() == &Token::RParen { self.advance(); }
                RefusalReason::ConstraintViolation(name)
            }
            Token::KwExplicit => {
                self.advance();
                if self.peek() == &Token::LParen { self.advance(); }
                let msg = match self.peek() {
                    Token::Ident(_) => {
                        let s = self.expect_ident().unwrap_or(Symbol::new("msg"));
                        s.0
                    }
                    _ => "unspecified".to_string(),
                };
                if self.peek() == &Token::RParen { self.advance(); }
                RefusalReason::Explicit(msg)
            }
            _ => RefusalReason::Explicit("unspecified".into()),
        }
    }

    pub fn parse_type(&mut self) -> Result<Type, String> {
        match self.peek() {
            Token::KwUnit => { self.advance(); Ok(Type::Unit) }
            Token::KwNever => { self.advance(); Ok(Type::Never) }
            Token::KwType => {
                self.advance();
                match self.peek() {
                    Token::Number(n) => { let n = *n; self.advance(); Ok(Type::Universe(n)) }
                    _ => Ok(Type::Universe(0)),
                }
            }
            Token::KwAdmissible => {
                self.advance();
                if self.peek() == &Token::LParen { self.advance(); }
                let t = self.parse_type()?;
                if self.peek() == &Token::RParen { self.advance(); }
                Ok(Type::Admissible(Box::new(t)))
            }
            Token::Ident(_) => {
                let name = self.expect_ident()?;
                if self.peek() == &Token::Arrow {
                    self.advance();
                    let rhs = self.parse_type()?;
                    Ok(Type::Pi {
                        param: Symbol::new("_"),
                        param_ty: Box::new(Type::Var(name)),
                        body_ty: Box::new(rhs),
                    })
                } else { Ok(Type::Var(name)) }
            }
            Token::LParen => {
                self.advance();
                let t = self.parse_type()?;
                if self.peek() == &Token::Arrow {
                    self.advance();
                    let rhs = self.parse_type()?;
                    if self.peek() == &Token::RParen { self.advance(); }
                    Ok(Type::Pi {
                        param: Symbol::new("_"),
                        param_ty: Box::new(t),
                        body_ty: Box::new(rhs),
                    })
                } else {
                    if self.peek() == &Token::RParen { self.advance(); }
                    Ok(t)
                }
            }
            other => Err(format!("unexpected token in type: {:?}", other)),
        }
    }

    pub fn parse_term(&mut self) -> Result<Term, String> {
        match self.peek() {
            Token::Lambda => {
                self.advance();
                let param = self.expect_ident()?;
                let ty = if self.peek() == &Token::Colon { self.advance(); self.parse_type()? }
                         else { Type::Unit };
                if matches!(self.peek(), Token::Arrow | Token::Dot) { self.advance(); }
                let body = self.parse_term()?;
                Ok(Term::lam(param.0, ty, body))
            }

            Token::KwLet => {
                self.advance();
                let name = self.expect_ident()?;
                if self.peek() == &Token::Eq { self.advance(); }
                let value = self.parse_term()?;
                if self.peek() == &Token::KwIn { self.advance(); }
                let body = self.parse_term()?;
                Ok(Term::let_in(name.0, value, body))
            }

            Token::KwPop => { self.advance(); Ok(Term::pop(self.parse_atom()?)) }

            Token::KwRefuse => {
                self.advance();
                let inner = self.parse_atom()?;
                // Optional reason: refuse x violation(some_constraint)
                let reason = self.parse_refusal_reason();
                Ok(Term::refuse(inner, reason))
            }

            Token::KwCollapse => {
                self.advance();
                let inner = self.parse_atom()?;
                // Optional rule: collapse x last_write
                let rule = self.parse_collapse_rule();
                Ok(Term::collapse(inner, rule))
            }

            Token::KwBind => {
                self.advance();
                let lhs = self.parse_atom()?;
                let rhs = self.parse_atom()?;
                Ok(Term::bind(lhs, rhs))
            }

            Token::KwSeq => {
                self.advance();
                if self.peek() == &Token::LBracket { self.advance(); }
                let mut terms = Vec::new();
                loop {
                    if matches!(self.peek(), Token::RBracket | Token::Eof) { break; }
                    terms.push(self.parse_term()?);
                    if self.peek() == &Token::Semi { self.advance(); }
                }
                if self.peek() == &Token::RBracket { self.advance(); }
                Ok(Term::seq(terms))
            }

            _ => self.parse_app(),
        }
    }

    fn parse_app(&mut self) -> Result<Term, String> {
        let mut t = self.parse_atom()?;
        loop {
            match self.peek() {
                Token::LParen | Token::Ident(_) | Token::KwType | Token::KwUnit => {
                    let arg = self.parse_atom()?;
                    t = Term::app(t, arg);
                }
                _ => break,
            }
        }
        Ok(t)
    }

    fn parse_atom(&mut self) -> Result<Term, String> {
        match self.peek() {
            Token::Ident(_) => Ok(Term::var(self.expect_ident()?.0)),
            Token::KwType => { self.advance(); Ok(Term::Universe(0)) }
            Token::KwUnit => { self.advance(); Ok(Term::var("unit")) }
            Token::Number(n) => { let n = *n; self.advance(); Ok(Term::Universe(n)) }
            Token::LParen => {
                self.advance();
                let t = self.parse_term()?;
                let result = if self.peek() == &Token::Colon {
                    self.advance();
                    let ty = self.parse_type()?;
                    Term::ann(t, ty)
                } else { t };
                if self.peek() == &Token::RParen { self.advance(); }
                Ok(result)
            }
            other => Err(format!("unexpected token in atom: {:?}", other)),
        }
    }
}

pub fn parse(src: &str) -> Result<Term, String> {
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    parser.parse_term()
}
