//! AST 到 HSCIR 转换模块
//!
//! 本模块将 HSCLang AST 转换为 HSCIR 中间表示，实现：
//! - 类型映射：AST Type → HSCIR Type
//! - 语句转换：Statement → HSCIR Operation
//! - 表达式转换：Expression → HSCIR Value
//! - 控制流转换：If/While/For → CFG

use crate::ast::*;
use crate::hscir::capi::*;
use crate::hscir::types::HscirType;
use crate::hscir::ops::{HscirValue, HscirBlock, HscirRegion, HscirModule, HscirOperation};
use crate::hscir::builder::HscirBuilder;
use anyhow::{Result, anyhow};
use std::collections::HashMap;

/// 符号表：维护 AST 变量名 → HSCIR Value 的映射
pub struct SymbolTable {
    /// 作用域栈，每个作用域是一个变量名到值的映射
    scopes: Vec<HashMap<String, HscirValue>>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
        }
    }

    /// 进入新作用域
    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// 退出当前作用域
    pub fn exit_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// 在当前作用域插入变量
    pub fn insert(&mut self, name: String, value: HscirValue) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, value);
        }
    }

    /// 查找变量（从内向外搜索）
    pub fn lookup(&self, name: &str) -> Option<HscirValue> {
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Some(value.clone());
            }
        }
        None
    }
}

/// HSCIR 上下文，管理全局状态
pub struct HscirContext {
    raw: *mut crate::hscir::capi::HscirContext,
}

impl HscirContext {
    pub fn new() -> Self {
        unsafe {
            Self {
                raw: crate::hscir::capi::hscir_context_create(),
            }
        }
    }

    pub fn as_ptr(&self) -> *mut crate::hscir::capi::HscirContext {
        self.raw
    }
}

impl Drop for HscirContext {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe {
                crate::hscir::capi::hscir_context_destroy(self.raw);
            }
        }
    }
}

/// AST 到 HSCIR 的转换上下文
pub struct LoweringContext {
    /// HSCIR 上下文
    ctx: HscirContext,
    /// IR 构建器
    builder: HscirBuilder,
    /// 符号表
    symbols: SymbolTable,
    /// 当前模块
    module: Option<HscirModule>,
    /// 当前块（用于控制流）
    current_block: Option<HscirBlock>,
    /// break/continue 目标块（用于循环）
    break_target: Option<HscirBlock>,
    continue_target: Option<HscirBlock>,
}

impl LoweringContext {
    /// 创建新的转换上下文
    pub fn new() -> Self {
        let ctx = HscirContext::new();
        let builder = HscirBuilder::new(ctx.as_ptr());
        Self {
            ctx,
            builder,
            symbols: SymbolTable::new(),
            module: None,
            current_block: None,
            break_target: None,
            continue_target: None,
        }
    }

    /// 转换整个程序
    pub fn lower_program(&mut self, program: &Program) -> Result<HscirModule> {
        // 创建模块
        let mut module = HscirModule::new(self.ctx.as_ptr(), "main");

        // 转换所有函数
        for func in &program.functions {
            let op = self.lower_function(func)?;
            module.add_operation(op);
        }

        // 转换所有任务
        for task in &program.tasks {
            let op = self.lower_task(task)?;
            module.add_operation(op);
        }

        self.module = Some(module.clone());
        Ok(module)
    }

    // ========== 类型转换 ==========

    /// 转换 AST 类型到 HSCIR 类型
    pub fn lower_type(&self, ty: &Type) -> Result<HscirType> {
        match ty {
            // 整数类型
            Type::I8 => Ok(HscirType::integer(self.ctx.as_ptr(), 8, true)),
            Type::I16 => Ok(HscirType::integer(self.ctx.as_ptr(), 16, true)),
            Type::I32 => Ok(HscirType::i32(self.ctx.as_ptr())),
            Type::I64 => Ok(HscirType::i64(self.ctx.as_ptr())),
            Type::I128 => Ok(HscirType::integer(self.ctx.as_ptr(), 128, true)),
            Type::U8 => Ok(HscirType::integer(self.ctx.as_ptr(), 8, false)),
            Type::U16 => Ok(HscirType::integer(self.ctx.as_ptr(), 16, false)),
            Type::U32 => Ok(HscirType::integer(self.ctx.as_ptr(), 32, false)),
            Type::U64 => Ok(HscirType::integer(self.ctx.as_ptr(), 64, false)),
            Type::U128 => Ok(HscirType::integer(self.ctx.as_ptr(), 128, false)),

            // 浮点类型
            Type::F32 => Ok(HscirType::f32(self.ctx.as_ptr())),
            Type::F64 => Ok(HscirType::f64(self.ctx.as_ptr())),

            // 布尔类型
            Type::Bool => Ok(HscirType::bool(self.ctx.as_ptr())),

            // 字符类型
            Type::Char => Ok(HscirType::integer(self.ctx.as_ptr(), 8, false)),

            // Buffer 类型
            Type::Buffer(elem_ty, dim) => {
                let elem = self.lower_type(elem_ty)?;
                let shape = dim.map(|n| vec![n as i64]).unwrap_or_default();
                Ok(HscirType::buffer(self.ctx.as_ptr(), &elem, &shape))
            }

            // 命名类型（需要类型解析）
            Type::Named(name) => {
                // TODO: 实现命名类型解析
                Err(anyhow!("Named type '{}' not yet supported", name))
            }

            // 元组类型
            Type::Tuple(types) => {
                // TODO: 实现元组类型
                Err(anyhow!("Tuple types not yet supported"))
            }
        }
    }

    // ========== 函数转换 ==========

    /// 转换函数定义
    fn lower_function(&mut self, func: &Function) -> Result<HscirOperation> {
        // 转换参数类型
        let param_types: Vec<HscirType> = func.params.iter()
            .map(|p| self.lower_type(&p.ty))
            .collect::<Result<_>>()?;

        // 转换返回类型
        let return_types: Vec<HscirType> = func.return_type.as_ref()
            .map(|t| self.lower_type(t))
            .transpose()?
            .map(|t| vec![t])
            .unwrap_or_default();

        // 创建函数类型
        let param_refs: Vec<&HscirType> = param_types.iter().collect();
        let return_refs: Vec<&HscirType> = return_types.iter().collect();
        let func_type = HscirType::function(self.ctx.as_ptr(), &param_refs, &return_refs);

        // 创建函数体区域
        let mut body_region = self.builder.create_region();

        // 创建入口块，参数类型为函数参数类型
        let entry_block = self.builder.create_block(&mut body_region, &param_refs);

        // 设置插入点
        self.builder.set_insertion_point_to_end(&entry_block);
        self.current_block = Some(entry_block.clone());

        // 进入新作用域，注册参数到符号表
        self.symbols.enter_scope();
        for (i, param) in func.params.iter().enumerate() {
            let arg = entry_block.get_argument(i);
            self.symbols.insert(param.name.clone(), arg);
        }

        // 转换函数体
        self.lower_block(&func.body)?;

        // 确保有返回操作
        if !self.ends_with_return(&func.body) {
            self.builder.create_return_op(None);
        }

        self.symbols.exit_scope();

        // 创建函数操作
        Ok(self.builder.create_func_op(&func.name, &func_type, body_region))
    }

    /// 检查块是否以 return 语句结束
    fn ends_with_return(&self, block: &Block) -> bool {
        block.statements.last().map_or(false, |stmt| {
            matches!(stmt, Statement::Return(_))
        })
    }

    // ========== 任务转换 ==========

    /// 转换任务定义
    fn lower_task(&mut self, task: &Task) -> Result<HscirOperation> {
        // 转换参数类型
        let param_types: Vec<HscirType> = task.params.iter()
            .map(|p| self.lower_type(&p.ty))
            .collect::<Result<_>>()?;

        // 转换返回类型
        let return_types: Vec<HscirType> = task.return_type.as_ref()
            .map(|t| self.lower_type(t))
            .transpose()?
            .map(|t| vec![t])
            .unwrap_or_default();

        // 创建函数类型
        let param_refs: Vec<&HscirType> = param_types.iter().collect();
        let return_refs: Vec<&HscirType> = return_types.iter().collect();
        let func_type = HscirType::function(self.ctx.as_ptr(), &param_refs, &return_refs);

        // 创建任务体区域
        let mut body_region = self.builder.create_region();
        let entry_block = self.builder.create_block(&mut body_region, &param_refs);

        // 设置插入点
        self.builder.set_insertion_point_to_end(&entry_block);
        self.current_block = Some(entry_block.clone());

        // 进入新作用域，注册参数到符号表
        self.symbols.enter_scope();
        for (i, param) in task.params.iter().enumerate() {
            let arg = entry_block.get_argument(i);
            self.symbols.insert(param.name.clone(), arg);
        }

        // 转换任务体
        self.lower_block(&task.body)?;

        // 确保有返回操作
        if !self.ends_with_return(&task.body) {
            self.builder.create_return_op(None);
        }

        self.symbols.exit_scope();

        // 创建任务操作
        // TODO: 添加 pattern 和 policy 属性
        Ok(self.builder.create_task_op(&task.name, &func_type, body_region))
    }

    // ========== 块转换 ==========

    /// 转换语句块
    fn lower_block(&mut self, block: &Block) -> Result<()> {
        for stmt in &block.statements {
            self.lower_statement(stmt)?;
        }
        Ok(())
    }

    // ========== 语句转换 ==========

    /// 转换语句
    fn lower_statement(&mut self, stmt: &Statement) -> Result<()> {
        match stmt {
            Statement::Let { name, ty, init, .. } => {
                let value = if let Some(init_expr) = init {
                    self.lower_expression(init_expr)?
                } else {
                    // 未初始化变量：使用默认值或未定义
                    // TODO: 支持未定义值
                    return Err(anyhow!("Uninitialized variables not yet supported: {}", name));
                };
                self.symbols.insert(name.clone(), value);
                Ok(())
            }

            Statement::Return(expr) => {
                let ret_val = expr.as_ref()
                    .map(|e| self.lower_expression(e))
                    .transpose()?;
                self.builder.create_return_op(ret_val.as_ref());
                Ok(())
            }

            Statement::Expr(expr) => {
                self.lower_expression(expr)?;
                Ok(())
            }

            Statement::ParallelFor { var, range, body } => {
                self.lower_parallel_for(var, range, body)
            }

            Statement::For { var, range, body } => {
                self.lower_for(var, range, body)
            }

            Statement::If { condition, then_branch, else_branch } => {
                self.lower_if(condition, then_branch, else_branch.as_ref())
            }

            Statement::While { condition, body } => {
                self.lower_while(condition, body)
            }

            Statement::Loop(body) => {
                self.lower_loop(body)
            }

            Statement::Break => {
                // TODO: 实现 break 跳转
                Err(anyhow!("Break statement not yet supported"))
            }

            Statement::Continue => {
                // TODO: 实现 continue 跳转
                Err(anyhow!("Continue statement not yet supported"))
            }
        }
    }

    /// 转换并行循环
    fn lower_parallel_for(&mut self, var: &str, range: &(Expression, Expression), body: &Block) -> Result<()> {
        let lb = self.lower_expression(&range.0)?;
        let ub = self.lower_expression(&range.1)?;
        let step = self.builder.create_constant_i32(1);

        // 创建循环体区域
        let mut body_region = self.builder.create_region();
        
        // 循环变量类型为 i32
        let i32_type = HscirType::i32(self.ctx.as_ptr());
        let body_block = self.builder.create_block(&mut body_region, &[&i32_type]);

        // 进入循环体作用域
        self.symbols.enter_scope();
        
        // 循环变量作为块参数
        let loop_var = body_block.get_argument(0);
        self.symbols.insert(var.to_string(), loop_var);

        // 保存当前插入点，切换到循环体块
        let prev_block = self.current_block.clone();
        self.builder.set_insertion_point_to_end(&body_block);
        self.current_block = Some(body_block.clone());

        // 转换循环体
        self.lower_block(body)?;

        // 恢复插入点
        if let Some(block) = prev_block {
            self.builder.set_insertion_point_to_end(&block);
            self.current_block = Some(block.clone());
        }

        self.symbols.exit_scope();

        // 创建并行循环操作
        self.builder.create_parallel_for_op(&lb, &ub, &step, body_region);

        Ok(())
    }

    /// 转换普通 for 循环
    fn lower_for(&mut self, var: &str, range: &(Expression, Expression), body: &Block) -> Result<()> {
        // TODO: 实现普通 for 循环（降低到 CFG）
        // 目前简单处理为并行循环
        self.lower_parallel_for(var, range, body)
    }

    /// 转换 if 语句
    fn lower_if(&mut self, condition: &Expression, then_branch: &Block, else_branch: Option<&Block>) -> Result<()> {
        // TODO: 实现完整的 CFG 转换
        // 目前简化处理：先计算条件，然后执行 then 分支
        
        let _cond = self.lower_expression(condition)?;
        
        self.symbols.enter_scope();
        self.lower_block(then_branch)?;
        self.symbols.exit_scope();

        if let Some(else_block) = else_branch {
            self.symbols.enter_scope();
            self.lower_block(else_block)?;
            self.symbols.exit_scope();
        }

        Ok(())
    }

    /// 转换 while 循环
    fn lower_while(&mut self, condition: &Expression, body: &Block) -> Result<()> {
        // TODO: 实现 while 循环（降低到 CFG）
        let _cond = self.lower_expression(condition)?;
        self.symbols.enter_scope();
        self.lower_block(body)?;
        self.symbols.exit_scope();
        Ok(())
    }

    /// 转换无限循环
    fn lower_loop(&mut self, body: &Block) -> Result<()> {
        // TODO: 实现无限循环
        self.symbols.enter_scope();
        self.lower_block(body)?;
        self.symbols.exit_scope();
        Ok(())
    }

    // ========== 表达式转换 ==========

    /// 转换表达式
    fn lower_expression(&mut self, expr: &Expression) -> Result<HscirValue> {
        match expr {
            Expression::Integer(n) => {
                Ok(self.builder.create_constant_i64(*n))
            }

            Expression::Float(f) => {
                Ok(self.builder.create_constant_f64(*f))
            }

            Expression::Bool(b) => {
                Ok(self.builder.create_constant_bool(*b))
            }

            Expression::String(s) => {
                // TODO: 实现字符串常量
                Err(anyhow!("String constants not yet supported: {}", s))
            }

            Expression::Nil => {
                // TODO: 实现 nil 值
                Err(anyhow!("Nil not yet supported"))
            }

            Expression::Identifier(name) => {
                self.symbols.lookup(name)
                    .ok_or_else(|| anyhow!("Undefined variable: {}", name))
            }

            Expression::Path(path) => {
                self.lower_path(path)
            }

            Expression::Binary { left, op, right } => {
                let lhs = self.lower_expression(left)?;
                let rhs = self.lower_expression(right)?;
                Ok(self.builder.create_binary_op(*op, &lhs, &rhs))
            }

            Expression::Call { func, args } => {
                self.lower_call(func, args)
            }

            Expression::FieldAccess { obj, field } => {
                self.lower_field_access(obj, field)
            }

            Expression::Index { obj, index } => {
                let obj_val = self.lower_expression(obj)?;
                let idx_val = self.lower_expression(index)?;
                Ok(self.builder.create_load_op(&obj_val, &idx_val))
            }

            Expression::MethodCall { obj, method, args } => {
                self.lower_method_call(obj, method, args)
            }

            Expression::PlaceOn { expr, device } => {
                let val = self.lower_expression(expr)?;
                let dev = self.lower_expression(device)?;
                Ok(self.builder.create_place_on_op(&val, &dev))
            }

            Expression::MoveTo { expr, device } => {
                let val = self.lower_expression(expr)?;
                let dev = self.lower_expression(device)?;
                Ok(self.builder.create_move_to_op(&val, &dev))
            }

            Expression::Await(expr) => {
                // Await 通常与 Spawn 合并处理
                self.lower_expression(expr)
            }

            Expression::Array(elems) => {
                self.lower_array(elems)
            }

            Expression::Spawn { device, task, await_ } => {
                self.lower_spawn(device.as_deref(), task, *await_)
            }
        }
    }

    /// 转换路径表达式
    fn lower_path(&mut self, path: &Path) -> Result<HscirValue> {
        // 简单处理：单段路径作为标识符
        if path.segments.len() == 1 {
            let name = &path.segments[0].ident;
            self.symbols.lookup(name)
                .ok_or_else(|| anyhow!("Undefined variable: {}", name))
        } else {
            // 多段路径可能是模块成员访问
            // TODO: 实现模块成员访问
            Err(anyhow!("Complex paths not yet supported: {:?}", path))
        }
    }

    /// 转换函数调用
    fn lower_call(&mut self, func: &Expression, args: &[Expression]) -> Result<HscirValue> {
        // 获取函数名
        let func_name = match func {
            Expression::Path(path) if path.segments.len() == 1 => {
                path.segments[0].ident.clone()
            }
            Expression::Identifier(name) => name.clone(),
            _ => return Err(anyhow!("Invalid callee expression")),
        };

        // 转换参数
        let arg_vals: Vec<HscirValue> = args.iter()
            .map(|a| self.lower_expression(a))
            .collect::<Result<_>>()?;
        
        let arg_refs: Vec<&HscirValue> = arg_vals.iter().collect();

        Ok(self.builder.create_call_op(&func_name, &arg_refs))
    }

    /// 转换字段访问
    fn lower_field_access(&mut self, obj: &Expression, field: &str) -> Result<HscirValue> {
        // TODO: 实现字段访问
        let obj_val = self.lower_expression(obj)?;
        Err(anyhow!("Field access not yet supported: .{}", field))
    }

    /// 转换方法调用
    fn lower_method_call(&mut self, obj: &Expression, method: &str, args: &[Expression]) -> Result<HscirValue> {
        let obj_val = self.lower_expression(obj)?;

        // 特殊处理内置方法
        match method {
            "zeros" => {
                // Buffer::zeros
                // TODO: 实现 Buffer 分配
                Err(anyhow!("Buffer::zeros not yet supported"))
            }
            _ => {
                // 普通方法调用
                let arg_vals: Vec<HscirValue> = args.iter()
                    .map(|a| self.lower_expression(a))
                    .collect::<Result<_>>()?;
                let arg_refs: Vec<&HscirValue> = arg_vals.iter().collect();
                
                // 方法调用转换为函数调用，第一个参数为对象
                Err(anyhow!("Method call not yet supported: .{}", method))
            }
        }
    }

    /// 转换数组字面量
    fn lower_array(&mut self, elems: &[Expression]) -> Result<HscirValue> {
        // TODO: 实现数组字面量
        Err(anyhow!("Array literals not yet supported"))
    }

    /// 转换 spawn 表达式
    fn lower_spawn(&mut self, device: Option<&Expression>, task: &Expression, await_: bool) -> Result<HscirValue> {
        // 获取任务调用信息
        if let Expression::Call { func, args } = task {
            let task_name = match func.as_ref() {
                Expression::Path(path) if path.segments.len() == 1 => {
                    path.segments[0].ident.clone()
                }
                Expression::Identifier(name) => name.clone(),
                _ => return Err(anyhow!("Invalid spawn task")),
            };

            // 转换参数
            let arg_vals: Vec<HscirValue> = args.iter()
                .map(|a| self.lower_expression(a))
                .collect::<Result<_>>()?;

            // 转换设备
            let _device_val = device.map(|d| self.lower_expression(d)).transpose()?;

            let arg_refs: Vec<&HscirValue> = arg_vals.iter().collect();

            // 创建 spawn 操作
            // TODO: 正确处理任务引用
            Ok(self.builder.create_spawn_op(&arg_refs.first().unwrap_or(&&HscirValue { raw: std::ptr::null_mut() }), &arg_refs, await_))
        } else {
            Err(anyhow!("Spawn requires a task call"))
        }
    }
}

impl Default for LoweringContext {
    fn default() -> Self {
        Self::new()
    }
}

// ========== 测试模块 ==========

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_table() {
        let mut table = SymbolTable::new();
        
        let val = HscirValue { raw: std::ptr::null_mut() };
        table.insert("x".to_string(), val.clone());
        
        assert!(table.lookup("x").is_some());
        assert!(table.lookup("y").is_none());
        
        table.enter_scope();
        table.insert("y".to_string(), val.clone());
        
        assert!(table.lookup("x").is_some()); // 可以访问外层变量
        assert!(table.lookup("y").is_some());
        
        table.exit_scope();
        assert!(table.lookup("y").is_none()); // 退出作用域后不可见
    }

    #[test]
    fn test_context_creation() {
        let ctx = LoweringContext::new();
        assert!(!ctx.ctx.as_ptr().is_null());
    }
}
