//! HSCIR 到 Triton IR 的转换
//!
//! 本模块实现从 HSCIR 操作到 Triton 内核的转换

use super::types::TritonType;
use super::kernel::{TritonKernel, TritonModule, TritonStatement, TritonExpr, TritonParam, TritonConfig};
use super::codegen::TritonCodeGenerator;
use crate::ast::{Program, Task, Function, Statement, Expression, BinaryOp, Type};
use std::collections::HashMap;

/// Triton Lowering trait
///
/// 定义从 HSCIR 到 Triton IR 的转换接口
pub trait TritonLowering {
    /// 转换整个程序
    fn lower_program(&mut self, program: &Program) -> TritonModule;

    /// 转换任务
    fn lower_task(&mut self, task: &Task) -> TritonKernel;

    /// 转换函数
    fn lower_function(&mut self, func: &Function);

    /// 转换类型
    fn lower_type(&self, ty: &Type) -> TritonType;

    /// 转换语句
    fn lower_statement(&mut self, stmt: &Statement) -> Vec<TritonStatement>;

    /// 转换表达式
    fn lower_expression(&mut self, expr: &Expression) -> TritonExpr;
}

impl TritonLowering for TritonLoweringContext {
    // 方法已在上面实现，这里只是空实现以满足 trait
    fn lower_program(&mut self, program: &Program) -> TritonModule {
        self.lower_program(program)
    }

    fn lower_task(&mut self, task: &Task) -> TritonKernel {
        self.lower_task(task)
    }

    fn lower_function(&mut self, func: &Function) {
        self.lower_function(func)
    }

    fn lower_type(&self, ty: &Type) -> TritonType {
        self.lower_type(ty)
    }

    fn lower_statement(&mut self, stmt: &Statement) -> Vec<TritonStatement> {
        self.lower_statement(stmt)
    }

    fn lower_expression(&mut self, expr: &Expression) -> TritonExpr {
        self.lower_expression(expr)
    }
}

/// Triton Lowering 上下文
pub struct TritonLoweringContext {
    /// 符号表：变量名 -> 类型
    symbols: HashMap<String, TritonType>,
    /// 当前内核
    current_kernel: Option<TritonKernel>,
    /// 生成的模块
    module: TritonModule,
    /// 内核计数
    kernel_count: usize,
    /// 配置
    config: TritonConfig,
}

impl TritonLoweringContext {
    /// 创建新的 lowering 上下文
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
            current_kernel: None,
            module: TritonModule::new("main".to_string()),
            kernel_count: 0,
            config: TritonConfig::default(),
        }
    }

    /// 创建带配置的上下文
    pub fn with_config(config: TritonConfig) -> Self {
        Self {
            symbols: HashMap::new(),
            current_kernel: None,
            module: TritonModule::new("main".to_string()),
            kernel_count: 0,
            config,
        }
    }

    /// 转换整个程序
    pub fn lower_program(&mut self, program: &Program) -> TritonModule {
        self.module = TritonModule::new("main".to_string());

        // 转换所有任务
        for task in &program.tasks {
            let kernel = self.lower_task(task);
            self.module.add_kernel(kernel);
        }

        // 记录函数信息（用于生成启动代码）
        for func in &program.functions {
            self.lower_function(func);
        }

        self.module.clone()
    }

    /// 转换任务
    pub fn lower_task(&mut self, task: &Task) -> TritonKernel {
        self.kernel_count += 1;
        let kernel_name = format!("{}_kernel", task.name);
        
        // 创建内核
        let mut kernel = TritonKernel::with_config(kernel_name.clone(), self.config.clone());
        kernel.needs_mask = true;

        // 清空符号表
        self.symbols.clear();

        // 添加参数
        for param in &task.params {
            let ty = self.lower_type(&param.ty);
            self.symbols.insert(param.name.clone(), ty.clone());
            
            // Buffer 类型作为指针传递
            let triton_param = if matches!(param.ty, Type::Buffer(_, _)) {
                TritonParam::new(format!("{}_ptr", param.name), TritonType::pointer(ty))
            } else {
                TritonParam::new(param.name.clone(), ty)
            };
            kernel.add_param(triton_param);
        }

        // 添加 n_elements 参数
        let has_buffer = task.params.iter().any(|p| matches!(p.ty, Type::Buffer(_, _)));
        if has_buffer {
            kernel.add_param(TritonParam::new("n_elements".to_string(), TritonType::i32()));
        }

        // 添加 BLOCK_SIZE constexpr
        kernel.add_param(TritonParam::constexpr("BLOCK_SIZE".to_string(), TritonType::i32()));

        // 添加标准 prolog
        kernel.add_statement(TritonStatement::Let {
            name: "pid".to_string(),
            ty: None,
            init: Some(TritonExpr::call("tl.program_id", vec![TritonExpr::int(0)])),
        });
        kernel.add_statement(TritonStatement::Let {
            name: "block_start".to_string(),
            ty: None,
            init: Some(TritonExpr::binary(
                "*",
                TritonExpr::id("pid"),
                TritonExpr::id("BLOCK_SIZE"),
            )),
        });
        kernel.add_statement(TritonStatement::Let {
            name: "offsets".to_string(),
            ty: None,
            init: Some(TritonExpr::binary(
                "+",
                TritonExpr::id("block_start"),
                TritonExpr::arange(0, 1024), // 使用默认值，实际应该是 BLOCK_SIZE
            )),
        });
        
        if has_buffer {
            kernel.add_statement(TritonStatement::Let {
                name: "mask".to_string(),
                ty: None,
                init: Some(TritonExpr::binary(
                    "<",
                    TritonExpr::id("offsets"),
                    TritonExpr::id("n_elements"),
                )),
            });
        }

        // 转换任务体
        for stmt in &task.body.statements {
            let triton_stmts = self.lower_statement(stmt);
            kernel.add_statements(triton_stmts);
        }

        self.current_kernel = Some(kernel.clone());
        kernel
    }

    /// 转换函数
    pub fn lower_function(&mut self, func: &Function) {
        // 函数通常不需要转换为 Triton 内核
        // 但需要记录其签名供启动代码使用
        // 这里暂时忽略
    }

    /// 转换类型
    pub fn lower_type(&self, ty: &Type) -> TritonType {
        TritonType::from(ty)
    }

    /// 转换语句
    pub fn lower_statement(&mut self, stmt: &Statement) -> Vec<TritonStatement> {
        match stmt {
            Statement::Let { name, ty, init, mutable: _ } => {
                let triton_ty = ty.as_ref().map(|t| self.lower_type(t));
                let init_expr = init.as_ref().map(|e| self.lower_expression(e));
                
                if let Some(t) = ty {
                    self.symbols.insert(name.clone(), self.lower_type(t));
                }
                
                vec![TritonStatement::Let {
                    name: name.clone(),
                    ty: triton_ty,
                    init: init_expr,
                }]
            }
            
            Statement::Return(expr) => {
                let ret_expr = expr.as_ref().map(|e| self.lower_expression(e));
                vec![TritonStatement::Return(ret_expr)]
            }
            
            Statement::Expr(expr) => {
                vec![TritonStatement::Expr(self.lower_expression(expr))]
            }
            
            Statement::ParallelFor { var, range, body } => {
                self.lower_parallel_for(var, range, body)
            }
            
            Statement::For { var, range, body } => {
                let mut stmts = Vec::new();
                
                // 添加循环变量初始化
                for body_stmt in &body.statements {
                    stmts.extend(self.lower_statement(body_stmt));
                }
                
                stmts
            }
            
            Statement::If { condition, then_branch, else_branch } => {
                let cond = self.lower_expression(condition);
                let mut then_stmts = Vec::new();
                for s in &then_branch.statements {
                    then_stmts.extend(self.lower_statement(s));
                }
                
                let mut else_stmts = None;
                if let Some(else_block) = else_branch {
                    let mut stmts = Vec::new();
                    for s in &else_block.statements {
                        stmts.extend(self.lower_statement(s));
                    }
                    else_stmts = Some(stmts);
                }
                
                vec![TritonStatement::If {
                    condition: cond,
                    then_body: then_stmts,
                    else_body: else_stmts,
                }]
            }

            Statement::While { condition, body } => {
                // While 循环在 Triton 中不太常见，简化处理
                let mut stmts = Vec::new();
                for s in &body.statements {
                    stmts.extend(self.lower_statement(s));
                }
                stmts
            }

            Statement::Loop(body) => {
                // Loop 无限循环在 Triton 中不常见，简化处理为忽略
                // 或者可以转换为 while True 循环，但这里暂时返回空语句列表
                Vec::new()
            }

            Statement::Break => vec![],

            Statement::Continue => vec![],
        }
    }

    /// 转换 parallel for
    fn lower_parallel_for(&mut self, var: &str, _range: &(Expression, Expression), body: &crate::ast::Block) -> Vec<TritonStatement> {
        let mut stmts = Vec::new();
        
        // 在 Triton 中，parallel for 使用 offsets
        // 将循环变量映射到 offsets
        self.symbols.insert(var.to_string(), TritonType::i32());
        
        // 转换循环体
        for body_stmt in &body.statements {
            stmts.extend(self.lower_statement_with_var(body_stmt, var, "offsets"));
        }
        
        stmts
    }

    /// 转换语句，替换循环变量
    fn lower_statement_with_var(&mut self, stmt: &Statement, var: &str, replacement: &str) -> Vec<TritonStatement> {
        // 简化处理：直接转换，变量替换在实际代码生成时处理
        self.lower_statement(stmt)
    }

    /// 转换表达式
    pub fn lower_expression(&mut self, expr: &Expression) -> TritonExpr {
        match expr {
            Expression::Integer(i) => TritonExpr::Int(*i),
            
            Expression::Float(f) => TritonExpr::Float(*f),
            
            Expression::String(s) => TritonExpr::String(s.clone()),
            
            Expression::Bool(b) => TritonExpr::Int(if *b { 1 } else { 0 }),
            
            Expression::Nil => TritonExpr::Identifier("None".to_string()),
            
            Expression::Identifier(name) => {
                // 检查是否是 Buffer，需要加 _ptr
                if let Some(ty) = self.symbols.get(name) {
                    if ty.kind == super::types::TritonTypeKind::Tensor {
                        return TritonExpr::Identifier(format!("{}_ptr", name));
                    }
                }
                TritonExpr::Identifier(name.clone())
            }
            
            Expression::Path(path) => {
                let segments: Vec<String> = path.segments.iter()
                    .map(|s| s.ident.clone())
                    .collect();
                TritonExpr::Identifier(segments.join("."))
            }
            
            Expression::Binary { left, op, right } => {
                let lhs = self.lower_expression(left);
                let rhs = self.lower_expression(right);
                let op_str = match op {
                    BinaryOp::Add => "+",
                    BinaryOp::Sub => "-",
                    BinaryOp::Mul => "*",
                    BinaryOp::Div => "/",
                    BinaryOp::Eq => "==",
                    BinaryOp::Ne => "!=",
                    BinaryOp::Lt => "<",
                    BinaryOp::Le => "<=",
                    BinaryOp::Gt => ">",
                    BinaryOp::Ge => ">=",
                    BinaryOp::And => "&", // 在 Triton 中使用位运算
                    BinaryOp::Or => "|",
                };
                TritonExpr::binary(op_str, lhs, rhs)
            }
            
            Expression::Call { func, args } => {
                let func_name = self.get_func_name(func);
                let args_expr: Vec<TritonExpr> = args.iter()
                    .map(|a| self.lower_expression(a))
                    .collect();
                TritonExpr::call(&func_name, args_expr)
            }
            
            Expression::Index { obj, index } => {
                let obj_expr = self.lower_expression(obj);
                let idx_expr = self.lower_expression(index);
                
                // 对于 Buffer，生成 tl.load
                if let Expression::Identifier(name) = obj.as_ref() {
                    if let Some(ty) = self.symbols.get(name) {
                        if ty.kind == super::types::TritonTypeKind::Tensor {
                            return TritonExpr::load(
                                TritonExpr::binary("+", obj_expr, TritonExpr::id("offsets")),
                                Some(TritonExpr::id("mask")),
                                None,
                            );
                        }
                    }
                }
                
                TritonExpr::index(obj_expr, vec![idx_expr])
            }
            
            Expression::MethodCall { obj, method, args } => {
                let obj_expr = self.lower_expression(obj);
                let args_expr: Vec<TritonExpr> = args.iter()
                    .map(|a| self.lower_expression(a))
                    .collect();
                TritonExpr::method(obj_expr, method, args_expr)
            }
            
            Expression::FieldAccess { obj, field } => {
                let obj_expr = self.lower_expression(obj);
                TritonExpr::method(obj_expr, field, vec![])
            }
            
            Expression::Array(elems) => {
                let elems_expr: Vec<TritonExpr> = elems.iter()
                    .map(|e| self.lower_expression(e))
                    .collect();
                // 数组作为 tuple 处理
                TritonExpr::Call {
                    func: "tuple".to_string(),
                    args: elems_expr,
                }
            }
            
            Expression::PlaceOn { expr, device } => {
                // place_on 在内核中是空操作
                self.lower_expression(expr)
            }
            
            Expression::MoveTo { expr, device } => {
                // move_to 在内核中是空操作
                self.lower_expression(expr)
            }
            
            Expression::Await(expr) => {
                self.lower_expression(expr)
            }
            
            Expression::Spawn { device, task, await_ } => {
                // Spawn 生成内核调用
                let task_name = self.get_func_name(task);
                TritonExpr::Call {
                    func: format!("{}_kernel", task_name),
                    args: vec![],
                }
            }
        }
    }

    /// 获取函数名
    fn get_func_name(&self, expr: &Expression) -> String {
        match expr {
            Expression::Path(path) => {
                path.segments.last()
                    .map(|s| s.ident.clone())
                    .unwrap_or_default()
            }
            Expression::Identifier(name) => name.clone(),
            _ => String::new(),
        }
    }

    /// 获取模块
    pub fn module(&self) -> &TritonModule {
        &self.module
    }

    /// 生成 Python 代码
    pub fn generate_python(&self, program: &Program) -> String {
        let mut r#gen = TritonCodeGenerator::new();
        r#gen.generate(program)
    }
}

impl Default for TritonLoweringContext {
    fn default() -> Self {
        Self::new()
    }
}

/// 便捷函数：从程序生成 Triton Python 代码
pub fn lower_to_triton(program: &Program) -> String {
    let mut ctx = TritonLoweringContext::new();
    ctx.lower_program(program);
    ctx.generate_python(program)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn parse_program(source: &str) -> Program {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        parser.parse_program().expect("Failed to parse program")
    }

    #[test]
    fn test_empty_program_lowering() {
        let source = "";
        let program = parse_program(source);
        
        let mut ctx = TritonLoweringContext::new();
        let module = ctx.lower_program(&program);
        
        assert!(module.kernels.is_empty());
    }

    #[test]
    fn test_task_lowering() {
        let source = r#"
task simple_add {
    body(a: Buffer<f32>, b: Buffer<f32>) -> Buffer<f32> {
        parallel for i in 0..1024 {
            let result = a[i] + b[i];
        }
    }
}
"#;
        let program = parse_program(source);
        
        let mut ctx = TritonLoweringContext::new();
        let module = ctx.lower_program(&program);
        
        assert_eq!(module.kernels.len(), 1);
        assert!(module.kernels[0].name.contains("simple_add"));
    }

    #[test]
    fn test_type_lowering() {
        let ctx = TritonLoweringContext::new();
        
        let i32_type = ctx.lower_type(&Type::I32);
        assert_eq!(i32_type.to_triton_string(), "tl.int32");
        
        let f32_type = ctx.lower_type(&Type::F32);
        assert_eq!(f32_type.to_triton_string(), "tl.float32");
        
        let buffer_type = ctx.lower_type(&Type::Buffer(Box::new(Type::F32), Some(1024)));
        assert_eq!(buffer_type.kind, crate::triton::TritonTypeKind::Tensor);
    }

    #[test]
    fn test_expression_lowering() {
        let mut ctx = TritonLoweringContext::new();
        
        let expr = Expression::Binary {
            left: Box::new(Expression::Integer(1)),
            op: BinaryOp::Add,
            right: Box::new(Expression::Integer(2)),
        };
        
        let triton_expr = ctx.lower_expression(&expr);
        assert_eq!(triton_expr.to_code(), "(1 + 2)");
    }

    #[test]
    fn test_full_pipeline() {
        let source = r#"
task vector_add {
    body(a: Buffer<f32>, b: Buffer<f32>) -> Buffer<f32> {
        parallel for i in 0..N {
            let sum = a[i] + b[i];
        }
    }
}

fn main() {
    let a = Buffer::<f32>::zeros([1024]);
    let b = Buffer::<f32>::zeros([1024]);
    let result = spawn on GPU vector_add(a, b).await;
}
"#;
        let program = parse_program(source);
        
        let python_code = lower_to_triton(&program);
        
        assert!(python_code.contains("import triton"));
        assert!(python_code.contains("tl.program_id"));
    }
}
