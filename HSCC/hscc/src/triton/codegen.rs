//! Triton 代码生成器
//!
//! 将 Triton IR 转换为可执行的 Python 代码

use super::kernel::{TritonKernel, TritonModule, TritonStatement, TritonExpr, TritonParam, TritonConfig};
use super::types::TritonType;
use crate::ast::{BinaryOp, Expression, Statement, Type, Task, Function, Program};
use std::collections::HashMap;

/// Triton 代码生成器
pub struct TritonCodeGenerator {
    /// 输出代码
    output: String,
    /// 缩进级别
    indent: usize,
    /// 变量类型映射
    var_types: HashMap<String, TritonType>,
    /// 内核计数器
    kernel_count: usize,
    /// 生成的内核
    kernels: Vec<TritonKernel>,
    /// 当前内核名称
    current_kernel: Option<String>,
    /// 配置
    config: TritonConfig,
}

impl TritonCodeGenerator {
    /// 创建新的代码生成器
    pub fn new() -> Self {
        Self {
            output: String::new(),
            indent: 0,
            var_types: HashMap::new(),
            kernel_count: 0,
            kernels: Vec::new(),
            current_kernel: None,
            config: TritonConfig::default(),
        }
    }

    /// 创建带配置的代码生成器
    pub fn with_config(config: TritonConfig) -> Self {
        Self {
            output: String::new(),
            indent: 0,
            var_types: HashMap::new(),
            kernel_count: 0,
            kernels: Vec::new(),
            current_kernel: None,
            config,
        }
    }

    /// 发射代码
    fn emit(&mut self, s: &str) {
        self.output.push_str(s);
    }

    /// 发射一行代码
    fn emitln(&mut self, s: &str) {
        self.output.push_str(&"    ".repeat(self.indent));
        self.output.push_str(s);
        self.output.push('\n');
    }

    /// 增加缩进
    fn indent_inc(&mut self) {
        self.indent += 1;
    }

    /// 减少缩进
    fn indent_dec(&mut self) {
        self.indent = self.indent.saturating_sub(1);
    }

    /// 生成完整的 Python 模块
    pub fn generate(&mut self, program: &Program) -> String {
        self.output.clear();
        self.kernels.clear();
        self.kernel_count = 0;

        // 导入语句
        self.emitln("import torch");
        self.emitln("import triton");
        self.emitln("import triton.language as tl");
        self.emitln("");

        // 生成任务内核
        for task in &program.tasks {
            self.generate_task(task);
        }

        // 生成启动函数
        for task in &program.tasks {
            self.generate_launch_function(task);
        }

        // 生成主函数包装
        if let Some(main_func) = program.functions.iter().find(|f| f.name == "main") {
            self.generate_main_wrapper(main_func, &program.tasks);
        }

        self.output.clone()
    }

    /// 生成任务内核
    fn generate_task(&mut self, task: &Task) {
        let kernel_name = format!("{}_kernel", task.name);
        self.current_kernel = Some(kernel_name.clone());
        self.kernel_count += 1;

        // 内核装饰器
        self.emitln("@triton.jit");
        
        // 构建参数列表
        let mut params = Vec::new();
        
        // 添加输入参数
        for param in &task.params {
            let ty = TritonType::from(&param.ty);
            self.var_types.insert(param.name.clone(), ty.clone());
            
            // Buffer 类型作为指针传递
            if matches!(param.ty, Type::Buffer(_, _)) {
                params.push(format!("{}_ptr", param.name));
            } else {
                params.push(param.name.clone());
            }
        }
        
        // 添加 size 参数（如果有 Buffer）
        let has_buffer = task.params.iter().any(|p| matches!(p.ty, Type::Buffer(_, _)));
        if has_buffer {
            params.push("n_elements".to_string());
        }
        
        // 添加 BLOCK_SIZE constexpr
        params.push("BLOCK_SIZE: tl.constexpr".to_string());

        // 函数签名
        self.emitln(&format!("def {}({}):", kernel_name, params.join(", ")));
        self.indent_inc();

        // 生成 program_id
        self.emitln("pid = tl.program_id(axis=0)");
        self.emitln("block_start = pid * BLOCK_SIZE");
        self.emitln("offsets = block_start + tl.arange(0, BLOCK_SIZE)");
        
        if has_buffer {
            self.emitln("mask = offsets < n_elements");
        }
        self.emitln("");

        // 生成任务体
        self.generate_block(&task.body);

        self.indent_dec();
        self.emitln("");

        // 记录内核信息
        let kernel = TritonKernel::new(kernel_name);
        self.kernels.push(kernel);
    }

    /// 生成语句块
    fn generate_block(&mut self, block: &crate::ast::Block) {
        for stmt in &block.statements {
            self.generate_statement(stmt);
        }
    }

    /// 生成语句
    fn generate_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Let { name, ty, init, mutable: _ } => {
                if let Some(init_expr) = init {
                    let init_code = self.generate_expression(init_expr);

                    // 检查是否是 parallel for 中的计算
                    if matches!(init_expr, Expression::Binary { .. } | Expression::Identifier(_)) {
                        self.emitln(&format!("{} = {}", name, init_code));
                    } else {
                        self.emitln(&format!("{} = {}", name, init_code));
                    }

                    // 记录类型
                    if let Some(t) = ty {
                        self.var_types.insert(name.clone(), TritonType::from(t));
                    }
                }
            }

            Statement::Return(expr) => {
                if let Some(e) = expr {
                    let code = self.generate_expression(e);
                    self.emitln(&format!("return {}", code));
                }
            }
            
            Statement::Expr(expr) => {
                let code = self.generate_expression(expr);
                self.emitln(&format!("{};", code));
            }
            
            Statement::ParallelFor { var, range, body } => {
                self.generate_parallel_for(var, range, body);
            }

            Statement::For { var, range, body } => {
                // 普通 for 循环，在 Triton 中生成串行循环
                let range_start = self.generate_expression(&range.0);
                let range_end = self.generate_expression(&range.1);
                self.emitln(&format!("for {} in range({}, {}):",
                                     var,
                                     range_start,
                                     range_end));
                self.indent_inc();
                self.generate_block(body);
                self.indent_dec();
            }
            
            Statement::If { condition, then_branch, else_branch } => {
                let cond_code = self.generate_expression(condition);
                self.emitln(&format!("if {}:", cond_code));
                self.indent_inc();
                self.generate_block(then_branch);
                self.indent_dec();
                
                if let Some(else_block) = else_branch {
                    self.emitln("else:");
                    self.indent_inc();
                    self.generate_block(else_block);
                    self.indent_dec();
                }
            }
            
            Statement::While { condition, body } => {
                let cond_code = self.generate_expression(condition);
                self.emitln(&format!("while {}:", cond_code));
                self.indent_inc();
                self.generate_block(body);
                self.indent_dec();
            }
            
            _ => {
                // 其他语句暂时忽略
            }
        }
    }

    /// 生成 parallel for
    fn generate_parallel_for(&mut self, var: &str, range: &(Expression, Expression), body: &crate::ast::Block) {
        // 在 Triton 中，parallel for 已经通过 program_id 处理
        // 这里生成使用 offsets 的代码

        // 生成循环体，但使用 offsets 替代循环变量
        let range_start = self.generate_expression(&range.0);
        let range_end = self.generate_expression(&range.1);
        self.emitln(&format!("# parallel for {} in {}..{}", var,
                             range_start,
                             range_end));

        // 替换循环变量为 offsets
        // 这需要更复杂的变量替换逻辑，这里简化处理
        for stmt in &body.statements {
            self.generate_statement(stmt);
        }
    }

    /// 生成表达式
    fn generate_expression(&mut self, expr: &Expression) -> String {
        match expr {
            Expression::Integer(i) => i.to_string(),
            
            Expression::Float(f) => {
                if f.fract() == 0.0 {
                    format!("{}.0", f)
                } else {
                    f.to_string()
                }
            }
            
            Expression::String(s) => format!("\"{}\"", s),
            
            Expression::Bool(b) => b.to_string(),
            
            Expression::Nil => "None".to_string(),
            
            Expression::Identifier(name) => {
                // 检查是否需要加 _ptr 后缀
                if self.var_types.contains_key(name) {
                    let ty = self.var_types.get(name).unwrap();
                    if ty.kind == super::types::TritonTypeKind::Tensor {
                        format!("{}_ptr", name)
                    } else {
                        name.clone()
                    }
                } else {
                    name.clone()
                }
            }
            
            Expression::Path(path) => {
                // 生成路径
                let segments: Vec<String> = path.segments.iter()
                    .map(|s| s.ident.clone())
                    .collect();
                segments.join(".")
            }
            
            Expression::Binary { left, op, right } => {
                let lhs = self.generate_expression(left);
                let rhs = self.generate_expression(right);
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
                    BinaryOp::And => "and",
                    BinaryOp::Or => "or",
                };
                format!("({} {} {})", lhs, op_str, rhs)
            }
            
            Expression::Call { func, args } => {
                let func_name = self.get_func_name(func);
                let args_code: Vec<String> = args.iter()
                    .map(|a| self.generate_expression(a))
                    .collect();
                format!("{}({})", func_name, args_code.join(", "))
            }
            
            Expression::FieldAccess { obj, field } => {
                let obj_code = self.generate_expression(obj);
                format!("{}.{}", obj_code, field)
            }
            
            Expression::Index { obj, index } => {
                let obj_code = self.generate_expression(obj);
                let idx_code = self.generate_expression(index);
                
                // 如果 obj 是 Buffer，生成 tl.load
                if let Expression::Identifier(name) = obj.as_ref() {
                    if self.var_types.contains_key(name) {
                        let ty = self.var_types.get(name).unwrap();
                        if ty.kind == super::types::TritonTypeKind::Tensor {
                            // 生成带掩码的加载
                            return format!("tl.load({}_ptr + offsets, mask=mask)", name);
                        }
                    }
                }
                format!("{}[{}]", obj_code, idx_code)
            }
            
            Expression::MethodCall { obj, method, args } => {
                let obj_code = self.generate_expression(obj);
                let args_code: Vec<String> = args.iter()
                    .map(|a| self.generate_expression(a))
                    .collect();
                
                match method.as_str() {
                    "zeros" => {
                        // Buffer::zeros([shape]) -> 分配内存
                        format!("torch.zeros({}, device='cuda')", args_code.join(", "))
                    }
                    "move_to" => {
                        // 移动到设备
                        format!("{}.to('cuda')", obj_code)
                    }
                    _ => format!("{}.{}({})", obj_code, method, args_code.join(", ")),
                }
            }
            
            Expression::PlaceOn { expr, device } => {
                let expr_code = self.generate_expression(expr);
                let _device_code = self.generate_expression(device);
                format!("{}.to('cuda')", expr_code)
            }
            
            Expression::MoveTo { expr, device } => {
                let expr_code = self.generate_expression(expr);
                let _device_code = self.generate_expression(device);
                format!("{}.to('cuda')", expr_code)
            }
            
            Expression::Await(expr) => {
                self.generate_expression(expr)
            }
            
            Expression::Array(elems) => {
                let elems_code: Vec<String> = elems.iter()
                    .map(|e| self.generate_expression(e))
                    .collect();
                format!("[{}]", elems_code.join(", "))
            }
            
            Expression::Spawn { device, task, await_ } => {
                // 生成内核启动调用
                let task_name = self.get_func_name(task);
                let _device_code = device.as_ref()
                    .map(|d| self.generate_expression(d))
                    .unwrap_or_default();
                
                // 这应该在启动函数中处理
                format!("# spawn {} await={}", task_name, await_)
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

    /// 生成启动函数
    fn generate_launch_function(&mut self, task: &Task) {
        let kernel_name = format!("{}_kernel", task.name);
        let launch_name = format!("launch_{}", task.name);
        
        // 收集参数
        let mut params = Vec::new();
        let mut call_args = Vec::new();
        
        for param in &task.params {
            params.push(format!("{}: torch.Tensor", param.name));
            call_args.push(param.name.clone());
        }
        
        // 添加 n_elements 参数
        params.push("n_elements: int".to_string());
        
        self.emitln(&format!("def {}({}):", launch_name, params.join(", ")));
        self.indent_inc();
        
        // Grid 配置
        self.emitln("grid = lambda meta: (triton.cdiv(n_elements, meta['BLOCK_SIZE']),)");
        self.emitln("");
        
        // 内核调用
        let args_str = call_args.join(", ");
        self.emitln(&format!("{}[grid]({}, n_elements, BLOCK_SIZE=1024)", 
            kernel_name, args_str));
        
        self.indent_dec();
        self.emitln("");
    }

    /// 生成主函数包装
    fn generate_main_wrapper(&mut self, main_func: &Function, tasks: &[Task]) {
        self.emitln("def main():");
        self.indent_inc();
        
        // 简单示例：创建数据并调用内核
        self.emitln("n = 1024");
        self.emitln("a = torch.randn(n, device='cuda')");
        self.emitln("b = torch.randn(n, device='cuda')");
        self.emitln("c = torch.empty_like(a)");
        self.emitln("");
        
        // 调用第一个任务的启动函数
        if let Some(task) = tasks.first() {
            self.emitln(&format!("launch_{}(a, b, c, n)", task.name));
        }
        
        self.emitln("print(c)");
        self.emitln("return c");
        
        self.indent_dec();
        self.emitln("");
        
        // 入口点
        self.emitln("if __name__ == '__main__':");
        self.indent_inc();
        self.emitln("main()");
        self.indent_dec();
    }

    /// 获取生成的内核列表
    pub fn kernels(&self) -> &[TritonKernel] {
        &self.kernels
    }
}

impl Default for TritonCodeGenerator {
    fn default() -> Self {
        Self::new()
    }
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
    fn test_empty_program() {
        let source = "";
        let program = parse_program(source);
        
        let mut r#gen = TritonCodeGenerator::new();
        let code = r#gen.generate(&program);
        
        assert!(code.contains("import triton"));
    }

    #[test]
    fn test_simple_task() {
        let source = r#"
task vector_add {
    body(a: Buffer<f32>, b: Buffer<f32>) -> Buffer<f32> {
        parallel for i in 0..1024 {
            let sum = a[i] + b[i];
        }
    }
}
"#;
        let program = parse_program(source);
        
        let mut r#gen = TritonCodeGenerator::new();
        let code = r#gen.generate(&program);
        
        assert!(code.contains("@triton.jit"));
        assert!(code.contains("def vector_add_kernel"));
        assert!(code.contains("tl.program_id"));
    }

    #[test]
    fn test_kernel_generation() {
        let source = r#"
task compute {
    body(x: Buffer<f32>, y: Buffer<f32>) -> Buffer<f32> {
        parallel for i in 0..N {
            let result = x[i] * 2.0 + y[i];
        }
    }
}
"#;
        let program = parse_program(source);
        
        let mut r#gen = TritonCodeGenerator::new();
        let code = r#gen.generate(&program);
        
        // 检查基本结构
        assert!(code.contains("import torch"));
        assert!(code.contains("import triton"));
        assert!(code.contains("tl.arange"));
    }
}
