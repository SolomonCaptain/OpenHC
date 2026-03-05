use crate::ast::*;
use anyhow::{bail, Result};
use std::collections::HashMap;
use std::fmt::Debug;

// 类型检查错误定义
#[derive(Debug, Clone)]
pub struct TypeckError {
    pub kind: TypeckErrorKind,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub enum TypeckErrorKind {
    TypeMismatch { expected: String, found: String },
    UndefinedVariable(String),
    UndefinedFunction(String),
    BufferDimensionMismatch { expected: usize, found: usize },
    BufferElementTypeMismatch,
    InvalidBinaryOperation { op: BinaryOp, left: String, right: String },
    ReturnTypeError { expected: Option<String>, found: String },
    ConditionNotBool { found: String },
    AssignmentTypeMismatch { variable: String, expected: String, found: String },
    FunctionCallArgCountMismatch { expected: usize, found: usize },
    FunctionCallArgTypeMismatch { arg_index: usize, expected: String, found: String },
}

impl std::fmt::Display for TypeckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error at {}:{}: ", self.line, self.column)?;
        match &self.kind {
            TypeckErrorKind::TypeMismatch { expected, found } => {
                write!(f, "Type mismatch: expected {}, found {}", expected, found)
            }
            TypeckErrorKind::UndefinedVariable(name) => {
                write!(f, "Undefined variable: {}", name)
            }
            TypeckErrorKind::UndefinedFunction(name) => {
                write!(f, "Undefined function: {}", name)
            }
            TypeckErrorKind::BufferDimensionMismatch { expected, found } => {
                write!(f, "Buffer dimension mismatch: expected {}, found {}", expected, found)
            }
            TypeckErrorKind::BufferElementTypeMismatch => {
                write!(f, "Buffer element type mismatch")
            }
            TypeckErrorKind::InvalidBinaryOperation { op, left, right } => {
                write!(f, "Invalid binary operation {:?}: {} {} {}", op, left, op_as_string(*op), right)
            }
            TypeckErrorKind::ReturnTypeError { expected, found } => {
                match expected {
                    Some(ty) => write!(f, "Return type error: expected {}, found {}", ty, found),
                    None => write!(f, "Return type error: expected void, found {}", found),
                }
            }
            TypeckErrorKind::ConditionNotBool { found } => {
                write!(f, "Condition not bool: {}", found)
            }
            TypeckErrorKind::AssignmentTypeMismatch { variable, expected, found } => {
                write!(f, "Assignment type mismatch: variable {} is of type {}, found {}", variable, expected, found)
            }
            TypeckErrorKind::FunctionCallArgCountMismatch { expected, found } => {
                write!(f, "Function call argument count mismatch: expected {}, found {}", expected, found)
            }
            TypeckErrorKind::FunctionCallArgTypeMismatch { arg_index, expected, found } => {
                write!(f, "Function call argument type mismatch: argument {} is of type {}, found {}", arg_index, expected, found)
            }
        }
    }
}

fn op_as_string(op: BinaryOp) -> &'static str {
    match op {
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
        BinaryOp::And => "&&",
        BinaryOp::Or => "||",
    }
}

// 类型环境
#[derive(Debug, Clone)]
pub struct TypeEnv {
    variables: HashMap<String, Type>,
    function: HashMap<String, FunctionSignature>,
    parent: Option<Box<TypeEnv>>,
    debug_level: usize,
}

#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub params: Vec<Type>,
    pub return_type: Option<Type>,
}

impl TypeEnv {
    pub fn new(debug_level: usize) -> Self {
        TypeEnv {
            variables: HashMap::new(),
            function: HashMap::new(),
            parent: None,
            debug_level,
        }
    }

    pub fn with_parent(parent: TypeEnv, debug_level: usize) -> Self {
        TypeEnv {
            variables: HashMap::new(),
            function: HashMap::new(),
            parent: Some(Box::new(parent)),
            debug_level,
        }
    }

    pub fn insert_variable(&mut self, name: String, ty: Type) {
        if self.debug_level >= 2 {
            println!("[DEBUG] Inserting variable '{}' with type {:?}", name, ty);
        }
        self.variables.insert(name, ty);
    }

    pub fn lookup_variable(&self, name: &str) -> Option<&Type> {
        if let Some(ty) = self.variables.get(name) {
            return Some(ty);
        }
        if let Some(parent) = &self.parent {
            return parent.lookup_variable(name);
        }
        None
    }

    pub fn insert_function(&mut self, name: String, signature: FunctionSignature) {
        if self.debug_level >= 2 {
            println!("[DEBUG] Inserting function '{}' with signature {:?}", name, signature);
        }
        self.function.insert(name, signature);
    }

    pub fn lookup_function(&self, name: &str) -> Option<&FunctionSignature> {
        if let Some(sig) = self.function.get(name) {
            return Some(sig);
        }
        if let Some(parent) = &self.parent {
            return parent.lookup_function(name);
        }
        None
    }

    pub fn print_current_scope(&self) {
        if self.debug_level >= 2 {
            println!("[DEBUG] Current scope:");
            for (name, ty) in &self.variables {
                println!("    {} : {:?}", name, ty);
            }
        }
    }
}

// 类型检查器
pub struct TypeChecker {
    env: TypeEnv,
    errors: Vec<TypeckError>,
    debug_level: usize,
}

impl TypeChecker {
    pub fn new(debug_level: usize) -> Self {
        TypeChecker {
            env: TypeEnv::new(debug_level),
            errors: Vec::new(),
            debug_level,
        }
    }

    pub fn check_program(&mut self, program: &Program) -> Result<()> {
        if self.debug_level >= 1 {
            println!("[DEBUG] Checking program...");
        }

        // 首先注册所有函数签名
        self.register_function_signatures(program);

        // 检查所有函数
        for func in &program.functions {
            if let Err(e) = self.check_function(func) {
                self.errors.push(e);
            }
        }

        // 检查所有 task
        for task in &program.tasks {
            if let Err(e) = self.check_task(task) {
                self.errors.push(e);
            }
        }

        if !self.errors.is_empty() {
            bail!("Type checking failed with {} error(s)", self.errors.len());
        }

        if self.debug_level >= 1 {
            println!("[DEBUG] Program type checked successfully.");
        }

        Ok(())
    }

    fn register_function_signatures(&mut self, program: &Program) {
        for func in &program.functions {
            let sig = FunctionSignature {
                params: func.params.iter().map(|param| param.ty.clone()).collect(),
                return_type: func.return_type.clone(),
            };
            self.env.insert_function(func.name.clone(), sig);
        }

        for task in &program.tasks {
            let sig = FunctionSignature {
                params: task.params.iter().map(|param| param.ty.clone()).collect(),
                return_type: task.return_type.clone(),
            };
            self.env.insert_function(task.name.clone(), sig);
        }
    }

    fn check_function(&mut self, func: &Function) -> Result<(), TypeckError> {
        if self.debug_level >= 1 {
            println!("[DEBUG] Checking function '{}'...", func.name);
        }

        // 创建新的作用域
        let mut local_env = TypeEnv::with_parent(self.env.clone(), self.debug_level);

        // 添加参数到环境
        for param in &func.params {
            local_env.insert_variable(param.name.clone(), param.ty.clone());
        }

        // 临时替换环境
        std::mem::swap(&mut self.env, &mut local_env);

        // 检查函数体
        let result = self.check_block(&func.body, func.return_type.as_ref());

        // 恢复环境
        std::mem::swap(&mut self.env, &mut local_env);

        result
    }

    fn check_task(&mut self, task: &Task) -> Result<(), TypeckError> {
        if self.debug_level >= 1 {
            println!("[DEBUG] Checking task '{}'...", task.name);
        }

        // 创建新的作用域
        let mut local_env = TypeEnv::with_parent(self.env.clone(), self.debug_level);

        // 添加参数到环境
        for param in &task.params {
            local_env.insert_variable(param.name.clone(), param.ty.clone());
        }

        // 临时替换环境
        std::mem::swap(&mut self.env, &mut local_env);

        // 检查任务体
        let result = self.check_block(&task.body, task.return_type.as_ref());

        // 恢复环境
        std::mem::swap(&mut self.env, &mut local_env);

        result
    }

    fn check_block(&mut self, block: &Block, expected_return: Option<&Type>) -> Result<(), TypeckError> {
        if self.debug_level >= 2 {
            println!("  Checking block with {} statements", block.statements.len());
        }

        for stmt in &block.statements {
            self.check_statement(stmt)?;
        }

        Ok(())
    }

    fn check_statement(&mut self, stmt: &Statement) -> Result<(), TypeckError> {
        match stmt {
            Statement::Let { mutable: _, name, ty, init } => {
                if let Some(init_expr) = init {
                    let expr_ty = self.infer_expression(init_expr)?;

                    if let Some(declared_ty) = ty {
                        // 检查声明的类型与推断的类型是否匹配
                        if !self.types_compatible(&expr_ty, declared_ty) {
                            return Err(TypeckError {
                                kind: TypeckErrorKind::AssignmentTypeMismatch {
                                    variable: name.clone(),
                                    expected: self.type_to_string(declared_ty),
                                    found: self.type_to_string(&expr_ty),
                                },
                                line: 0, // TODO: 获取语句的行号
                                column: 0,
                            });
                        }
                        self.env.insert_variable(name.clone(), declared_ty.clone());
                    } else {
                        // 如果没有声明类型，则使用推断的类型
                        self.env.insert_variable(name.clone(), expr_ty);
                    }
                } else if ty.is_none() {
                    // 既没有初始化也没有类型注释
                    return Err(TypeckError {
                        kind: TypeckErrorKind::UndefinedVariable(name.clone()),
                        line: 0,
                        column: 0,
                    });
                } else if let Some(declared_ty) = ty {
                    self.env.insert_variable(name.clone(), declared_ty.clone());
                }
            }

            Statement::Return(expr) => {
                // TODO: 获取当前函数的返回类型
                let expected_return = None;

                if let Some(ret_expr) = expr {
                    let ret_ty = self.infer_expression(ret_expr)?;
                    if let Some(expected) = expected_return {
                        if !self.types_compatible(&ret_ty, expected) {
                            return Err(TypeckError {
                                kind: TypeckErrorKind::ReturnTypeError {
                                    expected: Some(self.type_to_string(expected)),
                                    found: self.type_to_string(&ret_ty),
                                },
                                line: 0,
                                column: 0,
                            });
                        }
                    }
                } else {
                    // return 语句，期望返回类型为 ()
                    if let Some(expected) = expected_return {
                        // 检查是否期望非单元类型
                        // 这里简化处理
                    }
                }
            }

            Statement::Expr(expr) => {
                self.infer_expression(expr)?;
            }

            Statement::ParallelFor { var, range, body } |
            Statement::For { var, range, body } => {
                // 循环变量类型为整数
                self.env.insert_variable(var.clone(), Type::I32);

                // 检查范围表达式
                let (start_ty, end_ty) = (
                    self.infer_expression(&range.0)?,
                    self.infer_expression(&range.1)?
                );

                // 范围必须是整数类型
                if !self.is_integer_type(&start_ty) || !self.is_integer_type(&end_ty) {
                    return Err(TypeckError {
                        kind: TypeckErrorKind::TypeMismatch {
                            expected: "integer type".to_string(),
                            found: format!("{} and {}", self.type_to_string(&start_ty), self.type_to_string(&end_ty)),
                        },
                        line: 0,
                        column: 0
                    });
                }

                // 检查循环体
                self.check_block(body, None)?;
            }

            Statement::If { condition, then_branch, else_branch } => {
                let cond_ty = self.infer_expression(condition)?;
                if !self.is_bool_type(&cond_ty) {
                    return Err(TypeckError {
                        kind: TypeckErrorKind::ConditionNotBool {
                            found: self.type_to_string(&cond_ty),
                        },
                        line: 0,
                        column: 0
                    });
                }

                self.check_block(then_branch, None)?;
                if let Some(else_branch) = else_branch {
                    self.check_block(else_branch, None)?;
                }
            }

            Statement::While { condition, body } => {
                let cond_ty = self.infer_expression(condition)?;
                if !self.is_bool_type(&cond_ty) {
                    return Err(TypeckError {
                        kind: TypeckErrorKind::ConditionNotBool {
                            found: self.type_to_string(&cond_ty),
                        },
                        line: 0,
                        column: 0
                    });
                }
                self.check_block(body, None)?;
            }

            Statement::Loop(body) => {
                self.check_block(body, None)?;
            }

            Statement::Break | Statement::Continue => {
                // 这些语句不需要额外检查
            }
        }

        Ok(())
    }

    fn infer_expression(&mut self, expr: &Expression) -> Result<Type, TypeckError> {
        match expr {
            Expression::Integer(_) => Ok(Type::I32),
            Expression::Float(_) => Ok(Type::F64),
            Expression::String(_) => Ok(Type::Named("String".to_string())),
            Expression::Bool(_) => Ok(Type::Bool),
            Expression::Nil => Ok(Type::Tuple(vec![])), // 单元类型

            Expression::Identifier(name) => {
                if let Some(ty) = self.env.lookup_variable(name) {
                    Ok(ty.clone())
                } else {
                    Err(TypeckError {
                        kind: TypeckErrorKind::UndefinedVariable(name.clone()),
                        line: 0,
                        column: 0,
                    })
                }
            }

            Expression::Path(path) => {
                // 路径可能是函数调用或模块访问
                if path.segments.len() == 1 {
                    let segment = &path.segments[0];
                    if let Some(ty) = self.env.lookup_variable(&segment.ident) {
                        Ok(ty.clone())
                    } else {
                        return Err(TypeckError {
                            kind: TypeckErrorKind::UndefinedVariable(segment.ident.clone()),
                            line: 0,
                            column: 0,
                        })
                    }
                } else {
                    // 模块访问
                    Ok(Type::Named("unknown".to_string()))
                }
            }

            Expression::Binary { left, op, right } => {
                let left_ty = self.infer_expression(left)?;
                let right_ty = self.infer_expression(right)?;

                // 检查二元操作的类型兼容性
                match op {
                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
                        if !self.is_numeric_type(&left_ty) || !self.is_numeric_type(&right_ty) {
                            return Err(TypeckError {
                                kind: TypeckErrorKind::InvalidBinaryOperation {
                                    op: *op,
                                    left: self.type_to_string(&left_ty),
                                    right: self.type_to_string(&right_ty),
                                },
                                line: 0,
                                column: 0,
                            });
                        }
                        // 返回较宽的类型
                        Ok(self.wider_numeric_type(&left_ty, &right_ty))
                    }
                    BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
                        // 比较操作返回 bool
                        if !self.types_compatible(&left_ty, &right_ty) {
                            return Err(TypeckError {
                                kind: TypeckErrorKind::TypeMismatch {
                                    expected: "types are not compatible".to_string(),
                                    found: format!("{} and {}", self.type_to_string(&left_ty), self.type_to_string(&right_ty)),
                                },
                                line: 0,
                                column: 0,
                            });
                        }
                        Ok(Type::Bool)
                    }
                    BinaryOp::And | BinaryOp::Or => {
                        if !self.is_bool_type(&left_ty) || !self.is_bool_type(&right_ty) {
                            return Err(TypeckError {
                                kind: TypeckErrorKind::InvalidBinaryOperation {
                                    op: *op,
                                    left: self.type_to_string(&left_ty),
                                    right: self.type_to_string(&right_ty),
                                },
                                line: 0,
                                column: 0,
                            });
                        }
                        Ok(Type::Bool)
                    }
                }
            }

            Expression::Call { func, args } => {
                // 获取函数签名
                let func_name = match func.as_ref() {
                    Expression::Identifier(name) => name,
                    Expression::Path(path) => &path.segments.last().unwrap().ident,
                    _ => {
                        return Err(TypeckError {
                            kind: TypeckErrorKind::UndefinedFunction("unknown".to_string()),
                            line: 0,
                            column: 0,
                        });
                    }
                };

                if let Some(signature) = self.env.lookup_function(func_name) {
                    // 克隆函数签名
                    let signature = signature.clone();
                    
                    // 检查参数数量
                    if args.len() != signature.params.len() {
                        return Err(TypeckError {
                            kind: TypeckErrorKind::FunctionCallArgCountMismatch {
                                expected: signature.params.len(),
                                found: args.len(),
                            },
                            line: 0,
                            column: 0,
                        });
                    }

                    // 检查每个参数的类型
                    for (i, (arg, param_ty)) in args.iter().zip(&signature.params).enumerate() {
                        let arg_ty = self.infer_expression(arg)?;
                        if !self.types_compatible(&arg_ty, param_ty) {
                            return Err(TypeckError {
                                kind: TypeckErrorKind::FunctionCallArgTypeMismatch {
                                    arg_index: i,
                                    expected: self.type_to_string(param_ty),
                                    found: self.type_to_string(&arg_ty),
                                },
                                line: 0,
                                column: 0,
                            });
                        }
                    }

                    Ok(signature.return_type.clone().unwrap_or(Type::Tuple(vec![])))
                } else {
                    Err(TypeckError {
                        kind: TypeckErrorKind::UndefinedFunction(func_name.to_string()),
                        line: 0,
                        column: 0,
                    })
                }
            }

            Expression::FieldAccess { obj, field: _ } => {
                // 简化处理，返回对象类型
                self.infer_expression(obj)
            }

            Expression::Index { obj, index } => {
                let obj_ty = self.infer_expression(obj)?;
                let index_ty = self.infer_expression(index)?;

                // 检查索引类型
                if !self.is_integer_type(&index_ty) {
                    return Err(TypeckError {
                        kind: TypeckErrorKind::TypeMismatch {
                            expected: "integer".to_string(),
                            found: self.type_to_string(&index_ty),
                        },
                        line: 0,
                        column: 0,
                    });
                }

                // 检查对象是否为 Buffer 或数组
                match obj_ty {
                    Type::Buffer(elem_ty, _) => Ok(*elem_ty),
                    Type::Named(ref name) if name == "Array" => {
                        // 简化处理
                        Ok(Type::I32)
                    }
                    _ => Err(TypeckError {
                        kind: TypeckErrorKind::TypeMismatch {
                            expected: "buffer or array".to_string(),
                            found: self.type_to_string(&obj_ty),
                        },
                        line: 0,
                        column: 0,
                    }),
                }
            }

            Expression::MethodCall { obj, method: _, args: _ } => {
                // 简化处理，返回对象类型
                self.infer_expression(obj)
            }

            Expression::PlaceOn { expr, device: _ } |
            Expression::MoveTo { expr, device: _ } => {
                // 设备相关操作，返回表达式类型
                self.infer_expression(expr)
            }

            Expression::Await(expr) => {
                // Await 解包 Future 类型，简化处理直接返回内部类型
                self.infer_expression(expr)
            }

            Expression::Array(elems) => {
                if elems.is_empty() {
                    return Ok(Type::Named("Array".to_string()));
                }

                let first_ty = self.infer_expression(&elems[0])?;
                for elem in &elems[1..] {
                    let elem_ty = self.infer_expression(elem)?;
                    if !self.types_compatible(&elem_ty, &first_ty) {
                        return Err(TypeckError {
                            kind: TypeckErrorKind::TypeMismatch {
                                expected: self.type_to_string(&first_ty),
                                found: self.type_to_string(&elem_ty),
                            },
                            line: 0,
                            column: 0,
                        });
                    }
                }

                Ok(Type::Buffer(Box::new(first_ty), Some(elems.len())))
            }

            Expression::Spawn { device: _, task, await_: _ } => {
                // Spawn 表达式的类型是任务的返回类型
                self.infer_expression(task)
            }
        }
    }

    // 类型兼容性检查
    fn types_compatible(&self, expected: &Type, found: &Type) -> bool {
        match (expected, found) {
            // 相同类型
            (a, b) if a == b => true,

            // 数值类型的隐式转换
            (Type::I32, Type::I8) |
            (Type::I32, Type::I16) |
            (Type::I64, Type::I8) |
            (Type::I64, Type::I16) |
            (Type::I64, Type::I32) |
            (Type::I128, Type::I8) |
            (Type::I128, Type::I16) |
            (Type::I128, Type::I32) |
            (Type::I128, Type::I64) => true,

            (Type::U32, Type::U8) |
            (Type::U32, Type::U16) |
            (Type::U64, Type::U8) |
            (Type::U64, Type::U16) |
            (Type::U64, Type::U32) |
            (Type::U128, Type::U8) |
            (Type::U128, Type::U16) |
            (Type::U128, Type::U32) |
            (Type::U128, Type::U64) => true,

            (Type::F64, Type::F32) => true,

            // Buffer 类型检查：协变
            (Type::Buffer(exp_inner, exp_dim), Type::Buffer(fnd_inner, fnd_dim)) => {
                // 元素类型必须兼容
                let inner_compatible = self.types_compatible(exp_inner, fnd_inner);
                // 维度必须匹配
                let dim_compatible = match (exp_dim, fnd_dim) {
                    (Some(exp_dim), Some(fnd_dim)) => exp_dim == fnd_dim,
                    (None, None) => true,
                    (None, Some(_)) | (Some(_), None) => true,
                };
                inner_compatible && dim_compatible
            }

            // 其他情况不兼容
            _ => false,
        }
    }

    fn is_integer_type(&self, ty: &Type) -> bool {
        matches!(ty,
            Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128 |
            Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128
        )
    }

    fn is_numeric_type(&self, ty: &Type) -> bool {
        self.is_integer_type(ty) || matches!(ty, Type::F32 | Type::F64)
    }

    fn is_bool_type(&self, ty: &Type) -> bool {
        matches!(ty, Type::Bool)
    }

    fn wider_numeric_type(&self, ty1: &Type, ty2: &Type) -> Type {
        // 返回更宽的数值类型
        let rank = |ty: &Type| -> u8 {
            match ty {
                Type::I8 => 1, Type::I16 => 2, Type::I32 => 3, Type::I64 => 4, Type::I128 => 5,
                Type::U8 => 1, Type::U16 => 2, Type::U32 => 3, Type::U64 => 4, Type::U128 => 5,
                Type::F32 => 6, Type::F64 => 7,
                _ => 0,
            }
        };

        if rank(ty1) >= rank(ty2) {
            ty1.clone()
        } else {
            ty2.clone()
        }
    }

    // 类型可视化辅助
    fn type_to_string(&self, ty: &Type) -> String {
        match ty {
            Type::I8 => "i8".to_string(),
            Type::I16 => "i16".to_string(),
            Type::I32 => "i32".to_string(),
            Type::I64 => "i64".to_string(),
            Type::I128 => "i128".to_string(),
            Type::U8 => "u8".to_string(),
            Type::U16 => "u16".to_string(),
            Type::U32 => "u32".to_string(),
            Type::U64 => "u64".to_string(),
            Type::U128 => "u128".to_string(),
            Type::F32 => "f32".to_string(),
            Type::F64 => "f64".to_string(),
            Type::Bool => "bool".to_string(),
            Type::Char => "char".to_string(),
            Type::Buffer(inner, dim) => {
                match dim {
                    Some(n) => format!("[{}; {}]", self.type_to_string(inner), n),
                    None => format!("Buffer<{}>", self.type_to_string(inner)),
                }
            }
            Type::Named(name) => name.clone(),
            Type::Tuple(types) => {
                let inner: Vec<String> = types.iter().map(|ty| self.type_to_string(ty)).collect();
                format!("({})", inner.join(", "))
            }
        }
    }

    pub fn print_type_hierarchy(&self, ty: &Type, indent: usize) {
        let prefix = " ".repeat(indent);
        match ty {
            Type::Buffer(inner, dim) => {
                println!("{}Buffer [dim={:?}]", prefix, dim);
                self.print_type_hierarchy(inner, indent + 1);
            }
            Type::Tuple(types) => {
                println!("{}Tuple [{} elements]", prefix, types.len());
                for ty in types {
                    self.print_type_hierarchy(ty, indent + 1);
                }
            }
            _ => {
                println!("{}Type: {}", prefix, self.type_to_string(ty));
            }
        }
    }

    // 导出公共 API
    pub fn typecheck_program(program: &Program, debug_level: usize) -> Result<()> {
        let mut checker = TypeChecker::new(debug_level);
        checker.check_program(program)
    }
}