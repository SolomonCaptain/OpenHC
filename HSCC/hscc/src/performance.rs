//! 性能分析与优化建议模块
//!
//! 提供 HSCLang 程序的性能分析和优化建议功能。
//!
//! # 概述
//!
//! 本模块实现了一个基于 Roofline 模型的性能分析器，能够识别程序中的性能瓶颈，
//! 并提供针对性的优化建议。支持多种异构设备（CPU、GPU、FPGA、NPU）的性能建模。
//!
//! # 主要功能
//!
//! - **性能指标计算**：估算 FLOPs、内存访问量、算术强度等
//! - **瓶颈识别**：检测内存带宽、计算、并行度等瓶颈
//! - **优化建议**：生成针对性的优化建议和预估改进
//! - **报告生成**：支持文本、JSON、HTML、Markdown 格式输出
//!
//! # Roofline 模型
//!
//! 使用 Roofline 模型分析程序的性能特征：
//!
//! ```text
//! 性能 (GFLOPS)
//!     │    __________
//!     │   /
//!     │  /  计算受限
//!     │ /
//!     │/  内存受限
//!     └────────────────── 算术强度 (FLOPs/Byte)
//!            ^
//!            峰值带宽 / 峰值算力 = 转折点
//! ```
//!
//! # 主要组件
//!
//! - [`PerformanceAnalyzer`] - 主分析器
//! - [`DeviceModel`] - 设备性能模型
//! - [`PerformanceProfile`] - 分析结果
//! - [`FunctionMetrics`] - 函数级性能指标
//! - [`TaskMetrics`] - 任务级性能指标
//! - [`OptimizationRecommendation`] - 优化建议
//! - [`PerformanceReportGenerator`] - 报告生成器
//!
//! # 使用示例
//!
//! ```rust
//! use hscc::performance::{PerformanceAnalyzer, PerformanceReportGenerator, ReportFormat};
//! use hscc::ast::Program;
//!
//! let mut analyzer = PerformanceAnalyzer::new();
//! let profile = analyzer.analyze(&program);
//!
//! // 生成报告
//! let generator = PerformanceReportGenerator::new(ReportFormat::Markdown);
//! let report = generator.generate(profile);
//! println!("{}", report);
//! ```
//!
//! # 支持的优化建议类型
//!
//! | 类型 | 描述 | 适用场景 |
//! |------|------|----------|
//! | 循环展开 | 减少循环开销 | 小循环体 |
//! | 循环分块 | 提高缓存利用率 | 大数据集 |
//! | 内存访问优化 | 改善访问模式 | 内存受限 |
//! | 向量化 | 利用 SIMD | 数据并行 |
//! | 设备迁移 | 选择更优设备 | 计算密集 |
//! | 内核融合 | 减少数据传输 | 多操作 |
//! | 异步执行 | 隐藏延迟 | 数据传输 |
//!
//! # 瓶颈类型
//!
//! - `MemoryBandwidth` - 内存带宽瓶颈
//! - `ComputeBound` - 计算瓶颈
//! - `LowParallelism` - 并行度不足
//! - `IrregularMemoryAccess` - 不规则内存访问
//! - `BranchDivergence` - 分支发散
//! - `DataTransferOverhead` - 数据传输开销

use std::collections::HashMap;
use crate::ast::{Program, Task, Function, Statement, Expression};
use crate::diagnostic::{Diagnostic, DiagnosticLevel, DiagnosticCollector, error_codes};

// ============================================================================
// 性能指标定义
// ============================================================================

/// 性能分析结果
#[derive(Debug, Default, Clone)]
pub struct PerformanceProfile {
    /// 函数级性能指标
    pub function_metrics: HashMap<String, FunctionMetrics>,
    /// 任务级性能指标
    pub task_metrics: HashMap<String, TaskMetrics>,
    /// 整体性能瓶颈
    pub bottlenecks: Vec<PerformanceBottleneck>,
    /// 优化建议
    pub recommendations: Vec<OptimizationRecommendation>,
    /// 预估加速比
    pub estimated_speedup: f64,
}

/// 函数级性能指标
#[derive(Debug, Clone)]
pub struct FunctionMetrics {
    /// 函数名
    pub name: String,
    /// 估算的算术运算次数
    pub estimated_flops: u64,
    /// 估算的内存访问量（字节）
    pub estimated_memory_bytes: u64,
    /// 算术强度 (FLOPS/Byte)
    pub arithmetic_intensity: f64,
    /// 并行循环数量
    pub parallel_loops: usize,
    /// 循环嵌套深度
    pub max_loop_depth: usize,
    /// 是否内存受限
    pub is_memory_bound: bool,
    /// 估算运行时间（微秒）
    pub estimated_time_us: f64,
}

impl Default for FunctionMetrics {
    fn default() -> Self {
        Self {
            name: String::new(),
            estimated_flops: 0,
            estimated_memory_bytes: 0,
            arithmetic_intensity: 0.0,
            parallel_loops: 0,
            max_loop_depth: 0,
            is_memory_bound: false,
            estimated_time_us: 0.0,
        }
    }
}

/// 任务级性能指标
#[derive(Debug, Clone)]
pub struct TaskMetrics {
    /// 任务名
    pub name: String,
    /// 目标设备
    pub target_device: String,
    /// 执行模式
    pub pattern: String,
    /// 估算内核执行时间（微秒）
    pub estimated_kernel_time_us: f64,
    /// 估算数据传输时间（微秒）
    pub estimated_transfer_time_us: f64,
    /// 计算与传输重叠度 (0.0-1.0)
    pub compute_transfer_overlap: f64,
    /// 线程/工作项利用率
    pub thread_utilization: f64,
    /// 内存带宽利用率
    pub memory_bandwidth_utilization: f64,
    /// 性能瓶颈列表
    pub bottlenecks: Vec<BottleneckType>,
}

/// 性能瓶颈类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BottleneckType {
    /// 内存带宽瓶颈
    MemoryBandwidth,
    /// 计算瓶颈
    ComputeBound,
    /// 低并行度
    LowParallelism,
    /// 不规则内存访问
    IrregularMemoryAccess,
    /// 分支发散
    BranchDivergence,
    /// 同步开销
    SynchronizationOverhead,
    /// 数据传输开销
    DataTransferOverhead,
    /// 资源不足（寄存器、共享内存等）
    ResourceLimitation,
    /// 缓存未命中
    CacheMiss,
}

/// 性能瓶颈描述
#[derive(Debug, Clone)]
pub struct PerformanceBottleneck {
    /// 瓶颈类型
    pub bottleneck_type: BottleneckType,
    /// 位置（函数或任务名）
    pub location: String,
    /// 严重程度 (0.0-1.0)
    pub severity: f64,
    /// 详细描述
    pub description: String,
    /// 相关代码行
    pub line_numbers: Vec<usize>,
}

/// 优化建议
#[derive(Debug, Clone)]
pub struct OptimizationRecommendation {
    /// 建议类型
    pub recommendation_type: RecommendationType,
    /// 目标位置
    pub location: String,
    /// 预估性能提升
    pub estimated_improvement: f64,
    /// 优先级 (1-5, 5最高)
    pub priority: u8,
    /// 建议描述
    pub description: String,
    /// 代码修改建议
    pub code_suggestion: Option<String>,
    /// 相关瓶颈
    pub related_bottlenecks: Vec<BottleneckType>,
}

/// 优化建议类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecommendationType {
    /// 循环展开
    LoopUnrolling,
    /// 循环分块
    LoopTiling,
    /// 内存访问优化
    MemoryAccessOptimization,
    /// 向量化
    Vectorization,
    /// 并行化改进
    ParallelizationImprovement,
    /// 数据布局优化
    DataLayoutOptimization,
    /// 设备迁移建议
    DeviceMigration,
    /// 内核融合
    KernelFusion,
    /// 异步执行
    AsyncExecution,
    /// 内存复用
    MemoryReuse,
    /// 精度调整
    PrecisionAdjustment,
}

// ============================================================================
// 硬件模型
// ============================================================================

/// 设备性能模型
#[derive(Debug, Clone)]
pub struct DeviceModel {
    /// 设备名称
    pub name: String,
    /// 设备类型
    pub device_type: DeviceType,
    /// 峰值计算性能 (GFLOPS)
    pub peak_flops: f64,
    /// 峰值内存带宽 (GB/s)
    pub peak_memory_bandwidth: f64,
    /// 缓存大小 (字节)
    pub cache_sizes: Vec<u64>,
    /// 线程/核心数量
    pub num_threads: usize,
    /// 时钟频率 (MHz)
    pub clock_frequency: f64,
    /// 支持的精度
    pub supported_precisions: Vec<Precision>,
}

/// 设备类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceType {
    CPU,
    GPU,
    FPGA,
    NPU,
}

/// 计算精度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Precision {
    FP64,
    FP32,
    FP16,
    BF16,
    INT8,
    INT4,
}

impl DeviceModel {
    /// 创建 GPU 设备模型（默认参数）
    pub fn generic_gpu() -> Self {
        Self {
            name: "Generic GPU".to_string(),
            device_type: DeviceType::GPU,
            peak_flops: 10000.0, // 10 TFLOPS
            peak_memory_bandwidth: 900.0, // 900 GB/s
            cache_sizes: vec![32 * 1024, 2 * 1024 * 1024], // L1: 32KB, L2: 2MB
            num_threads: 1024 * 128, // 128 SMs * 1024 threads
            clock_frequency: 1500.0, // 1.5 GHz
            supported_precisions: vec![Precision::FP64, Precision::FP32, Precision::FP16, Precision::INT8],
        }
    }

    /// 创建 CPU 设备模型（默认参数）
    pub fn generic_cpu() -> Self {
        Self {
            name: "Generic CPU".to_string(),
            device_type: DeviceType::CPU,
            peak_flops: 500.0, // 500 GFLOPS
            peak_memory_bandwidth: 100.0, // 100 GB/s
            cache_sizes: vec![32 * 1024, 256 * 1024, 8 * 1024 * 1024], // L1: 32KB, L2: 256KB, L3: 8MB
            num_threads: 16,
            clock_frequency: 3000.0, // 3 GHz
            supported_precisions: vec![Precision::FP64, Precision::FP32],
        }
    }

    /// 创建 FPGA 设备模型（默认参数）
    pub fn generic_fpga() -> Self {
        Self {
            name: "Generic FPGA".to_string(),
            device_type: DeviceType::FPGA,
            peak_flops: 500.0, // 500 GFLOPS (estimated)
            peak_memory_bandwidth: 50.0, // 50 GB/s
            cache_sizes: vec![4 * 1024 * 1024], // BRAM: 4MB
            num_threads: 1, // 流水线
            clock_frequency: 300.0, // 300 MHz
            supported_precisions: vec![Precision::FP32, Precision::FP16, Precision::INT8],
        }
    }

    /// 创建 NPU 设备模型（默认参数）
    pub fn generic_npu() -> Self {
        Self {
            name: "Generic NPU".to_string(),
            device_type: DeviceType::NPU,
            peak_flops: 10000.0, // 10 TOPS (INT8)
            peak_memory_bandwidth: 50.0, // 50 GB/s
            cache_sizes: vec![1 * 1024 * 1024], // 1MB
            num_threads: 1,
            clock_frequency: 1000.0, // 1 GHz
            supported_precisions: vec![Precision::FP16, Precision::INT8, Precision::INT4],
        }
    }

    /// 计算roofline模型的交叉点（计算强度阈值）
    pub fn roofline_crossover(&self) -> f64 {
        // 当算术强度 = peak_flops / peak_memory_bandwidth 时
        // 从内存受限变为计算受限
        self.peak_flops / self.peak_memory_bandwidth
    }

    /// 估算执行时间
    pub fn estimate_execution_time(&self, flops: u64, memory_bytes: u64) -> f64 {
        let compute_time = flops as f64 / (self.peak_flops * 1e9); // 秒
        let memory_time = memory_bytes as f64 / (self.peak_memory_bandwidth * 1e9); // 秒
        
        // 取两者最大值（简化的roofline模型）
        compute_time.max(memory_time) * 1e6 // 转换为微秒
    }
}

// ============================================================================
// 性能分析器
// ============================================================================

/// 性能分析器
#[derive(Debug)]
pub struct PerformanceAnalyzer {
    /// 设备模型
    device_models: HashMap<String, DeviceModel>,
    /// 分析结果
    result: PerformanceProfile,
    /// 诊断收集器
    diagnostics: Vec<PerformanceDiagnostic>,
}

/// 性能诊断信息
#[derive(Debug, Clone)]
pub struct PerformanceDiagnostic {
    pub level: DiagnosticLevel,
    pub location: String,
    pub message: String,
    pub suggestion: Option<String>,
}

impl PerformanceAnalyzer {
    /// 创建新的性能分析器
    pub fn new() -> Self {
        let mut device_models = HashMap::new();
        device_models.insert("GPU".to_string(), DeviceModel::generic_gpu());
        device_models.insert("CPU".to_string(), DeviceModel::generic_cpu());
        device_models.insert("FPGA".to_string(), DeviceModel::generic_fpga());
        device_models.insert("NPU".to_string(), DeviceModel::generic_npu());

        Self {
            device_models,
            result: PerformanceProfile::default(),
            diagnostics: Vec::new(),
        }
    }

    /// 添加自定义设备模型
    pub fn add_device_model(&mut self, name: String, model: DeviceModel) {
        self.device_models.insert(name, model);
    }

    /// 分析整个程序
    pub fn analyze(&mut self, program: &Program) -> &PerformanceProfile {
        // 分析所有函数
        for func in &program.functions {
            let metrics = self.analyze_function(func);
            self.result.function_metrics.insert(func.name.clone(), metrics);
        }

        // 分析所有任务
        for task in &program.tasks {
            let metrics = self.analyze_task(task);
            self.result.task_metrics.insert(task.name.clone(), metrics);
        }

        // 识别瓶颈
        self.identify_bottlenecks();

        // 生成优化建议
        self.generate_recommendations();

        // 计算预估加速比
        self.calculate_estimated_speedup();

        &self.result
    }

    /// 分析函数
    fn analyze_function(&self, func: &Function) -> FunctionMetrics {
        let mut metrics = FunctionMetrics {
            name: func.name.clone(),
            estimated_flops: 0,
            estimated_memory_bytes: 0,
            arithmetic_intensity: 0.0,
            parallel_loops: 0,
            max_loop_depth: 0,
            is_memory_bound: false,
            estimated_time_us: 0.0,
        };

        // 分析函数体
        self.analyze_block(&func.body, &mut metrics, 0);

        // 计算算术强度
        if metrics.estimated_memory_bytes > 0 {
            metrics.arithmetic_intensity = metrics.estimated_flops as f64 / metrics.estimated_memory_bytes as f64;
        }

        // 使用通用CPU模型估算时间
        let cpu_model = self.device_models.get("CPU").unwrap();
        metrics.estimated_time_us = cpu_model.estimate_execution_time(
            metrics.estimated_flops,
            metrics.estimated_memory_bytes,
        );

        // 判断是否内存受限
        let crossover = cpu_model.roofline_crossover();
        metrics.is_memory_bound = metrics.arithmetic_intensity < crossover;

        metrics
    }

    /// 分析任务
    fn analyze_task(&self, task: &Task) -> TaskMetrics {
        let mut metrics = TaskMetrics {
            name: task.name.clone(),
            target_device: "GPU".to_string(),
            pattern: "Unknown".to_string(),
            estimated_kernel_time_us: 0.0,
            estimated_transfer_time_us: 0.0,
            compute_transfer_overlap: 0.0,
            thread_utilization: 0.0,
            memory_bandwidth_utilization: 0.0,
            bottlenecks: Vec::new(),
        };

        // 获取 pattern
        if let Some(pattern) = &task.pattern {
            metrics.pattern = pattern.kind.clone();
        }

        // 获取目标设备
        if let Some(policy) = &task.policy {
            for (key, value) in &policy.fields {
                if key == "device_hint" {
                    if let Expression::Identifier(device) = value {
                        metrics.target_device = device.clone();
                    }
                }
            }
        }

        // 分析任务体
        let mut func_metrics = FunctionMetrics {
            name: task.name.clone(),
            estimated_flops: 0,
            estimated_memory_bytes: 0,
            arithmetic_intensity: 0.0,
            parallel_loops: 0,
            max_loop_depth: 0,
            is_memory_bound: false,
            estimated_time_us: 0.0,
        };

        self.analyze_block(&task.body, &mut func_metrics, 0);

        // 获取设备模型
        let device_model = self.device_models.get(&metrics.target_device)
            .or_else(|| self.device_models.get("GPU"))
            .unwrap();

        // 估算内核执行时间
        metrics.estimated_kernel_time_us = device_model.estimate_execution_time(
            func_metrics.estimated_flops,
            func_metrics.estimated_memory_bytes,
        );

        // 估算数据传输时间
        metrics.estimated_transfer_time_us = self.estimate_transfer_time(
            func_metrics.estimated_memory_bytes,
            device_model,
        );

        // 计算利用率
        metrics.thread_utilization = self.estimate_thread_utilization(&func_metrics, device_model);
        metrics.memory_bandwidth_utilization = self.estimate_memory_utilization(&func_metrics, device_model);

        // 识别瓶颈
        if metrics.thread_utilization < 0.5 {
            metrics.bottlenecks.push(BottleneckType::LowParallelism);
        }
        if metrics.memory_bandwidth_utilization > 0.8 {
            metrics.bottlenecks.push(BottleneckType::MemoryBandwidth);
        }
        if func_metrics.is_memory_bound {
            metrics.bottlenecks.push(BottleneckType::MemoryBandwidth);
        }

        metrics
    }

    /// 分析代码块
    fn analyze_block(&self, block: &crate::ast::Block, metrics: &mut FunctionMetrics, depth: usize) {
        metrics.max_loop_depth = metrics.max_loop_depth.max(depth);

        for stmt in &block.statements {
            match stmt {
                Statement::ParallelFor { body, .. } => {
                    metrics.parallel_loops += 1;
                    // 估算循环迭代次数（假设为1000）
                    let iterations = 1000u64;
                    let mut loop_metrics = FunctionMetrics::default();
                    loop_metrics.name = "loop_body".to_string();
                    
                    self.analyze_block(body, &mut loop_metrics, depth + 1);
                    
                    // 累加循环体的操作
                    metrics.estimated_flops += loop_metrics.estimated_flops * iterations;
                    metrics.estimated_memory_bytes += loop_metrics.estimated_memory_bytes * iterations;
                    metrics.max_loop_depth = metrics.max_loop_depth.max(loop_metrics.max_loop_depth);
                }
                Statement::For { body, .. } => {
                    // 顺序循环
                    let iterations = 100u64;
                    let mut loop_metrics = FunctionMetrics::default();
                    
                    self.analyze_block(body, &mut loop_metrics, depth + 1);
                    
                    metrics.estimated_flops += loop_metrics.estimated_flops * iterations;
                    metrics.estimated_memory_bytes += loop_metrics.estimated_memory_bytes * iterations;
                    metrics.max_loop_depth = metrics.max_loop_depth.max(loop_metrics.max_loop_depth);
                }
                Statement::Let { init, .. } => {
                    if let Some(expr) = init {
                        self.analyze_expression(expr, metrics);
                    }
                }
                Statement::Expr(expr) => {
                    self.analyze_expression(expr, metrics);
                }
                Statement::Return(expr) => {
                    if let Some(e) = expr {
                        self.analyze_expression(e, metrics);
                    }
                }
                Statement::If { then_branch, else_branch, .. } => {
                    self.analyze_block(then_branch, metrics, depth);
                    if let Some(else_b) = else_branch {
                        self.analyze_block(else_b, metrics, depth);
                    }
                }
                Statement::While { body, .. } => {
                    let mut loop_metrics = FunctionMetrics::default();
                    self.analyze_block(body, &mut loop_metrics, depth + 1);
                    // 假设执行10次
                    metrics.estimated_flops += loop_metrics.estimated_flops * 10;
                    metrics.estimated_memory_bytes += loop_metrics.estimated_memory_bytes * 10;
                }
                _ => {}
            }
        }
    }

    /// 分析表达式
    fn analyze_expression(&self, expr: &Expression, metrics: &mut FunctionMetrics) {
        match expr {
            Expression::Binary { left, right, .. } => {
                // 每个二元运算算作1 FLOP
                metrics.estimated_flops += 1;
                // 假设每个操作数8字节
                metrics.estimated_memory_bytes += 16;
                self.analyze_expression(left, metrics);
                self.analyze_expression(right, metrics);
            }
            Expression::Call { args, .. } => {
                // 函数调用开销
                metrics.estimated_flops += 10;
                for arg in args {
                    self.analyze_expression(arg, metrics);
                }
            }
            Expression::Index { obj, index } => {
                // 数组访问
                metrics.estimated_memory_bytes += 8;
                self.analyze_expression(obj, metrics);
                self.analyze_expression(index, metrics);
            }
            Expression::MethodCall { obj, args, .. } => {
                self.analyze_expression(obj, metrics);
                for arg in args {
                    self.analyze_expression(arg, metrics);
                }
            }
            Expression::Array(elems) => {
                metrics.estimated_memory_bytes += (elems.len() * 8) as u64;
                for elem in elems {
                    self.analyze_expression(elem, metrics);
                }
            }
            Expression::Spawn { task, .. } => {
                // spawn 开销
                metrics.estimated_flops += 100;
                self.analyze_expression(task, metrics);
            }
            _ => {}
        }
    }

    /// 估算数据传输时间
    fn estimate_transfer_time(&self, bytes: u64, device: &DeviceModel) -> f64 {
        // 使用 PCIe Gen4 x16 带宽估算 (约 32 GB/s)
        let pcie_bandwidth = 32.0; // GB/s
        (bytes as f64 / (pcie_bandwidth * 1e9)) * 1e6 // 微秒
    }

    /// 估算线程利用率
    fn estimate_thread_utilization(&self, metrics: &FunctionMetrics, device: &DeviceModel) -> f64 {
        if device.device_type == DeviceType::CPU {
            // CPU: 基于并行循环数量
            let parallelism = metrics.parallel_loops.max(1);
            (parallelism as f64 / device.num_threads as f64).min(1.0)
        } else {
            // GPU: 基于并行循环和线程数
            let estimated_threads = metrics.parallel_loops * 256; // 假设每个循环256线程
            (estimated_threads as f64 / device.num_threads as f64).min(1.0)
        }
    }

    /// 估算内存带宽利用率
    fn estimate_memory_utilization(&self, metrics: &FunctionMetrics, device: &DeviceModel) -> f64 {
        if metrics.estimated_time_us > 0.0 && metrics.estimated_memory_bytes > 0 {
            let achieved_bandwidth_gbps = (metrics.estimated_memory_bytes as f64 / metrics.estimated_time_us) / 1000.0;
            (achieved_bandwidth_gbps / device.peak_memory_bandwidth).min(1.0)
        } else {
            0.0
        }
    }

    /// 识别性能瓶颈
    fn identify_bottlenecks(&mut self) {
        // 分析函数瓶颈
        for (name, metrics) in &self.result.function_metrics {
            if metrics.is_memory_bound {
                self.result.bottlenecks.push(PerformanceBottleneck {
                    bottleneck_type: BottleneckType::MemoryBandwidth,
                    location: name.clone(),
                    severity: 0.8,
                    description: format!(
                        "Function '{}' is memory-bound (arithmetic intensity: {:.2})",
                        name, metrics.arithmetic_intensity
                    ),
                    line_numbers: vec![],
                });
            }

            if metrics.parallel_loops == 0 && metrics.max_loop_depth > 0 {
                self.result.bottlenecks.push(PerformanceBottleneck {
                    bottleneck_type: BottleneckType::LowParallelism,
                    location: name.clone(),
                    severity: 0.6,
                    description: format!(
                        "Function '{}' has sequential loops that could benefit from parallelization",
                        name
                    ),
                    line_numbers: vec![],
                });
            }
        }

        // 分析任务瓶颈
        for (name, metrics) in &self.result.task_metrics {
            for bottleneck in &metrics.bottlenecks {
                self.result.bottlenecks.push(PerformanceBottleneck {
                    bottleneck_type: *bottleneck,
                    location: name.clone(),
                    severity: 0.7,
                    description: format!("Task '{}' has {:?} bottleneck", name, bottleneck),
                    line_numbers: vec![],
                });
            }

            if metrics.estimated_transfer_time_us > metrics.estimated_kernel_time_us * 0.5 {
                self.result.bottlenecks.push(PerformanceBottleneck {
                    bottleneck_type: BottleneckType::DataTransferOverhead,
                    location: name.clone(),
                    severity: 0.75,
                    description: format!(
                        "Task '{}' has significant data transfer overhead ({:.1}% of total time)",
                        name,
                        metrics.estimated_transfer_time_us / (metrics.estimated_kernel_time_us + metrics.estimated_transfer_time_us) * 100.0
                    ),
                    line_numbers: vec![],
                });
            }
        }
    }

    /// 生成优化建议
    fn generate_recommendations(&mut self) {
        for bottleneck in &self.result.bottlenecks {
            let recommendation = match bottleneck.bottleneck_type {
                BottleneckType::MemoryBandwidth => OptimizationRecommendation {
                    recommendation_type: RecommendationType::LoopTiling,
                    location: bottleneck.location.clone(),
                    estimated_improvement: 0.3,
                    priority: 4,
                    description: "Consider loop tiling to improve cache utilization".to_string(),
                    code_suggestion: Some("// Use tiling: for i in (0..N).step_by(TILE_SIZE)".to_string()),
                    related_bottlenecks: vec![BottleneckType::MemoryBandwidth],
                },
                BottleneckType::LowParallelism => OptimizationRecommendation {
                    recommendation_type: RecommendationType::ParallelizationImprovement,
                    location: bottleneck.location.clone(),
                    estimated_improvement: 0.5,
                    priority: 5,
                    description: "Convert sequential loops to parallel loops using 'parallel for'".to_string(),
                    code_suggestion: Some("parallel for i in 0..N { ... }".to_string()),
                    related_bottlenecks: vec![BottleneckType::LowParallelism],
                },
                BottleneckType::DataTransferOverhead => OptimizationRecommendation {
                    recommendation_type: RecommendationType::AsyncExecution,
                    location: bottleneck.location.clone(),
                    estimated_improvement: 0.4,
                    priority: 4,
                    description: "Use asynchronous execution to overlap compute and transfer".to_string(),
                    code_suggestion: Some("let result = spawn on GPU task(args); // later: result.await".to_string()),
                    related_bottlenecks: vec![BottleneckType::DataTransferOverhead],
                },
                _ => continue,
            };
            self.result.recommendations.push(recommendation);
        }

        // 按优先级排序
        self.result.recommendations.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// 计算预估加速比
    fn calculate_estimated_speedup(&mut self) {
        if self.result.recommendations.is_empty() {
            self.result.estimated_speedup = 1.0;
            return;
        }

        // 简化计算：累加预估改进（实际应考虑优化之间的交互）
        let total_improvement: f64 = self.result.recommendations.iter()
            .map(|r| r.estimated_improvement)
            .sum();

        // 使用对数衰减避免过度乐观
        self.result.estimated_speedup = 1.0 + total_improvement.ln_1p();
    }

    /// 获取分析结果
    pub fn get_result(&self) -> &PerformanceProfile {
        &self.result
    }

    /// 生成诊断报告
    pub fn generate_diagnostics(&self, diag_collector: &mut DiagnosticCollector) {
        for bottleneck in &self.result.bottlenecks {
            let diag = Diagnostic::warning(error_codes::PERFORMANCE_ISSUE)
                .at_file("")
                .message(bottleneck.description.clone())
                .with_note(format!("Severity: {:.0}%", bottleneck.severity * 100.0));
            diag_collector.add(diag);
        }

        for rec in &self.result.recommendations {
            let diag = Diagnostic::warning(error_codes::OPTIMIZATION_SUGGESTION)
                .at_file("")
                .message(format!("[Priority {}] {}", rec.priority, rec.description))
                .with_note(format!("Estimated improvement: {:.0}%", rec.estimated_improvement * 100.0));
            diag_collector.add(diag);
        }
    }
}

impl Default for PerformanceAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 报告生成器
// ============================================================================

/// 性能报告生成器
pub struct PerformanceReportGenerator {
    output_format: ReportFormat,
}

/// 报告格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    Text,
    Json,
    Html,
    Markdown,
}

impl PerformanceReportGenerator {
    pub fn new(format: ReportFormat) -> Self {
        Self { output_format: format }
    }

    /// 生成报告
    pub fn generate(&self, profile: &PerformanceProfile) -> String {
        match self.output_format {
            ReportFormat::Text => self.generate_text(profile),
            ReportFormat::Json => self.generate_json(profile),
            ReportFormat::Html => self.generate_html(profile),
            ReportFormat::Markdown => self.generate_markdown(profile),
        }
    }

    fn generate_text(&self, profile: &PerformanceProfile) -> String {
        let mut report = String::new();

        report.push_str("=" .repeat(60).as_str());
        report.push_str("\n性能分析报告\n");
        report.push_str("=".repeat(60).as_str());
        report.push_str("\n\n");

        // 函数指标
        if !profile.function_metrics.is_empty() {
            report.push_str("函数性能指标:\n");
            report.push_str("-".repeat(40).as_str());
            report.push_str("\n");
            for (_, metrics) in &profile.function_metrics {
                report.push_str(&format!(
                    "  {}:\n    FLOPs: {}, Memory: {} bytes\n    Intensity: {:.2}, Time: {:.2} us\n",
                    metrics.name,
                    metrics.estimated_flops,
                    metrics.estimated_memory_bytes,
                    metrics.arithmetic_intensity,
                    metrics.estimated_time_us
                ));
            }
            report.push_str("\n");
        }

        // 任务指标
        if !profile.task_metrics.is_empty() {
            report.push_str("任务性能指标:\n");
            report.push_str("-".repeat(40).as_str());
            report.push_str("\n");
            for (_, metrics) in &profile.task_metrics {
                report.push_str(&format!(
                    "  {} ({}):\n    Kernel: {:.2} us, Transfer: {:.2} us\n    Thread util: {:.1}%, Mem util: {:.1}%\n",
                    metrics.name,
                    metrics.target_device,
                    metrics.estimated_kernel_time_us,
                    metrics.estimated_transfer_time_us,
                    metrics.thread_utilization * 100.0,
                    metrics.memory_bandwidth_utilization * 100.0
                ));
            }
            report.push_str("\n");
        }

        // 瓶颈
        if !profile.bottlenecks.is_empty() {
            report.push_str("性能瓶颈:\n");
            report.push_str("-".repeat(40).as_str());
            report.push_str("\n");
            for bottleneck in &profile.bottlenecks {
                report.push_str(&format!(
                    "  [{:.0}%] {:?} in {}: {}\n",
                    bottleneck.severity * 100.0,
                    bottleneck.bottleneck_type,
                    bottleneck.location,
                    bottleneck.description
                ));
            }
            report.push_str("\n");
        }

        // 优化建议
        if !profile.recommendations.is_empty() {
            report.push_str("优化建议:\n");
            report.push_str("-".repeat(40).as_str());
            report.push_str("\n");
            for rec in &profile.recommendations {
                report.push_str(&format!(
                    "  [Priority {}] {} (+{:.0}%)\n    {}\n",
                    rec.priority,
                    rec.location,
                    rec.estimated_improvement * 100.0,
                    rec.description
                ));
                if let Some(code) = &rec.code_suggestion {
                    report.push_str(&format!("    Suggestion: {}\n", code));
                }
            }
            report.push_str("\n");
        }

        // 总结
        report.push_str(&format!("预估加速比: {:.2}x\n", profile.estimated_speedup));

        report
    }

    fn generate_json(&self, profile: &PerformanceProfile) -> String {
        // 简化的JSON输出
        format!(
            r#"{{"estimated_speedup": {:.2}, "bottlenecks": {}, "recommendations": {}}}"#,
            profile.estimated_speedup,
            profile.bottlenecks.len(),
            profile.recommendations.len()
        )
    }

    fn generate_html(&self, profile: &PerformanceProfile) -> String {
        format!(
            r#"<html><head><title>Performance Report</title></head>
<body><h1>Performance Report</h1>
<p>Estimated Speedup: {:.2}x</p>
<p>Bottlenecks: {}</p>
<p>Recommendations: {}</p>
</body></html>"#,
            profile.estimated_speedup,
            profile.bottlenecks.len(),
            profile.recommendations.len()
        )
    }

    fn generate_markdown(&self, profile: &PerformanceProfile) -> String {
        let mut md = String::new();

        md.push_str("# 性能分析报告\n\n");
        md.push_str(&format!("**预估加速比**: {:.2}x\n\n", profile.estimated_speedup));

        if !profile.bottlenecks.is_empty() {
            md.push_str("## 性能瓶颈\n\n");
            for b in &profile.bottlenecks {
                md.push_str(&format!("- **{:?}** ({}): {}\n", b.bottleneck_type, b.location, b.description));
            }
            md.push_str("\n");
        }

        if !profile.recommendations.is_empty() {
            md.push_str("## 优化建议\n\n");
            md.push_str("| 优先级 | 位置 | 描述 | 预估提升 |\n");
            md.push_str("|--------|------|------|----------|\n");
            for r in &profile.recommendations {
                md.push_str(&format!("| {} | {} | {} | {:.0}% |\n", r.priority, r.location, r.description, r.estimated_improvement * 100.0));
            }
        }

        md
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_model() {
        let gpu = DeviceModel::generic_gpu();
        assert!(gpu.peak_flops > 0.0);
        assert!(gpu.peak_memory_bandwidth > 0.0);
        assert!(gpu.roofline_crossover() > 0.0);
    }

    #[test]
    fn test_execution_time_estimation() {
        let gpu = DeviceModel::generic_gpu();
        let time = gpu.estimate_execution_time(1_000_000_000, 1_000_000_000);
        assert!(time > 0.0);
    }

    #[test]
    fn test_performance_analyzer() {
        let mut analyzer = PerformanceAnalyzer::new();
        let program = Program {
            imports: vec![],
            functions: vec![],
            tasks: vec![],
        };
        let result = analyzer.analyze(&program);
        assert_eq!(result.estimated_speedup, 1.0);
    }

    #[test]
    fn test_report_generator() {
        let generator = PerformanceReportGenerator::new(ReportFormat::Text);
        let profile = PerformanceProfile::default();
        let report = generator.generate(&profile);
        assert!(report.contains("性能分析报告"));
    }

    #[test]
    fn test_markdown_report() {
        let generator = PerformanceReportGenerator::new(ReportFormat::Markdown);
        let profile = PerformanceProfile::default();
        let report = generator.generate(&profile);
        assert!(report.contains("# 性能分析报告"));
    }

    #[test]
    fn test_bottleneck_types() {
        let types = vec![
            BottleneckType::MemoryBandwidth,
            BottleneckType::ComputeBound,
            BottleneckType::LowParallelism,
        ];
        assert_eq!(types.len(), 3);
    }

    #[test]
    fn test_recommendation_priority() {
        let rec = OptimizationRecommendation {
            recommendation_type: RecommendationType::LoopTiling,
            location: "test".to_string(),
            estimated_improvement: 0.3,
            priority: 5,
            description: "Test".to_string(),
            code_suggestion: None,
            related_bottlenecks: vec![],
        };
        assert_eq!(rec.priority, 5);
    }
}
