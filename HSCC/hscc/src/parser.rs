use crate::ast::*;
use crate::lexer::{Token, TokenKind};
use alloc::vec::Vec;
use anyhow::{bail, Result};
use core::option::Option;
use core::prelude::v1::Ok;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn consume(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.pos).cloned();
        self.pos += 1;
        token
    }

    fn expect(&mut self, kind: TokenKind) -> Result<Token> {
        if let Some(token) = self.peek() {
            if token.kind == kind {
                return Ok(self.consume().unwrap());
            }
        }
        bail!("Expected {:?} at {}:{}", kind, self.peek().unwrap().line, self.peek().unwrap().column);
    }

    pub(crate) fn parse_program(&mut self) -> Result<Program> {
        let mut functions = Vec::new();
        let mut tasks = Vec::new();
        while self.peek().is_some() && self.peek().unwrap().kind != TokenKind::Eof {
            if let Some(token) = self.peek() {
                match token.kind {
                    TokenKind::Fn => functions.push(self.parse_function()?),
                    TokenKind::Task => tasks.push(self.parse_task()?),
                    _ => bail!("Unexpected token at start of declaration"),
                }
            } else {
                break;
            }
        }
        Ok(Program { functions, tasks })
    }

    fn parse_function(&mut self) -> Result<Function> {
        self.expect(TokenKind::Fn)?;
        let name = if let TokenKind::Identifier(id) = self.consume().unwrap().kind {
            id
        } else {
            bail!("Expected function name");
        };
        self.expect(TokenKind::LParen)?;
        let params = self.parse_param_list()?;
        self.expect(TokenKind::RParen)?;
        let return_type = if self.peek().map(|t| t.kind == TokenKind::Arrow).unwrap_or(false) {
            self.consume();
            Some(self.parse_type()?)
        } else {
            None
        };
        let body = self.parse_block()?;
        Ok(Function { name, params, return_type, body })
    }

    fn parse_task(&mut self) -> Result<Task> {
        self.expect(TokenKind::Task)?;
        let name = if let TokenKind::Identifier(id) = self.consume().unwrap().kind {
            id
        } else {
            bail!("Expected task name");
        };
        if self.peek().map(|t| t.kind == TokenKind::Colon).unwrap_or(false) {
            self.consume();
            self.parse_type()?;
        }
        self.expect(TokenKind::LBrace)?;
        let mut pattern = None;
        let mut policy = None;
        let mut body = None;
        while self.peek().is_some() && self.peek().unwrap().kind != TokenKind::RBrace {
            match self.peek().unwrap().kind {
                TokenKind::Pattern => {
                    self.consume();
                    self.expect(TokenKind::Colon)?;
                    pattern = Some(self.parse_pattern()?);
                    if self.peek().map(|t| t.kind == TokenKind::Comma).unwrap_or(false) {
                        self.consume();
                    }
                }
                TokenKind::Policy => {
                    self.consume();
                    self.expect(TokenKind::Colon)?;
                    policy = Some(self.parse_policy()?);
                    if self.peek().map(|t| t.kind == TokenKind::Comma).unwrap_or(false) {
                        self.consume();
                    }
                }
                TokenKind::Body => {
                    self.consume();
                    self.expect(TokenKind::LParen)?;
                    let params = self.parse_param_list()?;
                    self.expect(TokenKind::RParen)?;
                    let return_type = if self.peek().map(|t| t.kind == TokenKind::Arrow).unwrap_or(false) {
                        self.consume();
                        Some(self.parse_type()?)
                    } else {
                        None
                    };
                    let block = self.parse_block()?;
                    body = Some((params, return_type, block));
                }
                _ => bail!("Unexpected token in task body"),
            }
        }
        self.expect(TokenKind::RBrace)?;
        let (params, return_type, body) = body.ok_or_else(|| anyhow::anyhow!("Task body missing"))?;
        Ok(Task { name, pattern, policy, params, return_type, body })
    }

    fn parse_pattern(&mut self) -> Result<Pattern> {
        let kind = if let TokenKind::Identifier(id) = self.consume().unwrap().kind {
            id
        } else {
            bail!("Expected pattern kind");
        };
        self.expect(TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while self.peek().is_some() && self.peek().unwrap().kind != TokenKind::RBrace {
            let name = if let TokenKind::Identifier(id) = self.consume().unwrap().kind {
                id
            } else {
                bail!("Expected field name");
            };
            self.expect(TokenKind::Colon)?;
            let value = self.parse_expression()?;
            fields.push((name, value));
            if self.peek().map(|t| t.kind == TokenKind::Comma).unwrap_or(false) {
                self.consume();
            }
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Pattern { kind, fields })
    }

    fn parse_policy(&mut self) -> Result<Policy> {
        todo!()
    }

    fn parse_param_list(&mut self) -> Result<Vec<Param>> {
        let mut params = Vec::new();
        while self.peek().is_some() && self.peek().unwrap().kind != TokenKind::RParen {
            let name = if let TokenKind::Identifier(id) = self.consume().unwrap().kind {
                id
            } else {
                bail!("Expected parameter name");
            };
            self.expect(TokenKind::Colon)?;
            let ty = self.parse_type()?;
            params.push(Param { name, ty });
            if self.peek().map(|t| t.kind == TokenKind::Comma).unwrap_or(false) {
                self.consume();
            } else {
                break;
            }
        }
        Ok(params)
    }

    fn parse_type(&mut self) -> Result<Type> {
        match self.peek().unwrap().kind {
            TokenKind::I32 => { self.consume(); Ok(Type::I32) }
            TokenKind::F32 => { self.consume(); Ok(Type::F32) }
            TokenKind::Bool => { self.consume(); Ok(Type::Bool) }
            TokenKind::Buffer => {
                self.consume();
                self.expect(TokenKind::Lt)?;
                let inner = self.parse_type()?;
                let dims = if self.peek().map(|t| t.kind == TokenKind::Comma).unwrap_or(false) {
                    self.consume();
                    // 解析整数纬度
                    if let TokenKind::Integer(s) = &self.peek().unwrap().kind {
                        let dim = s.parse().ok();
                        self.consume();
                        dim
                    } else {
                        None
                    }
                } else {
                    None
                };
                self.expect(TokenKind::Gt)?;
                Ok(Type::Buffer(Box::new(inner), dims))
            }
            _ => {
                if let TokenKind::Identifier(id) = self.consume().unwrap().kind {
                    Ok(Type::Named(id))
                } else {
                    bail!("Expected type")
                }
            }
        }
    }

    fn parse_block(&mut self) -> Result<Block> {
        self.expect(TokenKind::LBrace)?;
        let mut statements = Vec::new();
        while self.peek().is_some() && self.peek().unwrap().kind != TokenKind::RBrace {
            statements.push(self.parse_statement()?);
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Block { statements })
    }

    fn parse_statement(&mut self) -> Result<Statement> {
        match self.peek().unwrap().kind {
            TokenKind::Let => self.parse_let_statement(),
            TokenKind::Return => self.parse_return_statement(),
            TokenKind::Spawn => self.parse_spawn_statement(),
            TokenKind::Parallel => self.parse_parallel_for(),
            _ => {
                let expr = self.parse_expression()?;
                self.expect(TokenKind::Semicolon)?;
                Ok(Statement::Expr(expr))
            }
        }
    }

    fn parse_let_statement(&mut self) -> Result<Statement> {
        self.expect(TokenKind::Let)?;
        let mutable = if self.peek().map(|t| t.kind == TokenKind::Mut).unwrap_or(false) {
            self.consume();
            true
        } else {
            false
        };
        let name = if let TokenKind::Identifier(id) = self.consume().unwrap().kind {
            id
        } else {
            bail!("Expected variable name");
        };
        let ty = if self.peek().map(|t| t.kind == TokenKind::Colon).unwrap_or(false) {
            self.consume();
            Some(self.parse_type()?)
        } else {
            None
        };
        let init = if self.peek().map(|t| t.kind == TokenKind::Eq).unwrap_or(false) {
            self.consume();
            Some(self.parse_expression()?)
        } else {
            None
        };
        self.expect(TokenKind::Semicolon)?;
        Ok(Statement::Let { mutable, name, ty, init })
    }

    fn parse_return_statement(&mut self) -> Result<Statement> {
        self.expect(TokenKind::Return)?;
        let expr = if self.peek().map(|t| t.kind != TokenKind::Semicolon).unwrap_or(false) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        self.expect(TokenKind::Semicolon)?;
        Ok(Statement::Return(expr))
    }

    fn parse_spawn_statement(&mut self) -> Result<Statement> {
        self.expect(TokenKind::Spawn)?;
        let device = if self.peek().map(|t| t.kind == TokenKind::On).unwrap_or(false) {
            self.consume();
            Some(self.parse_expression()?)
        } else {
            None
        };
        let task = self.parse_expression()?;
        let await_ = if self.peek().map(|t| t.kind == TokenKind::Dot).unwrap_or(false) {
            self.consume();
            self.expect(TokenKind::Await)?;
            true
        } else {
            false
        };
        self.expect(TokenKind::Semicolon)?;
        Ok(Statement::Spawn { device, task, await_ })
    }

    fn parse_parallel_for(&mut self) -> Result<Statement> {
        self.expect(TokenKind::Parallel)?;
        self.expect(TokenKind::For)?;
        let var = if let TokenKind::Identifier(id) = self.consume().unwrap().kind {
            id
        } else {
            bail!("Expected variable name");
        };
        self.expect(TokenKind::In)?;
        let start = self.parse_expression()?;
        self.expect(TokenKind::DotDot)?;
        let end = self.parse_expression()?;
        let body = self.parse_block()?;
        Ok(Statement::ParallelFor { var, range: (start, end), body })
    }

    fn parse_expression(&mut self) -> Result<Expression> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Expression> {
        let mut expr = self.parse_binary(0)?;
        if let Some(token) = self.peek() {
            match token.kind {
                TokenKind::Eq | TokenKind::PlusEq | TokenKind::MinusEq => {
                    let op = match token.kind {
                        TokenKind::Eq => BinaryOp::Eq,
                        _ => BinaryOp::Add,
                    };
                    self.consume();
                    let right = self.parse_assignment()?;
                    expr = Expression::Binary { left: Box::new(expr), op, right: Box::new(right) };
                }
                _ => {}
            }
        }
        Ok(expr)
    }

    fn parse_binary(&mut self, min_prec: u8) -> Result<Expression> {
        let mut lhs = self.parse_unary()?;
        while let Some(op) = self.peek_binary_op() {
            let (prec, _assoc) = op_precedence(op);
            if prec < min_prec {
                break;
            }
            self.consume();
            let rhs = self.parse_binary(prec + 1)?;
            lhs = Expression::Binary { left: Box::new(lhs), op, right: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn peek_binary_op(&self) -> Option<BinaryOp> {
        match self.peek()?.kind {
            TokenKind::Plus => Some(BinaryOp::Add),
            TokenKind::Minus => Some(BinaryOp::Sub),
            TokenKind::Star => Some(BinaryOp::Mul),
            TokenKind::Slash => Some(BinaryOp::Div),
            TokenKind::EqEq => Some(BinaryOp::Eq),
            TokenKind::Ne => Some(BinaryOp::Ne),
            TokenKind::Lt => Some(BinaryOp::Lt),
            TokenKind::Le => Some(BinaryOp::Le),
            TokenKind::Gt => Some(BinaryOp::Gt),
            TokenKind::Ge => Some(BinaryOp::Ge),
            TokenKind::AndAnd => Some(BinaryOp::And),
            TokenKind::OrOr => Some(BinaryOp::Or),
            _ => None,
        }
    }

    fn parse_unary(&mut self) -> Result<Expression> {
        if let Some(token) = self.peek() {
            match token.kind {
                TokenKind::Not => {
                    self.consume();
                    let expr = self.parse_unary()?;
                    Ok(expr)
                }
                _ => self.parse_postfix(),
            }
        } else {
            self.parse_postfix()
        }
    }

    fn parse_postfix(&mut self) -> Result<Expression> {
        let mut expr = self.parse_primary()?;
        loop {
            match self.peek().map(|t| &t.kind) {
                Some(TokenKind::Dot) => {
                    self.consume();
                    match self.peek().unwrap().kind {
                        TokenKind::Identifier(ref method) if method == "place_on" || method == "move_to" => {
                            let method = method.clone();
                            self.consume();
                            self.expect(TokenKind::LParen)?;
                            let args = vec![self.parse_expression()?];
                            self.expect(TokenKind::RParen)?;
                            expr = if method == "place_on" {
                                Expression::PlaceOn { expr: Box::new(expr), device: Box::new(args[0].clone()) }
                            } else {
                                Expression::MoveTo { expr: Box::new(expr), device: Box::new(args[0].clone()) }
                            };
                        }
                        TokenKind::Await => {
                            self.consume();
                            expr = Expression::Await(Box::new(expr));
                        }
                        TokenKind::Identifier(ref field) => {
                            let field = field.clone();
                            self.consume();
                            expr = Expression::FieldAccess { obj: Box::new(expr), field };
                        }
                        _ => bail!("Expected field access"),
                    }
                }
                Some(TokenKind::LParen) => {
                    self.consume();
                    let mut args = Vec::new();
                    if self.peek().map(|t| t.kind == TokenKind::RParen).unwrap_or(false) {
                        args.push(self.parse_expression()?);
                        while self.peek().map(|t| t.kind == TokenKind::Comma).unwrap_or(false) {
                            self.consume();
                            args.push(self.parse_expression()?);
                        }
                    }
                    self.expect(TokenKind::RParen)?;
                    expr = Expression::Call { func: Box::new(expr), args };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expression> {
        let token = self.consume().unwrap();
        match token.kind {
            TokenKind::Integer(s) => Ok(Expression::Integer(s.parse().unwrap())),
            TokenKind::Float(s) => Ok(Expression::Float(s.parse().unwrap())),
            TokenKind::String(s) => Ok(Expression::String(s)),
            TokenKind::True => Ok(Expression::Bool(true)),
            TokenKind::False => Ok(Expression::Bool(false)),
            TokenKind::Nil => Ok(Expression::Nil),
            TokenKind::Identifier(id) => Ok(Expression::Identifier(id)),
            TokenKind::LParen => {
                // 元组或括号表达式
                let expr = self.parse_expression()?;
                self.expect(TokenKind::RParen)?;
                Ok(expr)
            }
            _ => bail!("Expected primary expression"),
        }
    }
}

fn op_precedence(op: BinaryOp) -> (u8, u8) {
    match op {
        BinaryOp::Or => (1, 2),
        BinaryOp::And => (2, 2),
        BinaryOp::Eq | BinaryOp::Ne => (3, 2),
        BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => (4, 2),
        BinaryOp::Add | BinaryOp::Sub => (5, 2),
        BinaryOp::Mul | BinaryOp::Div => (6, 2),
    }
}