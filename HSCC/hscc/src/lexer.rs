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

// ========== 测试模块 ==========

#[cfg(test)]
mod tests {
    use super::*;

    // ========== 关键字测试 ==========

    #[test]
    fn test_keywords() {
        let keywords = [
            ("fn", TokenKind::Fn),
            ("let", TokenKind::Let),
            ("mut", TokenKind::Mut),
            ("if", TokenKind::If),
            ("else", TokenKind::Else),
            ("while", TokenKind::While),
            ("loop", TokenKind::Loop),
            ("break", TokenKind::Break),
            ("continue", TokenKind::Continue),
            ("return", TokenKind::Return),
            ("task", TokenKind::Task),
            ("spawn", TokenKind::Spawn),
            ("on", TokenKind::On),
            ("await", TokenKind::Await),
            ("pipeline", TokenKind::Pipeline),
            ("graph", TokenKind::Graph),
            ("parallel", TokenKind::Parallel),
            ("for", TokenKind::For),
            ("reduce", TokenKind::Reduce),
            ("scan", TokenKind::Scan),
            ("true", TokenKind::True),
            ("false", TokenKind::False),
            ("import", TokenKind::Import),
            ("as", TokenKind::As),
        ];

        for (input, expected) in keywords {
            let mut lexer = Lexer::new(input);
            let tokens = lexer.tokenize();
            assert_eq!(tokens.len(), 1, "Expected 1 token for '{}', got {}", input, tokens.len());
            assert_eq!(tokens[0].kind, expected, "Failed for keyword '{}'", input);
        }
    }

    #[test]
    fn test_type_keywords() {
        let type_keywords = [
            ("i8", TokenKind::I8),
            ("i16", TokenKind::I16),
            ("i32", TokenKind::I32),
            ("i64", TokenKind::I64),
            ("i128", TokenKind::I128),
            ("u8", TokenKind::U8),
            ("u16", TokenKind::U16),
            ("u32", TokenKind::U32),
            ("u64", TokenKind::U64),
            ("u128", TokenKind::U128),
            ("f32", TokenKind::F32),
            ("f64", TokenKind::F64),
            ("bool", TokenKind::Bool),
            ("char", TokenKind::Char),
            ("Buffer", TokenKind::Buffer),
        ];

        for (input, expected) in type_keywords {
            let mut lexer = Lexer::new(input);
            let tokens = lexer.tokenize();
            assert_eq!(tokens.len(), 1, "Expected 1 token for '{}', got {}", input, tokens.len());
            assert_eq!(tokens[0].kind, expected, "Failed for type keyword '{}'", input);
        }
    }

    #[test]
    fn test_device_keywords() {
        let device_keywords = [
            ("GPU", TokenKind::GPU),
            ("NPU", TokenKind::NPU),
            ("FPGA", TokenKind::FPGA),
            ("CPU", TokenKind::CPU),
            ("Host", TokenKind::Host),
        ];

        for (input, expected) in device_keywords {
            let mut lexer = Lexer::new(input);
            let tokens = lexer.tokenize();
            assert_eq!(tokens.len(), 1);
            assert_eq!(tokens[0].kind, expected);
        }
    }

    // ========== 标识符测试 ==========

    #[test]
    fn test_identifiers() {
        let test_cases = [
            ("x", "x"),
            ("_underscore", "_underscore"),
            ("camelCase", "camelCase"),
            ("snake_case", "snake_case"),
            ("var123", "var123"),
            ("_123var", "_123var"),
            ("MyType", "MyType"),
        ];

        for (input, expected_name) in test_cases {
            let mut lexer = Lexer::new(input);
            let tokens = lexer.tokenize();
            assert_eq!(tokens.len(), 1);
            match &tokens[0].kind {
                TokenKind::Identifier(name) => assert_eq!(name, expected_name),
                _ => panic!("Expected Identifier, got {:?}", tokens[0].kind),
            }
        }
    }

    // ========== 数字字面量测试 ==========

    #[test]
    fn test_integer_literals() {
        let test_cases = [
            ("0", "0"),
            ("42", "42"),
            ("123_456", "123_456"),
            ("0i32", "0"),
            ("42u64", "42"),
            ("100i8", "100"),
        ];

        for (input, expected_value) in test_cases {
            let mut lexer = Lexer::new(input);
            let tokens = lexer.tokenize();
            assert_eq!(tokens.len(), 1, "Failed for input '{}'", input);
            match &tokens[0].kind {
                TokenKind::Integer(value) => assert_eq!(value, expected_value, "Failed for input '{}'", input),
                _ => panic!("Expected Integer, got {:?}", tokens[0].kind),
            }
        }
    }

    #[test]
    fn test_float_literals() {
        let test_cases = [
            ("3.14", "3.14"),
            ("0.0", "0.0"),
            ("1.5f32", "1.5"),
            ("2.718f64", "2.718"),
            ("1e10", "1e10"),
            ("1.5e-3", "1.5e-3"),
            ("2.5E+10", "2.5E+10"),
        ];

        for (input, expected_value) in test_cases {
            let mut lexer = Lexer::new(input);
            let tokens = lexer.tokenize();
            assert_eq!(tokens.len(), 1, "Failed for input '{}'", input);
            match &tokens[0].kind {
                TokenKind::Float(value) => assert_eq!(value, expected_value, "Failed for input '{}'", input),
                _ => panic!("Expected Float for '{}', got {:?}", input, tokens[0].kind),
            }
        }
    }

    #[test]
    fn test_integer_vs_range_disambiguation() {
        // 测试整数后跟范围运算符的情况
        let mut lexer = Lexer::new("0..10");
        let tokens = lexer.tokenize();
        assert_eq!(tokens.len(), 3);
        match &tokens[0].kind {
            TokenKind::Integer(v) => assert_eq!(v, "0"),
            _ => panic!("Expected Integer"),
        }
        assert_eq!(tokens[1].kind, TokenKind::DotDot);
        match &tokens[2].kind {
            TokenKind::Integer(v) => assert_eq!(v, "10"),
            _ => panic!("Expected Integer"),
        }
    }

    // ========== 字符串和字符测试 ==========

    #[test]
    fn test_string_literals() {
        let test_cases = [
            (r#""hello""#, "hello"),
            (r#""""#, ""),
            (r#""hello world""#, "hello world"),
            (r#""escape\n""#, "escape\\n"),
            (r#""tab\t""#, "tab\\t"),
        ];

        for (input, expected) in test_cases {
            let mut lexer = Lexer::new(input);
            let tokens = lexer.tokenize();
            assert_eq!(tokens.len(), 1, "Failed for input '{}'", input);
            match &tokens[0].kind {
                TokenKind::String(value) => assert_eq!(value, expected, "Failed for input '{}'", input),
                _ => panic!("Expected String, got {:?}", tokens[0].kind),
            }
        }
    }

    #[test]
    fn test_char_literals() {
        let test_cases = [
            ("'a'", "a"),
            ("'0'", "0"),
            ("'\\n'", "\\n"),
            ("'\\t'", "\\t"),
        ];

        for (input, expected) in test_cases {
            let mut lexer = Lexer::new(input);
            let tokens = lexer.tokenize();
            assert_eq!(tokens.len(), 1, "Failed for input '{}'", input);
            match &tokens[0].kind {
                TokenKind::CharLiteral(value) => assert_eq!(value, expected, "Failed for input '{}'", input),
                _ => panic!("Expected CharLiteral, got {:?}", tokens[0].kind),
            }
        }
    }

    // ========== 运算符测试 ==========

    #[test]
    fn test_single_char_operators() {
        let operators = [
            ("+", TokenKind::Plus),
            ("-", TokenKind::Minus),
            ("*", TokenKind::Star),
            ("/", TokenKind::Slash),
            ("%", TokenKind::Percent),
            ("=", TokenKind::Eq),
            ("!", TokenKind::Not),
            ("<", TokenKind::Lt),
            (">", TokenKind::Gt),
            ("&", TokenKind::And),
            ("|", TokenKind::Or),
            ("^", TokenKind::Xor),
            ("~", TokenKind::Tilde),
            ("@", TokenKind::At),
            ("#", TokenKind::Hash),
            ("$", TokenKind::Dollar),
            ("?", TokenKind::Question),
        ];

        for (input, expected) in operators {
            let mut lexer = Lexer::new(input);
            let tokens = lexer.tokenize();
            assert_eq!(tokens.len(), 1, "Failed for operator '{}'", input);
            assert_eq!(tokens[0].kind, expected, "Failed for operator '{}'", input);
        }
    }

    #[test]
    fn test_multi_char_operators() {
        let operators = [
            ("==", TokenKind::EqEq),
            ("!=", TokenKind::Ne),
            ("<=", TokenKind::Le),
            (">=", TokenKind::Ge),
            ("&&", TokenKind::AndAnd),
            ("||", TokenKind::OrOr),
            ("+=", TokenKind::PlusEq),
            ("-=", TokenKind::MinusEq),
            ("*=", TokenKind::StarEq),
            ("/=", TokenKind::SlashEq),
            ("%=", TokenKind::PercentEq),
            ("&=", TokenKind::AndEq),
            ("|=", TokenKind::OrEq),
            ("^=", TokenKind::XorEq),
            ("<<", TokenKind::Shl),
            (">>", TokenKind::Shr),
            ("<<=", TokenKind::ShlEq),
            (">>=", TokenKind::ShrEq),
            ("->", TokenKind::Arrow),
            ("=>", TokenKind::FatArrow),
            ("::", TokenKind::ColonColon),
            ("..", TokenKind::DotDot),
            ("...", TokenKind::DotDotDot),
            ("|>", TokenKind::PipeGt),
            ("<|", TokenKind::LtPipe),
        ];

        for (input, expected) in operators {
            let mut lexer = Lexer::new(input);
            let tokens = lexer.tokenize();
            assert_eq!(tokens.len(), 1, "Failed for operator '{}', got {} tokens", input, tokens.len());
            assert_eq!(tokens[0].kind, expected, "Failed for operator '{}'", input);
        }
    }

    // ========== 分隔符测试 ==========

    #[test]
    fn test_delimiters() {
        let delimiters = [
            ("{", TokenKind::LBrace),
            ("}", TokenKind::RBrace),
            ("[", TokenKind::LBracket),
            ("]", TokenKind::RBracket),
            ("(", TokenKind::LParen),
            (")", TokenKind::RParen),
            (",", TokenKind::Comma),
            (";", TokenKind::Semicolon),
            (":", TokenKind::Colon),
            (".", TokenKind::Dot),
        ];

        for (input, expected) in delimiters {
            let mut lexer = Lexer::new(input);
            let tokens = lexer.tokenize();
            assert_eq!(tokens.len(), 1);
            assert_eq!(tokens[0].kind, expected);
        }
    }

    // ========== 注释测试 ==========

    #[test]
    fn test_line_comments() {
        let input = r#"
let x = 5; // 这是一个注释
let y = 10;
"#;
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();
        
        // 验证注释被忽略
        let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
        assert!(!kinds.iter().any(|k| matches!(k, TokenKind::Comment)));
        assert!(kinds.contains(&&TokenKind::Let));
        assert!(kinds.contains(&&TokenKind::Identifier("x".to_string())));
        assert!(kinds.contains(&&TokenKind::Identifier("y".to_string())));
    }

    #[test]
    fn test_block_comments() {
        let input = r#"
let x = 5; /* 这是
多行注释 */
let y = 10;
"#;
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();
        
        // 验证块注释被忽略
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Identifier(ref s) if s == "x")));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Identifier(ref s) if s == "y")));
    }

    // ========== 位置信息测试 ==========

    #[test]
    fn test_token_positions() {
        let input = "let x = 5;";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        // let 应该在 1:1
        assert_eq!(tokens[0].line, 1);
        assert_eq!(tokens[0].column, 1);

        // x 应该在 1:5
        assert_eq!(tokens[1].line, 1);
        assert_eq!(tokens[1].column, 5);
    }

    #[test]
    fn test_multiline_positions() {
        let input = "let x = 5;\nlet y = 10;";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        // 找到 y 标识符
        let y_token = tokens.iter().find(|t| matches!(&t.kind, TokenKind::Identifier(s) if s == "y"));
        assert!(y_token.is_some());
        let y_token = y_token.unwrap();
        
        // y 应该在第 2 行
        assert_eq!(y_token.line, 2);
        assert_eq!(y_token.column, 5);
    }

    // ========== 复合表达式测试 ==========

    #[test]
    fn test_complex_expression() {
        let input = "let result = (a + b) * c / d;";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        let expected_kinds = vec![
            TokenKind::Let,
            TokenKind::Identifier("result".to_string()),
            TokenKind::Eq,
            TokenKind::LParen,
            TokenKind::Identifier("a".to_string()),
            TokenKind::Plus,
            TokenKind::Identifier("b".to_string()),
            TokenKind::RParen,
            TokenKind::Star,
            TokenKind::Identifier("c".to_string()),
            TokenKind::Slash,
            TokenKind::Identifier("d".to_string()),
            TokenKind::Semicolon,
        ];

        assert_eq!(tokens.len(), expected_kinds.len());
        for (i, (token, expected)) in tokens.iter().zip(expected_kinds.iter()).enumerate() {
            assert_eq!(&token.kind, expected, "Token {} mismatch", i);
        }
    }

    #[test]
    fn test_function_definition() {
        let input = "fn add(a: i32, b: i32) -> i32 { return a + b; }";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        assert!(tokens.iter().any(|t| t.kind == TokenKind::Fn));
        assert!(tokens.iter().any(|t| matches!(&t.kind, TokenKind::Identifier(s) if s == "add")));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::I32));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Arrow));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Return));
    }

    #[test]
    fn test_buffer_type() {
        let input = "Buffer<f32, 10>";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        let expected_kinds = vec![
            TokenKind::Buffer,
            TokenKind::Lt,
            TokenKind::F32,
            TokenKind::Comma,
            TokenKind::Integer("10".to_string()),
            TokenKind::Gt,
        ];

        assert_eq!(tokens.len(), expected_kinds.len());
        for (token, expected) in tokens.iter().zip(expected_kinds.iter()) {
            assert_eq!(&token.kind, expected);
        }
    }

    #[test]
    fn test_import_statement() {
        let input = "import hsc::*;";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        assert!(tokens.iter().any(|t| t.kind == TokenKind::Import));
        assert!(tokens.iter().any(|t| matches!(&t.kind, TokenKind::Identifier(s) if s == "hsc")));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::ColonColon));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Star));
    }

    // ========== 边界情况测试 ==========

    #[test]
    fn test_empty_input() {
        let mut lexer = Lexer::new("");
        let tokens = lexer.tokenize();
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_whitespace_only() {
        let mut lexer = Lexer::new("   \n\t\n   ");
        let tokens = lexer.tokenize();
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_consecutive_operators() {
        let input = "a===b";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();
        
        // 应该解析为 a == = b 或者 a === b（取决于语法）
        assert!(tokens.len() >= 3);
        assert!(matches!(&tokens[0].kind, TokenKind::Identifier(s) if s == "a"));
        assert_eq!(tokens[1].kind, TokenKind::EqEq);
    }

    #[test]
    fn test_underscores_in_numbers() {
        let mut lexer = Lexer::new("1_000_000");
        let tokens = lexer.tokenize();
        assert_eq!(tokens.len(), 1);
        match &tokens[0].kind {
            TokenKind::Integer(v) => assert_eq!(v, "1_000_000"),
            _ => panic!("Expected Integer"),
        }
    }
}