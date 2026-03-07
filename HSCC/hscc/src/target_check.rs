//! 目标特定检查模块
//!
//! 提供针对不同目标设备（GPU、FPGA、NPU）的特定约束检查，包括：
//! - GPU：线程块限制、共享内存、同步约束
//! - FPGA：流水线约束、资源限制、突发访问
//! - NPU：算子支持、数据布局、精度要求

use crate::ast::*;
use crate::diagnostic::{Diagnostic, DiagnosticCollector, DiagnosticTag, SourceSpan, error_codes};
use std::collections::HashMap;

// ============================================================================
// GPU 约束检查
// ============================================================================

/// GPU 设备规格
#[derive(Debug, Clone)]
pub struct GpuSpec {
    /// 每块最大线程数
    pub max_threads_per_block: usize,
    /// 每网格最大块数
    pub max_blocks_per_grid: usize,
    /// 共享内存大小 (字节)
    pub shared_memory_size: usize,
    /// 寄存器数量
    pub registers_per_block: usize,
    /// Warp 大小
    pub warp_size: usize,
    /// 最大维度
    pub max_grid_dims: [usize; 3],
    /// 计算能力
    pub compute_capability: (usize, usize),
}

impl Default for GpuSpec {
    fn default() -> Self {
        Self {
            max_threads_per_block: 1024,
            max_blocks_per_grid: 2147483647,
            shared_memory_size: 49152,
            registers_per_block: 65536,
            warp_size: 32,
            max_grid_dims: [2147483647, 65535, 65535],
            compute_capability: (7, 5),
        }
    }
}

/// GPU 约束检查器
pub struct GpuConstraintChecker {
    /// 设备规格
    spec: GpuSpec,
}

impl GpuConstraintChecker {
    /// 创建新的检查器
    pub fn new() -> Self {
        Self {
            spec: GpuSpec::default(),
        }
    }

    /// 使用自定义规格
    pub fn with_spec(spec: GpuSpec) -> Self {
        Self { spec }
    }

    /// 检查任务
    pub fn check_task(&self, task: &Task, collector: &mut DiagnosticCollector) {
        // 检查并行循环的线程配置
        self.check_parallel_loop(&task.body, collector);

        // 检查同步约束
        self.check_sync_constraints(&task.body, collector);

        // 检查共享内存使用
        self.check_shared_memory(&task.body, collector);
    }

    /// 检查并行循环
    fn check_parallel_loop(&self, block: &Block, collector: &mut DiagnosticCollector) {
        for stmt in &block.statements {
            if let Statement::ParallelFor { range, body, .. } = stmt {
                // 检查迭代范围是否合理
                if let (Expression::Integer(start), Expression::Integer(end)) = (&range.0, &range.1) {
                    let iterations = (end - start) as usize;
                    
                    // 警告：迭代次数超过最大线程数
                    if iterations > self.spec.max_threads_per_block * self.spec.max_blocks_per_grid {
                        collector.add(
                            Diagnostic::warning(error_codes::GPU_THREAD_LIMIT_EXCEEDED)
                                .message(format!(
                                    "Parallel loop has {} iterations, which may exceed GPU capacity",
                                    iterations
                                ))
                                .tag(DiagnosticTag::Performance)
                        );
                    }
                }

                // 递归检查嵌套循环
                self.check_parallel_loop(body, collector);
            }

            // 检查嵌套语句
            match stmt {
                Statement::If { then_branch, else_branch, .. } => {
                    self.check_parallel_loop(then_branch, collector);
                    if let Some(else_block) = else_branch {
                        self.check_parallel_loop(else_block, collector);
                    }
                }
                Statement::For { body, .. } |
                Statement::While { body, .. } |
                Statement::Loop(body) => {
                    self.check_parallel_loop(body, collector);
                }
                _ => {}
            }
        }
    }

    /// 检查同步约束
    fn check_sync_constraints(&self, block: &Block, collector: &mut DiagnosticCollector) {
        // 检查条件分支中的同步
        self.check_sync_in_conditional(block, false, collector);
    }

    /// 检查条件分支中的同步
    fn check_sync_in_conditional(&self, block: &Block, in_conditional: bool, collector: &mut DiagnosticCollector) {
        for stmt in &block.statements {
            match stmt {
                Statement::Expr(expr) => {
                    if in_conditional && self.contains_sync_call(expr) {
                        collector.add(
                            Diagnostic::warning(error_codes::GPU_SYNC_IN_CONDITIONAL)
                                .message("Synchronization inside conditional may cause deadlock")
                                .tag(DiagnosticTag::Correctness)
                        );
                    }
                }
                Statement::If { then_branch, else_branch, .. } => {
                    self.check_sync_in_conditional(then_branch, true, collector);
                    if let Some(else_block) = else_branch {
                        self.check_sync_in_conditional(else_block, true, collector);
                    }
                }
                Statement::For { body, .. } |
                Statement::ParallelFor { body, .. } |
                Statement::While { body, .. } |
                Statement::Loop(body) => {
                    self.check_sync_in_conditional(body, in_conditional, collector);
                }
                _ => {}
            }
        }
    }

    /// 检查表达式是否包含同步调用
    fn contains_sync_call(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Call { func, .. } => {
                if let Expression::Identifier(name) = func.as_ref() {
                    name == "sync_threads" || name == "__syncthreads" || name == "barrier"
                } else {
                    false
                }
            }
            Expression::MethodCall { method, .. } => {
                method == "sync" || method == "barrier"
            }
            _ => false,
        }
    }

    /// 检查共享内存使用
    fn check_shared_memory(&self, block: &Block, collector: &mut DiagnosticCollector) {
        let shared_mem_usage = self.estimate_shared_memory(block);

        if shared_mem_usage > self.spec.shared_memory_size {
            collector.add(
                Diagnostic::error(error_codes::GPU_SHARED_MEMORY_EXCEEDED)
                    .message(format!(
                        "Shared memory usage ({}) exceeds limit ({})",
                        shared_mem_usage, self.spec.shared_memory_size
                    ))
                    .tag(DiagnosticTag::Correctness)
            );
        }
    }

    /// 估算共享内存使用量
    fn estimate_shared_memory(&self, block: &Block) -> usize {
        let mut total = 0;

        for stmt in &block.statements {
            if let Statement::Let { ty, init, .. } = stmt {
                // 检查是否是共享内存声明
                if let Some(Type::Buffer(_, Some(size))) = ty {
                    // 假设 f32 类型
                    total += size * 4;
                }
                if let Some(expr) = init {
                    total += self.estimate_expr_memory(expr);
                }
            }
        }

        total
    }

    /// 估算表达式的内存使用
    fn estimate_expr_memory(&self, expr: &Expression) -> usize {
        match expr {
            Expression::Array(elems) => elems.len() * 4, // 假设 f32
            Expression::MethodCall { obj, method, args } => {
                if method == "shared" || method == "__shared__" {
                    // 共享内存声明
                    if let Some(arg) = args.first() {
                        if let Expression::Integer(size) = arg {
                            return (*size as usize) * 4;
                        }
                    }
                }
                self.estimate_expr_memory(obj)
            }
            _ => 0,
        }
    }
}

impl Default for GpuConstraintChecker {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// FPGA 约束检查
// ============================================================================

/// FPGA 设备规格
#[derive(Debug, Clone)]
pub struct FpgaSpec {
    /// DSP 数量
    pub dsp_count: usize,
    /// BRAM 数量
    pub bram_count: usize,
    /// LUT 数量
    pub lut_count: usize,
    /// 最大流水线深度
    pub max_pipeline_depth: usize,
    /// 时钟频率 (MHz)
    pub clock_frequency: usize,
}

impl Default for FpgaSpec {
    fn default() -> Self {
        Self {
            dsp_count: 1024,
            bram_count: 1024,
            lut_count: 500000,
            max_pipeline_depth: 64,
            clock_frequency: 300,
        }
    }
}

/// FPGA 约束检查器
pub struct FpgaConstraintChecker {
    spec: FpgaSpec,
}

impl FpgaConstraintChecker {
    pub fn new() -> Self {
        Self {
            spec: FpgaSpec::default(),
        }
    }

    pub fn with_spec(spec: FpgaSpec) -> Self {
        Self { spec }
    }

    /// 检查任务
    pub fn check_task(&self, task: &Task, collector: &mut DiagnosticCollector) {
        // 检查流水线约束
        self.check_pipeline_constraints(&task.body, collector);

        // 检查循环可流水线化
        self.check_loop_pipelineability(&task.body, collector);

        // 检查内存访问模式
        self.check_memory_access_pattern(&task.body, collector);
    }

    /// 检查流水线约束
    fn check_pipeline_constraints(&self, block: &Block, collector: &mut DiagnosticCollector) {
        for stmt in &block.statements {
            if let Statement::ParallelFor { body, .. } = stmt {
                // 检查循环体是否可以流水线化
                if self.has_loop_carried_dependency(body) {
                    collector.add(
                        Diagnostic::warning(error_codes::FPGA_PIPELINE_DEPENDENCY)
                            .message("Loop has loop-carried dependency, may not be pipelineable")
                            .tag(DiagnosticTag::Performance)
                    );
                }
            }
        }
    }

    /// 检查是否有循环携带依赖
    fn has_loop_carried_dependency(&self, block: &Block) -> bool {
        // 简化检查：查找先写后读的模式
        let mut writes = Vec::new();
        
        for stmt in &block.statements {
            if let Statement::Let { init, .. } = stmt {
                if let Some(expr) = init {
                    self.collect_writes(expr, &mut writes);
                }
            }
            if let Statement::Expr(expr) = stmt {
                self.collect_reads(expr, &writes);
                if self.has_raw_dependency(expr, &writes) {
                    return true;
                }
            }
        }

        false
    }

    fn collect_writes(&self, expr: &Expression, writes: &mut Vec<String>) {
        if let Expression::Index { obj, .. } = expr {
            if let Expression::Identifier(name) = obj.as_ref() {
                writes.push(name.clone());
            }
        }
    }

    fn collect_reads(&self, expr: &Expression, _writes: &[String]) {
        // 简化实现
        let _ = expr;
    }

    fn has_raw_dependency(&self, _expr: &Expression, _writes: &[String]) -> bool {
        // 简化实现
        false
    }

    /// 检查循环流水线化
    fn check_loop_pipelineability(&self, block: &Block, collector: &mut DiagnosticCollector) {
        for stmt in &block.statements {
            if let Statement::For { body, .. } = stmt {
                // 嵌套循环可能影响流水线化
                if self.has_nested_loops(body) {
                    collector.add(
                        Diagnostic::note("HSC0000")
                            .message("Nested loops may require loop flattening for optimal FPGA performance")
                            .tag(DiagnosticTag::Performance)
                    );
                }
            }
        }
    }

    fn has_nested_loops(&self, block: &Block) -> bool {
        for stmt in &block.statements {
            if matches!(stmt, Statement::For { .. } | Statement::ParallelFor { .. }) {
                return true;
            }
        }
        false
    }

    /// 检查内存访问模式
    fn check_memory_access_pattern(&self, block: &Block, collector: &mut DiagnosticCollector) {
        for stmt in &block.statements {
            if let Statement::ParallelFor { body, .. } = stmt {
                // 检查是否适合突发传输
                if !self.is_sequential_access(body) {
                    collector.add(
                        Diagnostic::warning(error_codes::FPGA_UNSUPPORTED_OPERATION)
                            .message("Non-sequential memory access may reduce FPGA performance")
                            .tag(DiagnosticTag::Performance)
                    );
                }
            }
        }
    }

    fn is_sequential_access(&self, _block: &Block) -> bool {
        // 简化实现
        true
    }
}

impl Default for FpgaConstraintChecker {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// NPU 约束检查
// ============================================================================

/// NPU 设备规格
#[derive(Debug, Clone)]
pub struct NpuSpec {
    /// 支持的操作列表
    pub supported_ops: Vec<String>,
    /// 支持的数据布局
    pub supported_layouts: Vec<String>,
    /// 支持的数据类型
    pub supported_dtypes: Vec<String>,
    /// 最大张量维度
    pub max_tensor_dims: usize,
    /// SIMD 宽度
    pub simd_width: usize,
}

impl Default for NpuSpec {
    fn default() -> Self {
        Self {
            supported_ops: vec![
                "conv2d", "matmul", "relu", "sigmoid", "tanh",
                "max_pool", "avg_pool", "batch_norm", "softmax",
                "add", "sub", "mul", "div", "concat", "reshape",
            ].iter().map(|s| s.to_string()).collect(),
            supported_layouts: vec!["NCHW", "NHWC"].iter().map(|s| s.to_string()).collect(),
            supported_dtypes: vec!["f32", "f16", "i8", "bf16"].iter().map(|s| s.to_string()).collect(),
            max_tensor_dims: 4,
            simd_width: 16,
        }
    }
}

/// NPU 约束检查器
pub struct NpuConstraintChecker {
    spec: NpuSpec,
}

impl NpuConstraintChecker {
    pub fn new() -> Self {
        Self {
            spec: NpuSpec::default(),
        }
    }

    pub fn with_spec(spec: NpuSpec) -> Self {
        Self { spec }
    }

    /// 检查任务
    pub fn check_task(&self, task: &Task, collector: &mut DiagnosticCollector) {
        // 检查算子支持
        self.check_operator_support(&task.body, collector);

        // 检查数据类型
        self.check_data_types(&task.body, collector);

        // 检查张量维度
        self.check_tensor_dimensions(&task.body, collector);
    }

    /// 检查算子支持
    fn check_operator_support(&self, block: &Block, collector: &mut DiagnosticCollector) {
        for stmt in &block.statements {
            if let Statement::Expr(expr) = stmt {
                self.check_expr_operator(expr, collector);
            }
        }
    }

    fn check_expr_operator(&self, expr: &Expression, collector: &mut DiagnosticCollector) {
        match expr {
            Expression::Call { func, .. } => {
                if let Expression::Identifier(name) = func.as_ref() {
                    if !self.spec.supported_ops.contains(name) {
                        collector.add(
                            Diagnostic::warning(error_codes::NPU_UNSUPPORTED_OPERATOR)
                                .message(format!(
                                    "Operator '{}' may not be supported by NPU (supported: {})",
                                    name,
                                    self.spec.supported_ops.join(", ")
                                ))
                                .tag(DiagnosticTag::Portability)
                        );
                    }
                }
            }
            Expression::Binary { .. } => {
                // 基本算术操作通常是支持的
            }
            Expression::MethodCall { method, .. } => {
                // 检查方法调用
                let _ = method;
            }
            _ => {}
        }
    }

    /// 检查数据类型
    fn check_data_types(&self, block: &Block, collector: &mut DiagnosticCollector) {
        for stmt in &block.statements {
            if let Statement::Let { ty, .. } = stmt {
                if let Some(t) = ty {
                    let type_str = self.type_to_string(t);
                    if !self.spec.supported_dtypes.contains(&type_str) {
                        collector.add(
                            Diagnostic::warning(error_codes::NPU_PRECISION_NOT_SUPPORTED)
                                .message(format!(
                                    "Data type '{}' may not be optimal for NPU (recommended: {})",
                                    type_str,
                                    self.spec.supported_dtypes.join(", ")
                                ))
                                .tag(DiagnosticTag::Performance)
                        );
                    }
                }
            }
        }
    }

    fn type_to_string(&self, ty: &Type) -> String {
        match ty {
            Type::F32 => "f32".to_string(),
            Type::F64 => "f64".to_string(),
            Type::I32 => "i32".to_string(),
            Type::I64 => "i64".to_string(),
            Type::I8 => "i8".to_string(),
            Type::Buffer(inner, _) => self.type_to_string(inner),
            _ => "unknown".to_string(),
        }
    }

    /// 检查张量维度
    fn check_tensor_dimensions(&self, block: &Block, collector: &mut DiagnosticCollector) {
        for stmt in &block.statements {
            if let Statement::Let { ty, .. } = stmt {
                if let Some(Type::Buffer(_, Some(dims))) = ty {
                    if *dims > self.spec.max_tensor_dims {
                        collector.add(
                            Diagnostic::warning(error_codes::NPU_DATA_LAYOUT_MISMATCH)
                                .message(format!(
                                    "Tensor with {} dimensions may not be optimal for NPU (max: {})",
                                    dims, self.spec.max_tensor_dims
                                ))
                                .tag(DiagnosticTag::Performance)
                        );
                    }
                }
            }
        }
    }
}

impl Default for NpuConstraintChecker {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 统一的目标检查接口
// ============================================================================

/// 目标设备类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetDevice {
    Gpu,
    Fpga,
    Npu,
    Cpu,
}

/// 目标检查器
pub struct TargetChecker {
    device: TargetDevice,
    gpu_checker: GpuConstraintChecker,
    fpga_checker: FpgaConstraintChecker,
    npu_checker: NpuConstraintChecker,
}

impl TargetChecker {
    pub fn new(device: TargetDevice) -> Self {
        Self {
            device,
            gpu_checker: GpuConstraintChecker::new(),
            fpga_checker: FpgaConstraintChecker::new(),
            npu_checker: NpuConstraintChecker::new(),
        }
    }

    /// 检查任务
    pub fn check_task(&self, task: &Task, collector: &mut DiagnosticCollector) {
        match self.device {
            TargetDevice::Gpu => self.gpu_checker.check_task(task, collector),
            TargetDevice::Fpga => self.fpga_checker.check_task(task, collector),
            TargetDevice::Npu => self.npu_checker.check_task(task, collector),
            TargetDevice::Cpu => {
                // CPU 通常没有特殊约束
            }
        }
    }

    /// 从字符串解析目标设备
    pub fn from_str(s: &str) -> Self {
        let device = match s.to_lowercase().as_str() {
            "cuda" | "gpu" => TargetDevice::Gpu,
            "fpga" => TargetDevice::Fpga,
            "npu" => TargetDevice::Npu,
            _ => TargetDevice::Cpu,
        };
        Self::new(device)
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_spec_default() {
        let spec = GpuSpec::default();
        assert_eq!(spec.max_threads_per_block, 1024);
        assert_eq!(spec.warp_size, 32);
    }

    #[test]
    fn test_gpu_checker_sync_detection() {
        let checker = GpuConstraintChecker::new();
        let mut collector = DiagnosticCollector::new();

        let source = r#"
task test {
    body() {
        if true {
            sync_threads();
        }
    }
}
"#;
        let mut lexer = crate::lexer::Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = crate::parser::Parser::new(tokens);
        let ast = parser.parse_program().expect("Parse failed");

        if let Some(task) = ast.tasks.first() {
            checker.check_task(task, &mut collector);
            // 应该检测到条件分支中的同步
            assert!(collector.has_warnings());
        }
    }

    #[test]
    fn test_fpga_spec_default() {
        let spec = FpgaSpec::default();
        assert!(spec.dsp_count > 0);
        assert!(spec.max_pipeline_depth > 0);
    }

    #[test]
    fn test_npu_spec_default() {
        let spec = NpuSpec::default();
        assert!(spec.supported_ops.contains(&"conv2d".to_string()));
        assert!(spec.supported_layouts.contains(&"NCHW".to_string()));
    }

    #[test]
    fn test_target_checker_from_str() {
        let gpu_checker = TargetChecker::from_str("cuda");
        assert_eq!(gpu_checker.device, TargetDevice::Gpu);

        let fpga_checker = TargetChecker::from_str("fpga");
        assert_eq!(fpga_checker.device, TargetDevice::Fpga);

        let npu_checker = TargetChecker::from_str("npu");
        assert_eq!(npu_checker.device, TargetDevice::Npu);
    }
}
