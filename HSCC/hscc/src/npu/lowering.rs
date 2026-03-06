//! AST 到 NPU IR 转换
//!
//! 将 HSCLang AST 转换为 NPU 计算图。

use crate::ast::{self, Expression, BinaryOp};
use crate::npu::types::{NpuType, TensorLayout};
use crate::npu::graph::{
    NpuGraph, NpuOperation, NpuOpType, NpuTensor, OpHints, Padding,
};
use crate::npu::backends::{NpuBackend, NpuDevice, NpuHardwareSpec, NpuError};

/// AST 到 NPU IR 转换器
pub struct NpuLowering {
    /// 目标后端
    backend: Box<dyn NpuBackend>,
    /// 硬件规格
    spec: NpuHardwareSpec,
    /// 当前图
    graph: NpuGraph,
    /// 张量计数器
    tensor_counter: u64,
    /// 操作计数器
    op_counter: usize,
}

impl NpuLowering {
    /// 创建新的转换器
    pub fn new(backend: Box<dyn NpuBackend>, device: NpuDevice) -> Self {
        let spec = backend.hardware_spec(device);
        Self {
            backend,
            spec,
            graph: NpuGraph::default(),
            tensor_counter: 0,
            op_counter: 0,
        }
    }

    /// 转换 AST 程序
    pub fn lower_program(&mut self, program: &ast::Program) -> Result<NpuGraph, NpuError> {
        // 初始化图（使用第一个任务的名称）
        let program_name = program.tasks.first()
            .map(|t| t.name.clone())
            .unwrap_or_else(|| "program".to_string());

        self.graph = NpuGraph::new(&program_name);

        // 转换任务（主要计算逻辑）
        for task in &program.tasks {
            self.lower_task(task)?;
        }

        // 转换函数
        for func in &program.functions {
            if func.name == "main" {
                self.lower_main_function(func)?;
            }
        }

        // 计算生命周期
        self.graph.compute_lifetimes();

        // 验证
        self.graph.validate()
            .map_err(|e| NpuError::GraphValidationFailed {
                reason: e.to_string(),
            })?;

        // 转移图的所有权
        Ok(std::mem::take(&mut self.graph))
    }

    /// 转换任务
    fn lower_task(&mut self, task: &ast::Task) -> Result<(), NpuError> {
        // 设置图名称（使用第一个任务）
        if self.graph.name.is_empty() {
            self.graph.name = task.name.clone();
        }

        // 转换参数为输入
        for param in &task.params {
            let dtype = NpuType::from(&param.ty);
            let shape = self.infer_shape(&param.ty);
            self.add_input(&param.name, dtype, shape);
        }

        // 转换任务体
        self.lower_block(&task.body)?;

        Ok(())
    }

    /// 转换 main 函数
    fn lower_main_function(&mut self, func: &ast::Function) -> Result<(), NpuError> {
        // 转换函数体
        self.lower_block(&func.body)?;
        Ok(())
    }

    /// 转换代码块
    fn lower_block(&mut self, block: &ast::Block) -> Result<(), NpuError> {
        for stmt in &block.statements {
            self.lower_statement(stmt)?;
        }
        Ok(())
    }

    /// 转换语句
    fn lower_statement(&mut self, stmt: &ast::Statement) -> Result<(), NpuError> {
        match stmt {
            ast::Statement::Let { name, ty, init, .. } => {
                let dtype = ty.as_ref()
                    .map(|t| NpuType::from(t))
                    .unwrap_or_else(|| NpuType::f32());

                let shape = ty.as_ref()
                    .map(|t| self.infer_shape(t))
                    .unwrap_or_default();

                // 添加张量
                let tensor = NpuTensor {
                    id: self.tensor_counter,
                    name: name.clone(),
                    dtype,
                    shape,
                    layout: TensorLayout::NCHW,
                    memory_offset: None,
                    requires_quantization: false,
                    quant_params: None,
                    lifetime_start: self.op_counter,
                    lifetime_end: self.op_counter,
                };
                self.graph.add_tensor(tensor);
                self.tensor_counter += 1;

                // 处理初始化表达式
                if let Some(init_expr) = init {
                    self.lower_expression(init_expr)?;
                }
            }

            ast::Statement::Return(expr) => {
                if let Some(expr) = expr {
                    let result = self.lower_expression(expr)?;
                    // 添加为输出
                    if let Some(tensor) = self.graph.tensors.get(&result) {
                        self.add_output(&result, tensor.dtype.clone(), tensor.shape.clone());
                    }
                }
            }

            ast::Statement::Expr(expr) => {
                self.lower_expression(expr)?;
            }

            ast::Statement::ParallelFor { var, range, body } => {
                self.lower_parallel_for(var, range, body)?;
            }

            ast::Statement::For { var, range, body } => {
                self.lower_for_loop(var, range, body)?;
            }

            ast::Statement::If { condition, then_branch, else_branch } => {
                self.lower_expression(condition)?;
                self.lower_block(then_branch)?;
                if let Some(else_block) = else_branch {
                    self.lower_block(else_block)?;
                }
            }

            ast::Statement::While { condition, body } => {
                self.lower_expression(condition)?;
                self.lower_block(body)?;
            }

            _ => {}
        }

        Ok(())
    }

    /// 转换表达式
    fn lower_expression(&mut self, expr: &Expression) -> Result<String, NpuError> {
        match expr {
            Expression::Identifier(name) => {
                Ok(name.clone())
            }

            Expression::Integer(value) => {
                let result = self.new_tensor_name();
                self.add_constant(&result, NpuType::i64(), vec![]);
                Ok(result)
            }

            Expression::Float(value) => {
                let result = self.new_tensor_name();
                self.add_constant(&result, NpuType::f32(), vec![]);
                Ok(result)
            }

            Expression::Bool(value) => {
                let result = self.new_tensor_name();
                self.add_constant(&result, NpuType::bool(), vec![]);
                Ok(result)
            }

            Expression::Binary { left, op, right } => {
                let lhs = self.lower_expression(left)?;
                let rhs = self.lower_expression(right)?;
                let result = self.new_tensor_name();

                let op_type = match op {
                    BinaryOp::Add => NpuOpType::Add,
                    BinaryOp::Sub => NpuOpType::Sub,
                    BinaryOp::Mul => NpuOpType::Mul,
                    BinaryOp::Div => NpuOpType::Div,
                    _ => return Err(NpuError::UnsupportedOp {
                        op: format!("{:?}", op),
                        reason: "Binary operation not supported for NPU".to_string(),
                    }),
                };

                let operation = NpuOperation {
                    index: self.op_counter,
                    op_type,
                    name: format!("binary_{}", self.op_counter),
                    inputs: vec![lhs, rhs],
                    outputs: vec![result.clone()],
                    attributes: Default::default(),
                    hints: OpHints::default(),
                };

                self.graph.add_operation(operation);
                self.op_counter += 1;

                Ok(result)
            }

            Expression::Call { func, args } => {
                self.lower_call(func, args)
            }

            Expression::Index { obj, index } => {
                let base = self.lower_expression(obj)?;
                let idx = self.lower_expression(index)?;
                let result = self.new_tensor_name();

                let operation = NpuOperation {
                    index: self.op_counter,
                    op_type: NpuOpType::Gather { axis: 0 },
                    name: format!("gather_{}", self.op_counter),
                    inputs: vec![base, idx],
                    outputs: vec![result.clone()],
                    attributes: Default::default(),
                    hints: OpHints::default(),
                };

                self.graph.add_operation(operation);
                self.op_counter += 1;

                Ok(result)
            }

            Expression::MethodCall { obj, method, args } => {
                let base = self.lower_expression(obj)?;

                // 处理常见方法
                match method.as_str() {
                    "zeros" | "ones" => {
                        Ok(base)
                    }
                    "move_to" => {
                        // 数据迁移
                        Ok(base)
                    }
                    _ => {
                        Ok(base)
                    }
                }
            }

            Expression::Spawn { task, device, await_ } => {
                let task_name = self.lower_expression(task)?;
                let result = self.new_tensor_name();

                // spawn 操作
                let operation = NpuOperation {
                    index: self.op_counter,
                    op_type: NpuOpType::Identity, // 占位
                    name: format!("spawn_{}", self.op_counter),
                    inputs: vec![task_name],
                    outputs: vec![result.clone()],
                    attributes: Default::default(),
                    hints: OpHints {
                        device_hint: device.as_ref().map(|d| format!("{:?}", d)),
                        ..Default::default()
                    },
                };

                self.graph.add_operation(operation);
                self.op_counter += 1;

                Ok(result)
            }

            _ => {
                // 其他表达式类型
                let result = self.new_tensor_name();
                Ok(result)
            }
        }
    }

    /// 转换函数调用
    fn lower_call(&mut self, func: &Expression, args: &[Expression]) -> Result<String, NpuError> {
        let result = self.new_tensor_name();

        // 获取函数名
        let func_name = match func {
            Expression::Identifier(name) => name.clone(),
            Expression::Path(path) => {
                path.segments.last()
                    .map(|s| s.ident.clone())
                    .unwrap_or_default()
            }
            _ => return Ok(result),
        };

        // 映射内置函数到 NPU 操作
        let op_type = match func_name.as_str() {
            "matmul" | "mat_mul" => NpuOpType::MatMul,
            "relu" => NpuOpType::ReLU,
            "sigmoid" => NpuOpType::Sigmoid,
            "tanh" => NpuOpType::Tanh,
            "softmax" => NpuOpType::Softmax { axis: -1 },
            "gelu" => NpuOpType::GELU,
            "layer_norm" => NpuOpType::LayerNorm { epsilon: 1e-5, axis: -1 },
            "exp" => NpuOpType::Exp,
            "log" => NpuOpType::Log,
            "sqrt" => NpuOpType::Sqrt,
            "sum" => NpuOpType::ReduceSum { axes: vec![], keep_dims: false },
            "mean" => NpuOpType::ReduceMean { axes: vec![], keep_dims: false },
            "concat" | "cat" => NpuOpType::Concat { axis: 0 },
            "reshape" | "view" => NpuOpType::Reshape,
            _ => {
                // 未知函数，创建通用调用
                NpuOpType::Custom { op_name: func_name.clone() }
            }
        };

        // 转换参数
        let mut input_names = Vec::new();
        for arg in args {
            let name = self.lower_expression(arg)?;
            input_names.push(name);
        }

        let operation = NpuOperation {
            index: self.op_counter,
            op_type,
            name: format!("{}_{}", func_name, self.op_counter),
            inputs: input_names,
            outputs: vec![result.clone()],
            attributes: Default::default(),
            hints: OpHints::default(),
        };

        self.graph.add_operation(operation);
        self.op_counter += 1;

        Ok(result)
    }

    /// 转换 parallel for
    fn lower_parallel_for(
        &mut self,
        var: &str,
        range: &(Expression, Expression),
        body: &ast::Block,
    ) -> Result<(), NpuError> {
        // 对于 NPU，parallel for 通常会被转换为张量操作
        // 这里我们简单地展开循环体

        // TODO: 实现模式识别，将 parallel for 转换为 MatMul/Conv 等操作

        self.lower_block(body)?;

        Ok(())
    }

    /// 转换普通 for 循环
    fn lower_for_loop(
        &mut self,
        var: &str,
        range: &(Expression, Expression),
        body: &ast::Block,
    ) -> Result<(), NpuError> {
        // 对于 NPU，for 循环通常会被展开或转换为其他操作
        self.lower_block(body)?;
        Ok(())
    }

    // ========================================================================
    // 辅助方法
    // ========================================================================

    /// 添加输入
    fn add_input(&mut self, name: &str, dtype: NpuType, shape: Vec<i64>) {
        self.graph.add_input(name, dtype.clone(), shape.clone());

        let tensor = NpuTensor {
            id: self.tensor_counter,
            name: name.to_string(),
            dtype,
            shape,
            layout: TensorLayout::NCHW,
            memory_offset: None,
            requires_quantization: false,
            quant_params: None,
            lifetime_start: 0,
            lifetime_end: 0,
        };

        self.graph.add_tensor(tensor);
        self.tensor_counter += 1;
    }

    /// 添加输出
    fn add_output(&mut self, name: &str, dtype: NpuType, shape: Vec<i64>) {
        self.graph.add_output(name, dtype, shape);
    }

    /// 添加常量
    fn add_constant(&mut self, name: &str, dtype: NpuType, shape: Vec<i64>) {
        let tensor = NpuTensor {
            id: self.tensor_counter,
            name: name.to_string(),
            dtype,
            shape,
            layout: TensorLayout::NCHW,
            memory_offset: None,
            requires_quantization: false,
            quant_params: None,
            lifetime_start: 0,
            lifetime_end: 0,
        };

        self.graph.add_tensor(tensor);
        self.tensor_counter += 1;
    }

    /// 生成新的张量名称
    fn new_tensor_name(&mut self) -> String {
        let name = format!("tensor_{}", self.tensor_counter);
        self.tensor_counter += 1;
        name
    }

    /// 推断类型形状
    fn infer_shape(&self, ty: &ast::Type) -> Vec<i64> {
        match ty {
            ast::Type::Buffer(_, dim) => {
                dim.map(|d| vec![d as i64]).unwrap_or_default()
            }
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::npu::backends::intel_npu::IntelNpuBackend;
    use crate::npu::backends::intel_npu::IntelNpuGeneration;

    #[test]
    fn test_lowering_creation() {
        let backend = Box::new(IntelNpuBackend::new());
        let device = NpuDevice::IntelNPU(IntelNpuGeneration::MeteorLake);
        let lowering = NpuLowering::new(backend, device);

        assert_eq!(lowering.op_counter, 0);
    }
}