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
        let mut imports = Vec::new();
        let mut functions = Vec::new();
        let mut tasks = Vec::new();

        while self.peek().is_some() && self.peek().unwrap().kind != TokenKind::Eof {
            match self.peek().unwrap().kind {
                TokenKind::Import => imports.push(self.parse_import()?),
                TokenKind::Fn => functions.push(self.parse_function()?),
                TokenKind::Task => tasks.push(self.parse_task()?),
                _ => bail!("Unexpected token at start of declaration"),
            }
        }
        Ok(Program { imports, functions, tasks })
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
        println!("Starting to parse task");
        self.expect(TokenKind::Task)?;
        let name = if let TokenKind::Identifier(id) = self.consume().unwrap().kind {
            id
        } else {
            bail!("Expected task name");
        };
        println!("Task name: {}", name);
        if self.peek().map(|t| t.kind == TokenKind::Colon).unwrap_or(false) {
            self.consume();
            self.parse_type()?;
        }
        self.expect(TokenKind::LBrace)?;
        println!("Task opening brace parsed");
        let mut pattern = None;
        let mut policy = None;
        let mut body = None;

        // 解析 pattern（如果存在）
        if self.peek().map(|t| t.kind == TokenKind::Pattern).unwrap_or(false) {
            println!("Found pattern");
            self.consume();
            self.expect(TokenKind::Colon)?;
            pattern = Some(self.parse_pattern()?);
            println!("Parsed pattern successfully");
            if self.peek().map(|t| t.kind == TokenKind::Comma).unwrap_or(false) {
                self.consume();
            }
        }

        // 解析 policy（如果存在）
        if self.peek().map(|t| t.kind == TokenKind::Policy).unwrap_or(false) {
            println!("Found policy");
            self.consume();
            self.expect(TokenKind::Colon)?;
            policy = Some(self.parse_policy()?);
            println!("Parsed policy successfully");
            if self.peek().map(|t| t.kind == TokenKind::Comma).unwrap_or(false) {
                self.consume();
            }
        }

        // 解析 body
        println!("Looking for body, current token: {:?}", self.peek().map(|t| &t.kind));
        if self.peek().is_some() && self.peek().unwrap().kind != TokenKind::RBrace {
            // 检查是否是直接的 body 标识符
            if self.peek().map(|t| t.kind == TokenKind::Body).unwrap_or(false) {
                println!("Found body keyword");
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
                println!("Parsed body successfully");
            } else if let TokenKind::Identifier(ref ident) = self.peek().unwrap().kind {
                println!("Found identifier: {}", ident);
                // 检查是否是标识符形式的 body
                if ident == "body" {
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
                    println!("Parsed body successfully");
                } else {
                    bail!("Unexpected identifier in task body: {}", ident);
                }
            } else {
                bail!("Expected body definition in task, got {:?}", self.peek().map(|t| &t.kind).unwrap_or(&TokenKind::Eof));
            }
        }
        println!("Expecting RBrace");
        self.expect(TokenKind::RBrace)?;
        let (params, return_type, body) = body.ok_or_else(|| anyhow::anyhow!("Task body missing"))?;
        println!("Task parsing completed successfully");
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
        let kind = if let TokenKind::Identifier(id) = self.consume().unwrap().kind {
            id
        } else {
            bail!("Expected policy kind");
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
        Ok(Policy { kind, fields })
    }

    fn parse_import(&mut self) -> Result<Import> {
        self.expect(TokenKind::Import)?;

        // 检查是否是带大括号的导入
        if self.peek().map(|t| t.kind == TokenKind::LBrace).unwrap_or(false) {
            // 处理 import {item1, item2} 格式
            self.consume(); // 消费 {
            // 跳过大括号内容（简化处理）
            while self.peek().is_some() && self.peek().unwrap().kind != TokenKind::RBrace {
                self.consume();
            }
            self.expect(TokenKind::RBrace)?;
            self.expect(TokenKind::Semicolon)?;
            // 返回一个简单的导入（实际应该解析大括号内的内容）
            let dummy_path = Path { segments: vec![PathSegment { ident: "dummy".to_string(), generic_args: None }] };
            return Ok(Import { path: dummy_path, alias: None });
        }

        let path = self.parse_path()?;
        let alias = if self.peek().map(|t| t.kind == TokenKind::As).unwrap_or(false) {
            self.consume();
            if let TokenKind::Identifier(id) = self.consume().unwrap().kind {
                Some(id)
            } else {
                bail!("Expected identifier after as, got {:?} at line:{} col:{}", self.peek().map(|t| &t.kind), self.peek().map(|t| t.line).unwrap_or(0), self.peek().map(|t| t.column).unwrap_or(0));
            }
        } else {
            None
        };
        self.expect(TokenKind::Semicolon)?;
        Ok(Import { path, alias })
    }

    fn parse_path(&mut self) -> Result<Path> {
        let mut segments = Vec::new();
        // 第一个标识符或关键字（Buffer等）
        let first = match self.consume().unwrap().kind {
            TokenKind::Identifier(id) => id,
            TokenKind::Buffer => "Buffer".to_string(),
            TokenKind::GPU => "GPU".to_string(),
            TokenKind::CPU => "CPU".to_string(),
            TokenKind::NPU => "NPU".to_string(),
            TokenKind::FPGA => "FPGA".to_string(),
            TokenKind::Host => "Host".to_string(),
            kind => {
                bail!("Expected identifier or Buffer keyword, got {:?} at line:{} col:{}", kind, self.peek().map(|t| t.line).unwrap_or(0), self.peek().map(|t| t.column).unwrap_or(0));
            }
        };
        segments.push(PathSegment { ident: first, generic_args: None });

        while self.peek().map(|t| t.kind == TokenKind::ColonColon).unwrap_or(false) {
            self.consume(); // ::

            // 首先检查是否有泛型参数 <...>，这属于前一个 segment
            if let Some(TokenKind::Lt) = self.peek().map(|t| &t.kind) {
                // 解析泛型参数
                self.consume(); // 消费 <
                let mut args = Vec::new();
                loop {
                    let ty = self.parse_type()?;
                    args.push(ty);
                    if self.peek().map(|t| t.kind == TokenKind::Comma).unwrap_or(false) {
                        self.consume();
                    } else {
                        break;
                    }
                }
                self.expect(TokenKind::Gt)?;
                // 将泛型参数附加到最后一个 segment
                if let Some(last) = segments.last_mut() {
                    last.generic_args = Some(args);
                } else {
                    bail!("Unexpected generic arguments without a preceding segment");
                }
                // 继续循环，可能后面还有 :: 和标识符
                continue;
            }

            // 然后，检查下一个 token 是标识符/通配符/大括号等，用于创建新的 segment
            match self.peek().map(|t| &t.kind) {
                Some(TokenKind::Star) => {
                    self.consume();
                    segments.push(PathSegment { ident: "*".to_string(), generic_args: None });
                    break; // 通配符后不能再有其他段
                }
                Some(TokenKind::LBrace) => {
                    // 处理大括号 {item1, item2, ...}
                    self.consume(); // 消费 {
                    // 跳过大括号内容（简化处理）
                    while self.peek().is_some() && self.peek().unwrap().kind != TokenKind::RBrace {
                        self.consume();
                    }
                    self.expect(TokenKind::RBrace)?;
                    segments.push(PathSegment { ident: "{".to_string(), generic_args: None });
                    break; // 大括号后不能再有其他段
                }
                Some(TokenKind::Identifier(_)) | Some(TokenKind::Buffer)
                | Some(TokenKind::GPU) | Some(TokenKind::CPU)
                | Some(TokenKind::NPU) | Some(TokenKind::FPGA)
                | Some(TokenKind::Host) => {
                    // 处理普通标识符或关键字
                    let ident_str = match self.consume().unwrap().kind {
                        TokenKind::Identifier(s) => s,
                        TokenKind::Buffer => "Buffer".to_string(),
                        TokenKind::GPU => "GPU".to_string(),
                        TokenKind::CPU => "CPU".to_string(),
                        TokenKind::NPU => "NPU".to_string(),
                        TokenKind::FPGA => "FPGA".to_string(),
                        TokenKind::Host => "Host".to_string(),
                        _ => unreachable!()
                    };
                    segments.push(PathSegment { ident: ident_str, generic_args: None });
                }
                Some(kind) => {
                    bail!("Expected identifier, *, or {{ after ::, got {:?}", kind);
                }
                None => {
                    bail!("Unexpected end of input after ::");
                }
            }
        }
        Ok(Path { segments })
    }

    fn parse_param_list(&mut self) -> Result<Vec<Param>> {
        println!("Starting to parse parameter list");
        let mut params = Vec::new();
        while self.peek().is_some() && self.peek().unwrap().kind != TokenKind::RParen {
            println!("Current token in param list: {:?}", self.peek().map(|t| &t.kind));
            let name = if let TokenKind::Identifier(id) = self.consume().unwrap().kind {
                println!("Parameter name: {}", id);
                id
            } else {
                bail!("Expected parameter name, got {:?} at line:{} col:{}", self.peek().map(|t| &t.kind), self.peek().map(|t| t.line).unwrap_or(0), self.peek().map(|t| t.column).unwrap_or(0));
            };
            self.expect(TokenKind::Colon)?;
            println!("Parsing type for parameter: {}", name);
            let ty = self.parse_type()?;
            println!("Parsed type successfully for parameter: {}", name);
            params.push(Param { name, ty });
            if self.peek().map(|t| t.kind == TokenKind::Comma).unwrap_or(false) {
                println!("Found comma, continuing to next parameter");
                self.consume();
            } else {
                println!("No comma found, breaking parameter list");
                break;
            }
        }
        println!("Finished parsing parameter list with {} parameters", params.len());
        Ok(params)
    }

    fn parse_type(&mut self) -> Result<Type> {
        println!("Starting to parse type, current token: {:?}", self.peek().map(|t| &t.kind));
        match self.peek().unwrap().kind {
            TokenKind::LParen => {
                // 元组类型: (Type, Type, ...) 或 ()
                self.consume(); // 消费 '('
                let mut types = Vec::new();
                // 如果不是直接遇到右括号，则解析类型列表
                if self.peek().map(|t| t.kind != TokenKind::RParen).unwrap_or(false) {
                    types.push(self.parse_type()?);
                    while self.peek().map(|t| t.kind == TokenKind::Comma).unwrap_or(false) {
                        self.consume(); // 消费 ','
                        types.push(self.parse_type()?);
                    }
                }
                self.expect(TokenKind::RParen)?;
                Ok(Type::Tuple(types))
            }
            TokenKind::I32 => {
                println!("Found I32 type");
                self.consume();
                Ok(Type::I32)
            }
            TokenKind::F32 => {
                println!("Found F32 type");
                self.consume();
                Ok(Type::F32)
            }
            TokenKind::Bool => {
                println!("Found Bool type");
                self.consume();
                Ok(Type::Bool)
            }
            TokenKind::Buffer => {
                println!("Found Buffer type");
                self.consume();
                self.expect(TokenKind::Lt)?;
                println!("Parsing buffer inner type");
                let inner = self.parse_type()?;
                println!("Parsed buffer inner type: {:?}", inner);
                let dims = if self.peek().map(|t| t.kind == TokenKind::Comma).unwrap_or(false) {
                    println!("Found comma in buffer type, parsing dimensions");
                    self.consume();
                    // 解析整数纬度
                    if let TokenKind::Integer(s) = &self.peek().unwrap().kind {
                        println!("Found dimension: {}", s);
                        let dim = s.parse().ok();
                        self.consume();
                        dim
                    } else {
                        println!("No integer dimension found");
                        None
                    }
                } else {
                    println!("No comma found in buffer type");
                    None
                };
                println!("Expecting GT for buffer type");
                self.expect(TokenKind::Gt)?;
                println!("Successfully parsed Buffer type");
                Ok(Type::Buffer(Box::new(inner), dims))
            }
            _ => {
                if let TokenKind::Identifier(id) = self.consume().unwrap().kind {
                    println!("Found named type: {}", id);
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
            TokenKind::Parallel => self.parse_parallel_for(),
            TokenKind::For => self.parse_for_statement(),
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

    fn parse_for_statement(&mut self) -> Result<Statement> {
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
        Ok(Statement::For { var, range: (start, end), body })
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
                    Ok(expr) // 可扩展为真正的 Unary 表达式
                }
                TokenKind::Spawn => {
                    self.consume();
                    let device = if self.peek().map(|t| t.kind == TokenKind::On).unwrap_or(false) {
                        self.consume();
                        Some(Box::new(self.parse_expression()?))
                    } else {
                        None
                    };
                    let task = Box::new(self.parse_expression()?);
                    let await_ = if self.peek().map(|t| t.kind == TokenKind::Dot).unwrap_or(false) {
                        self.consume();
                        self.expect(TokenKind::Await)?;
                        true
                    } else {
                        false
                    };
                    Ok(Expression::Spawn { device, task, await_ })
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
                Some(TokenKind::LBracket) => {
                    self.consume();
                    let index = self.parse_expression()?;
                    self.expect(TokenKind::RBracket)?;
                    expr = Expression::Index { obj: Box::new(expr), index: Box::new(index) };
                }
                Some(TokenKind::LParen) => {
                    self.consume();
                    let mut args = Vec::new();
                    if self.peek().map(|t| t.kind != TokenKind::RParen).unwrap_or(false) {
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
        println!("Parsing primary expression at line {}, col {}, token kind: {:?}", token.line, token.column, token.kind);  // 调试信息
        match token.kind {
            TokenKind::Integer(s) => Ok(Expression::Integer(s.parse().unwrap())),
            TokenKind::Float(s) => Ok(Expression::Float(s.parse().unwrap())),
            TokenKind::String(s) => Ok(Expression::String(s)),
            TokenKind::True => Ok(Expression::Bool(true)),
            TokenKind::False => Ok(Expression::Bool(false)),
            TokenKind::Nil => Ok(Expression::Nil),
            // 处理设备关键字：它们应该作为路径的一部分，而不是单独的 Identifier
            TokenKind::GPU | TokenKind::CPU | TokenKind::NPU | TokenKind::FPGA | TokenKind::Host => {
                self.pos -= 1; // 回退，让 parse_path 处理
                let path = self.parse_path()?;
                Ok(Expression::Path(path))
            }
            TokenKind::Identifier(id) => {
                // 检查下一个 token 是否为 '!'
                if let Some(Token { kind: TokenKind::Not, .. }) = self.peek() {
                    self.consume(); // 消费 '!'
                    let macro_name = format!("{}!", id);
                    Ok(Expression::Identifier(macro_name))
                } else {
                    // 回退然后解析路径
                    self.pos -= 1;
                    let path = self.parse_path()?;
                    Ok(Expression::Path(path))
                }
            }
            TokenKind::Buffer => {
                // Buffer 关键字作为路径的一部分，回退然后解析路径
                self.pos -= 1;
                let path = self.parse_path()?;
                Ok(Expression::Path(path))
            }
            TokenKind::LBracket => {
                // 解析数组字面量 [expr, expr, ...]
                let mut elems = Vec::new();
                if self.peek().map(|t| t.kind != TokenKind::RBracket).unwrap_or(false) {
                    elems.push(self.parse_expression()?);
                    while self.peek().map(|t| t.kind == TokenKind::Comma).unwrap_or(false) {
                        self.consume();
                        elems.push(self.parse_expression()?);
                    }
                }
                self.expect(TokenKind::RBracket)?;
                Ok(Expression::Array(elems))
            },
            TokenKind::LParen => {
                // 元组或括号表达式
                let expr = self.parse_expression()?;
                self.expect(TokenKind::RParen)?;
                Ok(expr)
            }
            _ => bail!("Expected primary expression, got {:?}", token.kind),
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