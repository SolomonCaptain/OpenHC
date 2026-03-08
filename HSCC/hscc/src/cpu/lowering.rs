//! CPU 后端 Lowering
//!
//! 将 AST 转换为 CPU 后端可处理的中间表示

use crate::ast::*;
use crate::cpu::types::{CpuType, TypeMapper};
use std::collections::HashMap;

/// CPU 函数表示
#[derive(Debug, Clone)]
pub struct CpuFunction {
    /// 函数名
    pub name: String,
    /// 参数列表
    pub params: Vec<CpuParam>,
    /// 返回类型
    pub return_type: Option<CpuType>,
    /// 函数体
    pub body: Vec<CpuStatement>,
    /// 是否是任务
    pub is_task: bool,
}

/// CPU 参数表示
#[derive(Debug, Clone)]
pub struct CpuParam {
    /// 参数名
    pub name: String,
    /// 参数类型
    pub ty: CpuType,
    /// 是否是引用
    pub is_ref: bool,
}

/// CPU 语句表示
#[derive(Debug, Clone)]
pub struct CpuStatement {
    /// 语句类型
    pub kind: CpuStatementKind,
}

/// CPU 语句种类
#[derive(Debug, Clone)]
pub enum CpuStatementKind {
    /// 变量声明
    VarDecl {
        name: String,
        ty: CpuType,
        init: Option<CpuExpr>,
    },
    /// 表达式语句
    Expr(CpuExpr),
    /// 返回
    Return(Option<CpuExpr>),
    /// 条件语句
    If {
        condition: CpuExpr,
        then_body: Vec<CpuStatement>,
        else_body: Option<Vec<CpuStatement>>,
    },
    /// 循环
    For {
        var: String,
        start: CpuExpr,
        end: CpuExpr,
        body: Vec<CpuStatement>,
    },
    /// 并行循环
    ParallelFor {
        var: String,
        start: CpuExpr,
        end: CpuExpr,
        body: Vec<CpuStatement>,
        /// 归约变量
        reductions: Vec<ReductionInfo>,
    },
    /// While 循环
    While {
        condition: CpuExpr,
        body: Vec<CpuStatement>,
    },
    /// Loop 循环
    Loop {
        body: Vec<CpuStatement>,
    },
    /// Break
    Break,
    /// Continue
    Continue,
}

/// 归约信息
#[derive(Debug, Clone)]
pub struct ReductionInfo {
    /// 变量名
    pub var: String,
    /// 归约操作
    pub op: ReductionOp,
    /// 初始值
    pub init_value: CpuExpr,
}

/// 归约操作
#[derive(Debug, Clone, Copy)]
pub enum ReductionOp {
    Sum,
    Product,
    Min,
    Max,
}

/// CPU 表达式表示
#[derive(Debug, Clone)]
pub struct CpuExpr {
    /// 表达式类型
    pub kind: CpuExprKind,
    /// 结果类型
    pub ty: CpuType,
}

/// CPU 表达式种类
#[derive(Debug, Clone)]
pub enum CpuExprKind {
    /// 整数字面量
    IntLit(i64),
    /// 浮点字面量
    FloatLit(f64),
    /// 布尔字面量
    BoolLit(bool),
    /// 字符串字面量
    StringLit(String),
    /// Nil
    Nil,
    /// 标识符
    Identifier(String),
    /// 路径
    Path(Vec<String>),
    /// 二元操作
    Binary {
        op: BinaryOp,
        left: Box<CpuExpr>,
        right: Box<CpuExpr>,
    },
    /// 函数调用
    Call {
        func: Box<CpuExpr>,
        args: Vec<CpuExpr>,
    },
    /// 字段访问
    FieldAccess {
        obj: Box<CpuExpr>,
        field: String,
    },
    /// 索引
    Index {
        base: Box<CpuExpr>,
        index: Box<CpuExpr>,
    },
    /// 方法调用
    MethodCall {
        obj: Box<CpuExpr>,
        method: String,
        args: Vec<CpuExpr>,
    },
    /// PlaceOn
    PlaceOn {
        expr: Box<CpuExpr>,
        device: Box<CpuExpr>,
    },
    /// MoveTo
    MoveTo {
        expr: Box<CpuExpr>,
        device: Box<CpuExpr>,
    },
    /// Await
    Await(Box<CpuExpr>),
    /// 数组
    Array(Vec<CpuExpr>),
    /// Spawn
    Spawn {
        device: Option<Box<CpuExpr>>,
        task: Box<CpuExpr>,
        await_: bool,
    },
}

/// CPU 模块
#[derive(Debug, Clone)]
pub struct CpuModule {
    /// 函数列表
    pub functions: Vec<CpuFunction>,
    /// 任务列表
    pub tasks: Vec<CpuFunction>,
    /// 全局变量
    pub globals: HashMap<String, CpuType>,
}

impl CpuModule {
    pub fn new() -> Self {
        CpuModule {
            functions: vec![],
            tasks: vec![],
            globals: HashMap::new(),
        }
    }
}

impl Default for CpuModule {
    fn default() -> Self {
        Self::new()
    }
}

/// CPU Lowering 上下文
pub struct CpuLoweringContext {
    /// 符号表
    symbols: HashMap<String, CpuType>,
    /// 当前函数
    current_function: Option<CpuFunction>,
    /// 生成的模块
    module: CpuModule,
}

impl CpuLoweringContext {
    pub fn new() -> Self {
        CpuLoweringContext {
            symbols: HashMap::new(),
            current_function: None,
            module: CpuModule::new(),
        }
    }

    /// Lower 整个程序
    pub fn lower_program(&mut self, program: &Program) -> CpuModule {
        self.module = CpuModule::new();

        // Lower 任务
        for task in &program.tasks {
            let cpu_task = self.lower_task(task);
            self.module.tasks.push(cpu_task);
        }

        // Lower 函数
        for func in &program.functions {
            let cpu_func = self.lower_function(func);
            self.module.functions.push(cpu_func);
        }

        self.module.clone()
    }

    /// Lower 任务
    fn lower_task(&mut self, task: &Task) -> CpuFunction {
        self.symbols.clear();

        // 添加参数到符号表
        let params: Vec<CpuParam> = task.params.iter().map(|p| {
            let ty = CpuType::from_ast(&p.ty);
            self.symbols.insert(p.name.clone(), ty.clone());
            CpuParam {
                name: p.name.clone(),
                ty,
                is_ref: matches!(p.ty, Type::Buffer(_, _)),
            }
        }).collect();

        // Lower 任务体
        let body = self.lower_block(&task.body);

        // 确定返回类型
        let return_type = task.return_type.as_ref().map(|t| CpuType::from_ast(t));

        CpuFunction {
            name: task.name.clone(),
            params,
            return_type,
            body,
            is_task: true,
        }
    }

    /// Lower 函数
    fn lower_function(&mut self, func: &Function) -> CpuFunction {
        self.symbols.clear();

        // 添加参数到符号表
        let params: Vec<CpuParam> = func.params.iter().map(|p| {
            let ty = CpuType::from_ast(&p.ty);
            self.symbols.insert(p.name.clone(), ty.clone());
            CpuParam {
                name: p.name.clone(),
                ty,
                is_ref: matches!(p.ty, Type::Buffer(_, _)),
            }
        }).collect();

        // Lower 函数体
        let body = self.lower_block(&func.body);

        // 确定返回类型
        let return_type = func.return_type.as_ref().map(|t| CpuType::from_ast(t));

        CpuFunction {
            name: func.name.clone(),
            params,
            return_type,
            body,
            is_task: false,
        }
    }

    /// Lower 块
    fn lower_block(&mut self, block: &Block) -> Vec<CpuStatement> {
        block.statements.iter().map(|s| self.lower_statement(s)).collect()
    }

    /// Lower 单个语句
    fn lower_statement(&mut self, stmt: &Statement) -> CpuStatement {
        let kind = match stmt {
            Statement::Let { mutable: _, name, ty, init } => {
                let cpu_ty = ty.as_ref()
                    .map(|t| CpuType::from_ast(t))
                    .unwrap_or_else(|| CpuType::void());
                self.symbols.insert(name.clone(), cpu_ty.clone());
                let init_expr = init.as_ref().map(|e| self.lower_expression(e));
                CpuStatementKind::VarDecl {
                    name: name.clone(),
                    ty: cpu_ty,
                    init: init_expr,
                }
            }

            Statement::Return(value) => {
                CpuStatementKind::Return(value.as_ref().map(|e| self.lower_expression(e)))
            }

            Statement::Expr(expr) => {
                CpuStatementKind::Expr(self.lower_expression(expr))
            }

            Statement::If { condition, then_branch, else_branch } => {
                CpuStatementKind::If {
                    condition: self.lower_expression(condition),
                    then_body: self.lower_block(then_branch),
                    else_body: else_branch.as_ref().map(|b| self.lower_block(b)),
                }
            }

            Statement::While { condition, body } => {
                CpuStatementKind::While {
                    condition: self.lower_expression(condition),
                    body: self.lower_block(body),
                }
            }

            Statement::For { var, range, body } => {
                self.symbols.insert(var.clone(), CpuType::integer(32, true));
                CpuStatementKind::For {
                    var: var.clone(),
                    start: self.lower_expression(&range.0),
                    end: self.lower_expression(&range.1),
                    body: self.lower_block(body),
                }
            }

            Statement::ParallelFor { var, range, body } => {
                self.symbols.insert(var.clone(), CpuType::integer(32, true));

                // 检测归约变量
                let reductions = self.detect_reductions(&body.statements, var);

                CpuStatementKind::ParallelFor {
                    var: var.clone(),
                    start: self.lower_expression(&range.0),
                    end: self.lower_expression(&range.1),
                    body: self.lower_block(body),
                    reductions,
                }
            }

            Statement::Loop(body) => {
                CpuStatementKind::Loop {
                    body: self.lower_block(body),
                }
            }

            Statement::Break => CpuStatementKind::Break,

            Statement::Continue => CpuStatementKind::Continue,
        };

        CpuStatement { kind }
    }

    /// Lower 表达式
    fn lower_expression(&self, expr: &Expression) -> CpuExpr {
        let (kind, ty) = match expr {
            Expression::Integer(n) => {
                (CpuExprKind::IntLit(*n), CpuType::integer(64, true))
            }
            Expression::Float(f) => {
                (CpuExprKind::FloatLit(*f), CpuType::float(64))
            }
            Expression::Bool(b) => {
                (CpuExprKind::BoolLit(*b), CpuType::integer(1, false))
            }
            Expression::String(s) => {
                (CpuExprKind::StringLit(s.clone()), CpuType::void())
            }
            Expression::Nil => {
                (CpuExprKind::Nil, CpuType::void())
            }
            Expression::Identifier(name) => {
                let ty = self.symbols.get(name).cloned().unwrap_or_else(|| CpuType::void());
                (CpuExprKind::Identifier(name.clone()), ty)
            }
            Expression::Path(path) => {
                let segments: Vec<String> = path.segments.iter()
                    .map(|s| s.ident.clone())
                    .collect();
                let ty = if segments.len() == 1 {
                    self.symbols.get(&segments[0]).cloned().unwrap_or_else(|| CpuType::void())
                } else {
                    CpuType::void()
                };
                (CpuExprKind::Path(segments), ty)
            }
            Expression::Binary { op, left, right } => {
                let left_expr = self.lower_expression(left);
                let right_expr = self.lower_expression(right);
                let ty = left_expr.ty.clone();
                (
                    CpuExprKind::Binary {
                        op: *op,
                        left: Box::new(left_expr),
                        right: Box::new(right_expr),
                    },
                    ty,
                )
            }
            Expression::Call { func, args } => {
                let func_expr = self.lower_expression(func);
                let args_exprs: Vec<CpuExpr> = args.iter()
                    .map(|a| self.lower_expression(a))
                    .collect();
                (
                    CpuExprKind::Call {
                        func: Box::new(func_expr),
                        args: args_exprs,
                    },
                    CpuType::void(),
                )
            }
            Expression::FieldAccess { obj, field } => {
                let obj_expr = self.lower_expression(obj);
                let ty = obj_expr.ty.clone();
                (
                    CpuExprKind::FieldAccess {
                        obj: Box::new(obj_expr),
                        field: field.clone(),
                    },
                    ty,
                )
            }
            Expression::Index { obj, index } => {
                let base_expr = self.lower_expression(obj);
                let index_expr = self.lower_expression(index);
                // 索引访问通常返回元素类型
                let ty = if let Some(elem_ty) = &base_expr.ty.element_type {
                    elem_ty.as_ref().clone()
                } else {
                    base_expr.ty.clone()
                };
                (
                    CpuExprKind::Index {
                        base: Box::new(base_expr),
                        index: Box::new(index_expr),
                    },
                    ty,
                )
            }
            Expression::MethodCall { obj, method, args } => {
                let obj_expr = self.lower_expression(obj);
                let args_exprs: Vec<CpuExpr> = args.iter()
                    .map(|a| self.lower_expression(a))
                    .collect();
                (
                    CpuExprKind::MethodCall {
                        obj: Box::new(obj_expr),
                        method: method.clone(),
                        args: args_exprs,
                    },
                    CpuType::void(),
                )
            }
            Expression::PlaceOn { expr, device } => {
                let inner_expr = self.lower_expression(expr);
                let device_expr = self.lower_expression(device);
                let ty = inner_expr.ty.clone();
                (
                    CpuExprKind::PlaceOn {
                        expr: Box::new(inner_expr),
                        device: Box::new(device_expr),
                    },
                    ty,
                )
            }
            Expression::MoveTo { expr, device } => {
                let inner_expr = self.lower_expression(expr);
                let device_expr = self.lower_expression(device);
                let ty = inner_expr.ty.clone();
                (
                    CpuExprKind::MoveTo {
                        expr: Box::new(inner_expr),
                        device: Box::new(device_expr),
                    },
                    ty,
                )
            }
            Expression::Await(inner) => {
                let inner_expr = self.lower_expression(inner);
                let ty = inner_expr.ty.clone();
                (
                    CpuExprKind::Await(Box::new(inner_expr)),
                    ty,
                )
            }
            Expression::Array(elements) => {
                let elem_exprs: Vec<CpuExpr> = elements.iter()
                    .map(|e| self.lower_expression(e))
                    .collect();
                (
                    CpuExprKind::Array(elem_exprs),
                    CpuType::void(),
                )
            }
            Expression::Spawn { device, task, await_ } => {
                let task_expr = self.lower_expression(task);
                let device_expr = device.as_ref().map(|d| Box::new(self.lower_expression(d)));
                (
                    CpuExprKind::Spawn {
                        device: device_expr,
                        task: Box::new(task_expr),
                        await_: *await_,
                    },
                    CpuType::void(),
                )
            }
        };

        CpuExpr { kind, ty }
    }

    /// 检测归约变量
    fn detect_reductions(&self, body: &[Statement], _loop_var: &str) -> Vec<ReductionInfo> {
        let mut reductions = vec![];

        for stmt in body {
            match stmt {
                Statement::Let { name, init, .. } => {
                    if let Some(expr) = init {
                        if self.is_reduction_pattern(expr, name) {
                            let init_value = CpuExpr {
                                kind: CpuExprKind::IntLit(0),
                                ty: CpuType::integer(32, true),
                            };
                            reductions.push(ReductionInfo {
                                var: name.clone(),
                                op: ReductionOp::Sum,
                                init_value,
                            });
                        }
                    }
                }
                Statement::Expr(expr) => {
                    // 检查是否是归约表达式，如 sum += arr[i]
                    if let CpuExpr { kind: CpuExprKind::Call { func, args }, .. } = self.lower_expression(expr) {
                        if let CpuExprKind::Identifier(func_name) = &func.kind {
                            if func_name == "assign" && args.len() == 2 {
                                if let CpuExprKind::Identifier(var_name) = &args[0].kind {
                                    if let CpuExprKind::Binary { op, .. } = &args[1].kind {
                                        let reduction_op = match op {
                                            BinaryOp::Add => Some(ReductionOp::Sum),
                                            BinaryOp::Mul => Some(ReductionOp::Product),
                                            _ => None,
                                        };
                                        if let Some(red_op) = reduction_op {
                                            let init_value = match red_op {
                                                ReductionOp::Sum => CpuExpr {
                                                    kind: CpuExprKind::IntLit(0),
                                                    ty: CpuType::integer(32, true),
                                                },
                                                ReductionOp::Product => CpuExpr {
                                                    kind: CpuExprKind::IntLit(1),
                                                    ty: CpuType::integer(32, true),
                                                },
                                                _ => CpuExpr {
                                                    kind: CpuExprKind::IntLit(0),
                                                    ty: CpuType::integer(32, true),
                                                },
                                            };
                                            reductions.push(ReductionInfo {
                                                var: var_name.clone(),
                                                op: red_op,
                                                init_value,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        reductions
    }

    /// 检查是否是归约模式
    fn is_reduction_pattern(&self, expr: &Expression, var_name: &str) -> bool {
        match expr {
            Expression::Binary { op, left, right } => {
                if let Expression::Identifier(name) = left.as_ref() {
                    if name == var_name {
                        return matches!(op, BinaryOp::Add | BinaryOp::Mul);
                    }
                }
                false
            }
            _ => false,
        }
    }
}

impl Default for CpuLoweringContext {
    fn default() -> Self {
        Self::new()
    }
}

/// CPU Lowering trait
pub trait CpuLowering {
    fn lower(&mut self, program: &Program) -> CpuModule;
}

impl CpuLowering for CpuLoweringContext {
    fn lower(&mut self, program: &Program) -> CpuModule {
        self.lower_program(program)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lowering_context() {
        let mut ctx = CpuLoweringContext::new();
        let program = Program {
            functions: vec![],
            tasks: vec![],
            imports: vec![],
        };

        let module = ctx.lower(&program);
        assert!(module.functions.is_empty());
        assert!(module.tasks.is_empty());
    }

    #[test]
    fn test_lower_simple_function() {
        let mut ctx = CpuLoweringContext::new();
        let program = Program {
            functions: vec![Function {
                name: "test".to_string(),
                params: vec![],
                return_type: None,
                body: Block {
                    statements: vec![Statement::Return(None)],
                },
            }],
            tasks: vec![],
            imports: vec![],
        };

        let module = ctx.lower(&program);
        assert_eq!(module.functions.len(), 1);
        assert_eq!(module.functions[0].name, "test");
    }
}
