//! Triton 内核表示
//!
//! 定义 Triton 内核的数据结构，包括：
//! - 内核参数
//! - 内核体
//! - Grid/Block 配置

use super::types::TritonType;
use std::collections::HashMap;

/// Triton 内核参数
#[derive(Debug, Clone)]
pub struct TritonParam {
    /// 参数名
    pub name: String,
    /// 参数类型
    pub ty: TritonType,
    /// 是否为编译时常量
    pub is_constexpr: bool,
}

impl TritonParam {
    /// 创建新参数
    pub fn new(name: String, ty: TritonType) -> Self {
        Self {
            name,
            ty,
            is_constexpr: false,
        }
    }

    /// 创建编译时常量参数
    pub fn constexpr(name: String, ty: TritonType) -> Self {
        Self {
            name,
            ty,
            is_constexpr: true,
        }
    }

    /// 生成参数声明代码
    pub fn to_code(&self) -> String {
        if self.is_constexpr {
            format!("{}: tl.constexpr", self.name)
        } else {
            // 指针类型参数不需要类型注解
            if self.ty.kind == super::types::TritonTypeKind::Pointer {
                self.name.clone()
            } else {
                format!("{}: {}", self.name, self.ty.to_triton_string())
            }
        }
    }
}

/// Triton 内核语句
#[derive(Debug, Clone)]
pub enum TritonStatement {
    /// 变量声明
    Let {
        name: String,
        ty: Option<TritonType>,
        init: Option<TritonExpr>,
    },
    /// 赋值语句
    Assign {
        target: TritonExpr,
        value: TritonExpr,
    },
    /// 存储操作
    Store {
        ptr: TritonExpr,
        value: TritonExpr,
        mask: Option<TritonExpr>,
    },
    /// 表达式语句
    Expr(TritonExpr),
    /// 返回语句
    Return(Option<TritonExpr>),
    /// For 循环 (串行)
    For {
        var: String,
        start: TritonExpr,
        end: TritonExpr,
        body: Vec<TritonStatement>,
    },
    /// If 语句
    If {
        condition: TritonExpr,
        then_body: Vec<TritonStatement>,
        else_body: Option<Vec<TritonStatement>>,
    },
}

/// Triton 表达式
#[derive(Debug, Clone)]
pub enum TritonExpr {
    /// 整数常量
    Int(i64),
    /// 浮点常量
    Float(f64),
    /// 字符串常量
    String(String),
    /// 标识符
    Identifier(String),
    /// 二元操作
    Binary {
        op: String,
        lhs: Box<TritonExpr>,
        rhs: Box<TritonExpr>,
    },
    /// 一元操作
    Unary {
        op: String,
        operand: Box<TritonExpr>,
    },
    /// 函数调用
    Call {
        func: String,
        args: Vec<TritonExpr>,
    },
    /// 方法调用
    MethodCall {
        obj: Box<TritonExpr>,
        method: String,
        args: Vec<TritonExpr>,
    },
    /// 索引操作
    Index {
        obj: Box<TritonExpr>,
        indices: Vec<TritonExpr>,
    },
    /// 加载操作
    Load {
        ptr: Box<TritonExpr>,
        mask: Option<Box<TritonExpr>>,
        other: Option<Box<TritonExpr>>,
    },
    /// 算术范围
    Arange {
        start: i64,
        end: i64,
    },
    /// 块大小常量
    BlockSize(String),
}

impl TritonExpr {
    /// 创建标识符表达式
    pub fn id(name: &str) -> Self {
        TritonExpr::Identifier(name.to_string())
    }

    /// 创建整数常量
    pub fn int(value: i64) -> Self {
        TritonExpr::Int(value)
    }

    /// 创建浮点常量
    pub fn float(value: f64) -> Self {
        TritonExpr::Float(value)
    }

    /// 创建二元操作
    pub fn binary(op: &str, lhs: TritonExpr, rhs: TritonExpr) -> Self {
        TritonExpr::Binary {
            op: op.to_string(),
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    }

    /// 创建函数调用
    pub fn call(func: &str, args: Vec<TritonExpr>) -> Self {
        TritonExpr::Call {
            func: func.to_string(),
            args,
        }
    }

    /// 创建方法调用
    pub fn method(obj: TritonExpr, method: &str, args: Vec<TritonExpr>) -> Self {
        TritonExpr::MethodCall {
            obj: Box::new(obj),
            method: method.to_string(),
            args,
        }
    }

    /// 创建索引操作
    pub fn index(obj: TritonExpr, indices: Vec<TritonExpr>) -> Self {
        TritonExpr::Index {
            obj: Box::new(obj),
            indices,
        }
    }

    /// 创建加载操作
    pub fn load(ptr: TritonExpr, mask: Option<TritonExpr>, other: Option<TritonExpr>) -> Self {
        TritonExpr::Load {
            ptr: Box::new(ptr),
            mask: mask.map(Box::new),
            other: other.map(Box::new),
        }
    }

    /// 创建 tl.arange
    pub fn arange(start: i64, end: i64) -> Self {
        TritonExpr::Arange { start, end }
    }

    /// 生成 Python 代码
    pub fn to_code(&self) -> String {
        match self {
            TritonExpr::Int(i) => i.to_string(),
            TritonExpr::Float(f) => {
                if f.fract() == 0.0 {
                    format!("{}.0", f)
                } else {
                    f.to_string()
                }
            }
            TritonExpr::String(s) => format!("\"{}\"", s),
            TritonExpr::Identifier(name) => name.clone(),
            TritonExpr::Binary { op, lhs, rhs } => {
                let lhs_code = lhs.to_code();
                let rhs_code = rhs.to_code();
                format!("({} {} {})", lhs_code, op, rhs_code)
            }
            TritonExpr::Unary { op, operand } => {
                let operand_code = operand.to_code();
                format!("{}{}", op, operand_code)
            }
            TritonExpr::Call { func, args } => {
                let args_code: Vec<String> = args.iter().map(|a| a.to_code()).collect();
                format!("{}({})", func, args_code.join(", "))
            }
            TritonExpr::MethodCall { obj, method, args } => {
                let obj_code = obj.to_code();
                let args_code: Vec<String> = args.iter().map(|a| a.to_code()).collect();
                format!("{}.{}({})", obj_code, method, args_code.join(", "))
            }
            TritonExpr::Index { obj, indices } => {
                let obj_code = obj.to_code();
                let indices_code: Vec<String> = indices.iter().map(|i| i.to_code()).collect();
                format!("{}[{}]", obj_code, indices_code.join(", "))
            }
            TritonExpr::Load { ptr, mask, other } => {
                let ptr_code = ptr.to_code();
                match (mask, other) {
                    (None, None) => format!("tl.load({})", ptr_code),
                    (Some(m), None) => format!("tl.load({}, mask={})", ptr_code, m.to_code()),
                    (None, Some(o)) => format!("tl.load({}, other={})", ptr_code, o.to_code()),
                    (Some(m), Some(o)) => format!(
                        "tl.load({}, mask={}, other={})",
                        ptr_code,
                        m.to_code(),
                        o.to_code()
                    ),
                }
            }
            TritonExpr::Arange { start, end } => {
                format!("tl.arange({}, {})", start, end)
            }
            TritonExpr::BlockSize(name) => name.clone(),
        }
    }
}

// ========== 运算符重载 ==========

impl std::ops::Add for TritonExpr {
    type Output = Self;
    
    fn add(self, rhs: Self) -> Self::Output {
        TritonExpr::binary("+", self, rhs)
    }
}

impl std::ops::Sub for TritonExpr {
    type Output = Self;
    
    fn sub(self, rhs: Self) -> Self::Output {
        TritonExpr::binary("-", self, rhs)
    }
}

impl std::ops::Mul for TritonExpr {
    type Output = Self;
    
    fn mul(self, rhs: Self) -> Self::Output {
        TritonExpr::binary("*", self, rhs)
    }
}

impl std::ops::Div for TritonExpr {
    type Output = Self;
    
    fn div(self, rhs: Self) -> Self::Output {
        TritonExpr::binary("/", self, rhs)
    }
}

impl std::ops::Add for &TritonExpr {
    type Output = TritonExpr;
    
    fn add(self, rhs: Self) -> Self::Output {
        TritonExpr::binary("+", self.clone(), rhs.clone())
    }
}

impl std::ops::Sub for &TritonExpr {
    type Output = TritonExpr;
    
    fn sub(self, rhs: Self) -> Self::Output {
        TritonExpr::binary("-", self.clone(), rhs.clone())
    }
}

impl std::ops::Mul for &TritonExpr {
    type Output = TritonExpr;
    
    fn mul(self, rhs: Self) -> Self::Output {
        TritonExpr::binary("*", self.clone(), rhs.clone())
    }
}

impl std::ops::Div for &TritonExpr {
    type Output = TritonExpr;
    
    fn div(self, rhs: Self) -> Self::Output {
        TritonExpr::binary("/", self.clone(), rhs.clone())
    }
}

impl std::cmp::PartialEq<i64> for TritonExpr {
    fn eq(&self, other: &i64) -> bool {
        match self {
            TritonExpr::Int(i) => *i == *other,
            _ => false,
        }
    }
}

impl TritonExpr {
    /// 小于比较
    pub fn lt(self, other: TritonExpr) -> TritonExpr {
        TritonExpr::binary("<", self, other)
    }
    
    /// 小于等于比较
    pub fn le(self, other: TritonExpr) -> TritonExpr {
        TritonExpr::binary("<=", self, other)
    }
    
    /// 大于比较
    pub fn gt(self, other: TritonExpr) -> TritonExpr {
        TritonExpr::binary(">", self, other)
    }
    
    /// 大于等于比较
    pub fn ge(self, other: TritonExpr) -> TritonExpr {
        TritonExpr::binary(">=", self, other)
    }
    
    /// 等于比较
    pub fn eq_expr(self, other: TritonExpr) -> TritonExpr {
        TritonExpr::binary("==", self, other)
    }
    
    /// 不等于比较
    pub fn ne(self, other: TritonExpr) -> TritonExpr {
        TritonExpr::binary("!=", self, other)
    }
}

/// Triton 内核配置
#[derive(Debug, Clone)]
pub struct TritonConfig {
    /// 块大小配置 (BLOCK_M, BLOCK_N, BLOCK_K 等)
    pub block_sizes: HashMap<String, u32>,
    /// 是否使用共享内存
    pub use_shared_memory: bool,
    /// 展开因子
    pub unroll_factor: u32,
    /// 流数量
    pub num_stages: u32,
    /// 使用的 warp 数量
    pub num_warps: u32,
}

impl Default for TritonConfig {
    fn default() -> Self {
        Self {
            block_sizes: [
                ("BLOCK_SIZE".to_string(), 1024),
                ("BLOCK_M".to_string(), 128),
                ("BLOCK_N".to_string(), 128),
                ("BLOCK_K".to_string(), 32),
            ]
            .iter()
            .cloned()
            .collect(),
            use_shared_memory: false,
            unroll_factor: 1,
            num_stages: 2,
            num_warps: 4,
        }
    }
}

impl TritonConfig {
    /// 创建默认配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 为向量操作创建配置
    pub fn for_vector(size: u32) -> Self {
        let mut config = Self::default();
        config.block_sizes.insert("BLOCK_SIZE".to_string(), size);
        config
    }

    /// 为矩阵乘法创建配置
    pub fn for_matmul(m: u32, n: u32, k: u32) -> Self {
        let mut config = Self::default();
        config.block_sizes.insert("BLOCK_M".to_string(), m);
        config.block_sizes.insert("BLOCK_N".to_string(), n);
        config.block_sizes.insert("BLOCK_K".to_string(), k);
        config.use_shared_memory = true;
        config.num_stages = 4;
        config
    }

    /// 生成块大小参数列表
    pub fn block_params(&self) -> Vec<TritonParam> {
        self.block_sizes
            .iter()
            .map(|(name, _)| TritonParam::constexpr(name.clone(), TritonType::i32()))
            .collect()
    }
}

/// Triton 内核
#[derive(Debug, Clone)]
pub struct TritonKernel {
    /// 内核名称
    pub name: String,
    /// 参数列表
    pub params: Vec<TritonParam>,
    /// 内核体
    pub body: Vec<TritonStatement>,
    /// 配置
    pub config: TritonConfig,
    /// 是否需要掩码 (边界检查)
    pub needs_mask: bool,
}

impl TritonKernel {
    /// 创建新内核
    pub fn new(name: String) -> Self {
        Self {
            name,
            params: Vec::new(),
            body: Vec::new(),
            config: TritonConfig::default(),
            needs_mask: true,
        }
    }

    /// 创建带配置的内核
    pub fn with_config(name: String, config: TritonConfig) -> Self {
        Self {
            name,
            params: Vec::new(),
            body: Vec::new(),
            config,
            needs_mask: true,
        }
    }

    /// 添加参数
    pub fn add_param(&mut self, param: TritonParam) {
        self.params.push(param);
    }

    /// 添加语句
    pub fn add_statement(&mut self, stmt: TritonStatement) {
        self.body.push(stmt);
    }

    /// 添加多个语句
    pub fn add_statements(&mut self, stmts: Vec<TritonStatement>) {
        self.body.extend(stmts);
    }

    /// 生成内核函数代码
    pub fn to_code(&self) -> String {
        let mut code = String::new();

        // 函数装饰器
        code.push_str("@triton.jit\n");

        // 函数签名
        let params_code: Vec<String> = self.params.iter().map(|p| p.to_code()).collect();
        code.push_str(&format!("def (\n{}", self.name));
        code.push_str("    "); // 缩进
        code.push_str(&params_code.join(",\n    "));
        code.push_str("\n):\n");

        // 函数体
        for stmt in &self.body {
            code.push_str(&self.statement_to_code(stmt, 1));
        }

        code
    }

    /// 将语句转换为代码
    fn statement_to_code(&self, stmt: &TritonStatement, indent: usize) -> String {
        let indent_str = "    ".repeat(indent);
        match stmt {
            TritonStatement::Let { name, ty: _, init } => {
                match init {
                    Some(expr) => format!("{}{} = {}\n", indent_str, name, expr.to_code()),
                    None => format!("{}{} = None\n", indent_str, name),
                }
            }
            TritonStatement::Assign { target, value } => {
                format!("{}{} = {}\n", indent_str, target.to_code(), value.to_code())
            }
            TritonStatement::Store { ptr, value, mask } => {
                match mask {
                    Some(m) => format!(
                        "{}tl.store({}, {}, mask={})\n",
                        indent_str,
                        ptr.to_code(),
                        value.to_code(),
                        m.to_code()
                    ),
                    None => format!("{}tl.store({}, {})\n", indent_str, ptr.to_code(), value.to_code()),
                }
            }
            TritonStatement::Expr(expr) => {
                format!("{}{}\n", indent_str, expr.to_code())
            }
            TritonStatement::Return(Some(expr)) => {
                format!("{}return {}\n", indent_str, expr.to_code())
            }
            TritonStatement::Return(None) => {
                format!("{}return\n", indent_str)
            }
            TritonStatement::For { var, start, end, body } => {
                let mut code = format!("{}for {} in range({}, {}):\n", indent_str, var, start.to_code(), end.to_code());
                for s in body {
                    code.push_str(&self.statement_to_code(s, indent + 1));
                }
                code
            }
            TritonStatement::If { condition, then_body, else_body } => {
                let mut code = format!("{}if {}:\n", indent_str, condition.to_code());
                for s in then_body {
                    code.push_str(&self.statement_to_code(s, indent + 1));
                }
                if let Some(else_stmts) = else_body {
                    code.push_str(&format!("{}else:\n", indent_str));
                    for s in else_stmts {
                        code.push_str(&self.statement_to_code(s, indent + 1));
                    }
                }
                code
            }
        }
    }
}

/// Triton 模块 (包含多个内核)
#[derive(Debug, Clone)]
pub struct TritonModule {
    /// 模块名称
    pub name: String,
    /// 内核列表
    pub kernels: Vec<TritonKernel>,
    /// 导入语句
    pub imports: Vec<String>,
    /// 启动函数
    pub launch_functions: Vec<String>,
}

impl TritonModule {
    /// 创建新模块
    pub fn new(name: String) -> Self {
        Self {
            name,
            kernels: Vec::new(),
            imports: vec![
                "import triton".to_string(),
                "import triton.language as tl".to_string(),
            ],
            launch_functions: Vec::new(),
        }
    }

    /// 添加内核
    pub fn add_kernel(&mut self, kernel: TritonKernel) {
        self.kernels.push(kernel);
    }

    /// 生成完整模块代码
    pub fn to_code(&self) -> String {
        let mut code = String::new();

        // 导入语句
        for import in &self.imports {
            code.push_str(import);
            code.push('\n');
        }
        code.push('\n');

        // 内核定义
        for kernel in &self.kernels {
            code.push_str(&kernel.to_code());
            code.push('\n');
        }

        // 启动函数
        for launch in &self.launch_functions {
            code.push_str(launch);
            code.push('\n');
        }

        code
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_param_code() {
        let param = TritonParam::new("x".to_string(), TritonType::float32());
        assert_eq!(param.to_code(), "x");

        let constexpr_param = TritonParam::constexpr("BLOCK_SIZE".to_string(), TritonType::i32());
        assert_eq!(constexpr_param.to_code(), "BLOCK_SIZE: tl.constexpr");
    }

    #[test]
    fn test_expr_code() {
        let expr = TritonExpr::binary("+", TritonExpr::int(1), TritonExpr::int(2));
        assert_eq!(expr.to_code(), "(1 + 2)");

        let load_expr = TritonExpr::load(
            TritonExpr::id("ptr"),
            Some(TritonExpr::id("mask")),
            None,
        );
        assert_eq!(load_expr.to_code(), "tl.load(ptr, mask=mask)");
    }

    #[test]
    fn test_kernel_code() {
        let mut kernel = TritonKernel::new("test_kernel".to_string());
        kernel.add_param(TritonParam::new("x".to_string(), TritonType::pointer(TritonType::float32())));
        kernel.add_param(TritonParam::constexpr("BLOCK_SIZE".to_string(), TritonType::i32()));
        kernel.add_statement(TritonStatement::Let {
            name: "pid".to_string(),
            ty: None,
            init: Some(TritonExpr::call("tl.program_id", vec![TritonExpr::int(0)])),
        });

        let code = kernel.to_code();
        assert!(code.contains("@triton.jit"));
        assert!(code.contains("def test_kernel"));
        assert!(code.contains("tl.program_id"));
    }

    #[test]
    fn test_module_code() {
        let mut module = TritonModule::new("test_module".to_string());
        
        let kernel = TritonKernel::new("simple_kernel".to_string());
        module.add_kernel(kernel);

        let code = module.to_code();
        assert!(code.contains("import triton"));
        assert!(code.contains("import triton.language as tl"));
        assert!(code.contains("def simple_kernel"));
    }
}
