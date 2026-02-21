#[derive(Debug, PartialEq, Clone)]
pub enum TokenKind {
    // 关键字
    Fn, Let, Mut, If, Else, While, Loop, Break, Continue, Return, Task, Spawn, On, Await, Pipeline, Graph, Stage, Node, Edge, Parallel, For, Reduce, Scan, True, False, Nil, GPU, NPU, FPGA, CPU, Host, DeviceLocal, Unified, Pinned, Pattern, Policy, Body, Buffer, In, Import, As,
    // 类型
    I8, I16, I32, I64, I128, U8, U16, U32, U64, U128, F32, F64, Bool, Char,
    // 符号
    Plus, Minus, Star, Slash, Percent, Eq, EqEq, Ne, Lt, Gt, Le, Ge, AndAnd, OrOr, Not, And, Or, Xor, Shl, Shr, Tilde, PlusEq, MinusEq, StarEq, SlashEq, PercentEq, AndEq, OrEq, XorEq, ShlEq, ShrEq, Arrow, FatArrow, Colon, ColonColon, Dot, DotDot, DotDotDot, At, Hash, Dollar, Question, QuestionQuestion, PipeGt, LtPipe,
    // 分隔符
    LBrace, RBrace, LBracket, RBracket, LParen, RParen, Comma, Semicolon, Pipe, Amp,
    // 字面量
    Integer(String), Float(String), String(String), CharLiteral(String), Identifier(String),
    // 注释
    Comment,
    // 文件结束
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub column: usize,
}

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Lexer {
            chars: input.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn bump(&mut self) {
        if let Some(c) = self.peek() {
            self.pos += 1;
            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.bump();
            } else {
                break;
            }
        }
    }

    fn next_token(&mut self) -> Option<Token> {
        self.skip_whitespace();
        let start_line = self.line;
        let start_col = self.col;
        match self.peek() {
            Some(c) => {
                // 注释处理
                if c == '/' {
                    if self.chars.get(self.pos + 1) == Some(&'/') {
                        // 行注释，跳过直到换行
                        while let Some (c) = self.peek() {
                            if c == '\n' {
                                break;
                            }
                            self.bump();
                        }
                        return self.next_token(); // 忽略注释
                    } else if self.chars.get(self.pos + 1) == Some(&'*') {
                        // 块注释，跳过直到 */
                        self.bump();
                        self.bump(); // 跳过 /*
                        while let (Some(a), Some(b)) = (self.peek(), self.chars.get(self.pos + 1)) {
                            if a == '*' && *b == '/' {
                                self.bump();
                                self.bump(); // 跳过 */
                                break;
                            }
                            self.bump();
                        }
                        return self.next_token(); // 忽略注释
                    }
                }
                // 处理通配符 *
                if c == '*' {
                    // 检查是否是导入语句中的通配符（后面跟着分号或 ::）
                    let next_char = self.chars.get(self.pos + 1);
                    if next_char == Some(&';') || next_char == Some(&':') {
                        self.bump();
                        return Some(Token { kind: TokenKind::Star, line: start_line, column: start_col });
                    }
                }
                
                // 识别标识符和关键字
                if c.is_alphabetic() || c == '_' {
                    let mut s = String::new();
                    while let Some(c) = self.peek() {
                        if c.is_alphanumeric() || c == '_' {
                            s.push(c);
                            self.bump();
                        } else {
                            break;
                        }
                    }
                    let kind = match s.as_str() {
                        "fn" => TokenKind::Fn,
                        "let" => TokenKind::Let,
                        "mut" => TokenKind::Mut,
                        "if" => TokenKind::If,
                        "else" => TokenKind::Else,
                        "while" => TokenKind::While,
                        "loop" => TokenKind::Loop,
                        "break" => TokenKind::Break,
                        "continue" => TokenKind::Continue,
                        "return" => TokenKind::Return,
                        "task" => TokenKind::Task,
                        "spawn" => TokenKind::Spawn,
                        "on" => TokenKind::On,
                        "await" => TokenKind::Await,
                        "pipeline" => TokenKind::Pipeline,
                        "graph" => TokenKind::Graph,
                        "stage" => TokenKind::Stage,
                        "node" => TokenKind::Node,
                        "edge" => TokenKind::Edge,
                        "parallel" => TokenKind::Parallel,
                        "for" => TokenKind::For,
                        "reduce" => TokenKind::Reduce,
                        "scan" => TokenKind::Scan,
                        "true" => TokenKind::True,
                        "false" => TokenKind::False,
                        "nil" => TokenKind::Nil,
                        "GPU" => TokenKind::GPU,
                        "NPU" => TokenKind::NPU,
                        "FPGA" => TokenKind::FPGA,
                        "CPU" => TokenKind::CPU,
                        "Host" => TokenKind::Host,
                        "DeviceLocal" => TokenKind::DeviceLocal,
                        "Unified" => TokenKind::Unified,
                        "Pinned" => TokenKind::Pinned,
                        "pattern" => TokenKind::Pattern,
                        "policy" => TokenKind::Policy,
                        "body" => TokenKind::Body,
                        "Buffer" => TokenKind::Buffer,
                        "in" => TokenKind::In,
                        "i8" => TokenKind::I8,
                        "i16" => TokenKind::I16,
                        "i32" => TokenKind::I32,
                        "i64" => TokenKind::I64,
                        "i128" => TokenKind::I128,
                        "u8" => TokenKind::U8,
                        "u16" => TokenKind::U16,
                        "u32" => TokenKind::U32,
                        "u64" => TokenKind::U64,
                        "u128" => TokenKind::U128,
                        "f32" => TokenKind::F32,
                        "f64" => TokenKind::F64,
                        "bool" => TokenKind::Bool,
                        "char" => TokenKind::Char,
                        "import" => TokenKind::Import,
                        "as" => TokenKind::As,
                        _ => TokenKind::Identifier(s),
                    };
                    return Some(Token { kind, line: start_line, column: start_col });
                }
                // 数字字面量
                if c.is_digit(10) {
                    let mut s = String::new();
                    while let Some(c) = self.peek() {
                        if c.is_digit(10) || c == '_' {
                            s.push(c);
                            self.bump();
                        } else {
                            break;
                        }
                    }
                    // 检查是否浮点数，但要排除范围运算符 ..
                    if self.peek() == Some('.') {
                        // 检查是否后面紧跟另一个点（即 .. 或 ...）
                        if let Some(next_next) = self.chars.get(self.pos + 1) {
                            if *next_next == '.' {
                                // 这不是浮点数，是范围运算符的开头，返回整数
                                // 注意：这里也需要检查后缀，但整数后面跟 .. 时不可能有后缀，所以直接返回
                                return Some(Token { kind: TokenKind::Integer(s), line: start_line, column: start_col });
                            }
                        }
                        // 否则是浮点数
                        s.push('.');
                        self.bump();
                        while let Some(c) = self.peek() {
                            if c.is_digit(10) || c == '_' {
                                s.push(c);
                                self.bump();
                            } else {
                                break;
                            }
                        }
                        // 指数部分
                        if self.peek() == Some('e') || self.peek() == Some('E') {
                            s.push(self.peek().unwrap());
                            self.bump();
                            if self.peek() == Some('+') || self.peek() == Some('-') {
                                s.push(self.peek().unwrap());
                                self.bump();
                            }
                            while let Some(c) = self.peek() {
                                if c.is_digit(10) {
                                    s.push(c);
                                    self.bump();
                                } else {
                                    break;
                                }
                            }
                        }
                        // 处理浮点数后缀（如 f32、f64），只消耗不加入字符串
                        if let Some(c) = self.peek() {
                            if c.is_alphabetic() {
                                while let Some(c) = self.peek() {
                                    if c.is_alphanumeric() {
                                        self.bump();
                                    } else {
                                        break;
                                    }
                                }
                            }
                        }
                        return Some(Token { kind: TokenKind::Float(s), line: start_line, column: start_col });
                    } else {
                        // 整数，处理后缀（如 u32、i64），只消耗不加入字符串
                        if let Some(c) = self.peek() {
                            if c.is_alphabetic() {
                                while let Some(c) = self.peek() {
                                    if c.is_alphanumeric() {
                                        self.bump();
                                    } else {
                                        break;
                                    }
                                }
                            }
                        }
                        return Some(Token { kind: TokenKind::Integer(s), line: start_line, column: start_col });
                    }
                }
                // 字符串
                if c == '"' {
                    self.bump(); // 跳过 "
                    let mut s = String::new();
                    while let Some(c) = self.peek() {
                        if c == '"' {
                            self.bump();
                            break;
                        } else if c == '\\' {
                            // 简单转义处理
                            self.bump();
                            if let Some(esc) = self.peek() {
                                s.push('\\');
                                s.push(esc);
                                self.bump();
                            }
                        } else {
                            s.push(c);
                            self.bump();
                        }
                    }
                    return Some(Token { kind: TokenKind::String(s), line: start_line, column: start_col });
                }
                // 字符
                if c == '\'' {
                    self.bump();
                    let mut s = String::new();
                    if let Some(c) = self.peek() {
                        if c == '\\' {
                            self.bump();
                            if let Some(esc) = self.peek() {
                                s.push('\\');
                                s.push(esc);
                                self.bump();
                            }
                        } else {
                            s.push(c);
                            self.bump();
                        }
                    }
                    if self.peek() == Some('\'') {
                        self.bump();
                    }
                    return Some(Token { kind: TokenKind::CharLiteral(s), line: start_line, column: start_col });
                }
                // 多字符操作符
                let two_char = match (c, self.chars.get(self.pos + 1)) {
                    ('=', Some('=')) => Some(TokenKind::EqEq),
                    ('!', Some('=')) => Some(TokenKind::Ne),
                    ('<', Some('=')) => Some(TokenKind::Le),
                    ('>', Some('=')) => Some(TokenKind::Ge),
                    ('&', Some('&')) => Some(TokenKind::AndAnd),
                    ('|', Some('|')) => Some(TokenKind::OrOr),
                    ('+', Some('=')) => Some(TokenKind::PlusEq),
                    ('-', Some('=')) => Some(TokenKind::MinusEq),
                    ('*', Some('=')) => Some(TokenKind::StarEq),
                    ('/', Some('=')) => Some(TokenKind::SlashEq),
                    ('%', Some('=')) => Some(TokenKind::PercentEq),
                    ('&', Some('=')) => Some(TokenKind::AndEq),
                    ('|', Some('=')) => Some(TokenKind::OrEq),
                    ('^', Some('=')) => Some(TokenKind::XorEq),
                    ('<', Some('<')) => {
                        if self.chars.get(self.pos + 2) == Some(&'=') {
                            self.bump();
                            self.bump();
                            self.bump();
                            return Some(Token { kind: TokenKind::ShlEq, line: start_line, column: start_col });
                        } else {
                            self.bump();
                            self.bump();
                            return Some(Token { kind: TokenKind::Shl, line: start_line, column: start_col });
                        }
                    },
                    ('>', Some('>')) => {
                        if self.chars.get(self.pos + 2) == Some(&'=') {
                            self.bump();
                            self.bump();
                            self.bump();
                            return Some(Token { kind: TokenKind::ShrEq, line: start_line, column: start_col });
                        } else {
                            self.bump();
                            self.bump();
                            return Some(Token { kind: TokenKind::Shr, line: start_line, column: start_col })
                        }
                    },
                    ('-', Some('>')) => Some(TokenKind::Arrow),
                    ('=', Some('>')) => Some(TokenKind::FatArrow),
                    (':', Some(':')) => Some(TokenKind::ColonColon),
                    ('.', Some('.')) => {
                        if self.chars.get(self.pos + 2) == Some(&'.') {
                            self.bump();
                            self.bump();
                            self.bump();
                            return Some(Token { kind: TokenKind::DotDotDot, line: start_line, column: start_col });
                        } else {
                            self.bump();
                            self.bump();
                            return Some(Token { kind: TokenKind::DotDot, line: start_line, column: start_col });
                        }
                    },
                    ('|', Some('>')) => Some(TokenKind::PipeGt),
                    ('<', Some('|')) => Some(TokenKind::LtPipe),
                    _ => None,
                };
                if let Some(kind) = two_char {
                    self.bump();
                    self.bump();
                    return Some(Token { kind, line: start_line, column: start_col });
                }
                // 单字符操作符或分隔符
                let kind = match c {
                    '+' => TokenKind::Plus,
                    '-' => TokenKind::Minus,
                    '*' => TokenKind::Star,
                    '/' => TokenKind::Slash,
                    '%' => TokenKind::Percent,
                    '=' => TokenKind::Eq,
                    '!' => TokenKind::Not,
                    '<' => TokenKind::Lt,
                    '>' => TokenKind::Gt,
                    '&' => TokenKind::And,
                    '|' => TokenKind::Or,
                    '^' => TokenKind::Xor,
                    '~' => TokenKind::Tilde,
                    '{' => TokenKind::LBrace,
                    '}' => TokenKind::RBrace,
                    '[' => TokenKind::LBracket,
                    ']' => TokenKind::RBracket,
                    '(' => TokenKind::LParen,
                    ')' => TokenKind::RParen,
                    ',' => TokenKind::Comma,
                    ';' => TokenKind::Semicolon,
                    ':' => TokenKind::Colon,
                    '.' => TokenKind::Dot,
                    '@' => TokenKind::At,
                    '#' => TokenKind::Hash,
                    '$' => TokenKind::Dollar,
                    '?' => TokenKind::Question,
                    _ => panic!("未知字符: {}", c), // 未知字符
                };
                self.bump();
                Some(Token { kind, line: start_line, column: start_col })
            }
            None => Some(Token { kind: TokenKind::Eof, line: self.line, column: self.col }),
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        while let Some(token) = self.next_token() {
            if token.kind == TokenKind::Eof {
                break;
            }
            tokens.push(token);
        }
        tokens
    }
}