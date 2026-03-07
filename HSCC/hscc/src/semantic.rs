//! 语义分析器模块
//!
//! 提供基于 AST 的语义分析，检测 HSCLang 程序中的语义错误和潜在问题。
//!
//! # 概述
//!
//! 本模块实现了编译器前端的语义分析阶段，在类型检查之后、IR 生成之前运行。
//! 主要负责检测那些需要跨语句、跨函数分析的语义问题。
//!
//! # 主要功能
//!
//! - **任务依赖分析**：构建任务依赖图，检测循环依赖
//! - **循环独立性验证**：验证 `independent` 声明是否与实际依赖冲突
//! - **设备亲和性分析**：检查设备放置和数据传输的一致性
//! - **Pattern/Policy 验证**：验证执行模式和策略的语义正确性
//!
//! # 主要组件
//!
//! - [`SemanticAnalyzer`] - 主分析器，协调整个分析流程
//! - [`TaskDependencyGraph`] - 任务依赖图，用于检测循环依赖
//! - [`SemanticError`] - 语义错误类型枚举
//! - [`SemanticWarning`] - 语义警告类型枚举
//!
//! # 使用示例
//!
//! ```rust
//! use hscc::semantic::SemanticAnalyzer;
//! use hscc::diagnostic::DiagnosticCollector;
//! use hscc::ast::Program;
//!
//! let mut analyzer = SemanticAnalyzer::new();
//! let mut collector = DiagnosticCollector::new();
//!
//! analyzer.set_file("main.hl");
//! analyzer.analyze(&program, &mut collector);
//!
//! if collector.has_errors() {
//!     collector.emit();
//! }
//! ```
//!
//! # 分析流程
//!
//! 1. 构建任务依赖图
//! 2. 检测任务图中的循环依赖
//! 3. 分析每个任务的 Pattern 和 Policy
//! 4. 执行数据流分析
//! 5. 检查设备相关约束
//! 6. 收集并报告诊断信息

use crate::ast::*;
use crate::diagnostic::{Diagnostic, DiagnosticCollector, DiagnosticTag, SourceSpan, error_codes};
use std::collections::{HashMap, HashSet, VecDeque};

// ============================================================================
// 语义错误类型
// ============================================================================

/// 语义分析错误类型
#[derive(Debug, Clone)]
pub enum SemanticError {
    /// 循环独立性声明与实际依赖冲突
    IndependentLoopWithDependency {
        task: String,
        loop_var: String,
        buffer: String,
        dependency_type: DependencyType,
        location: SourceSpan,
    },
    /// 任务图存在循环依赖
    TaskGraphCycle {
        cycle_path: Vec<String>,
    },
    /// 设备放置冲突
    DevicePlacementConflict {
        buffer: String,
        devices: Vec<String>,
        location: SourceSpan,
    },
    /// pattern 与 policy 不一致
    PatternPolicyMismatch {
        pattern: String,
        policy_field: String,
        reason: String,
        location: SourceSpan,
    },
    /// 跨设备数据传输
    CrossDeviceTransfer {
        buffer: String,
        from_device: String,
        to_device: String,
        location: SourceSpan,
    },
    /// 未初始化的 Buffer 使用
    UninitializedBufferUse {
        buffer: String,
        use_location: SourceSpan,
    },
    /// 无效的 Policy 值
    InvalidPolicyValue {
        field: String,
        value: String,
        reason: String,
        location: SourceSpan,
    },
    /// 不支持的目标设备
    UnsupportedTargetDevice {
        device: String,
        available: Vec<String>,
        location: SourceSpan,
    },
}

/// 依赖类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyType {
    /// 读后写 (RAW)
    ReadAfterWrite,
    /// 写后读 (WAR)
    WriteAfterRead,
    /// 写后写 (WAW)
    WriteAfterWrite,
}

impl std::fmt::Display for DependencyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DependencyType::ReadAfterWrite => write!(f, "RAW"),
            DependencyType::WriteAfterRead => write!(f, "WAR"),
            DependencyType::WriteAfterWrite => write!(f, "WAW"),
        }
    }
}

/// 语义警告
#[derive(Debug, Clone)]
pub enum SemanticWarning {
    /// 低效的数据传输
    InefficientDataTransfer {
        buffer: String,
        suggestion: String,
        location: SourceSpan,
    },
    /// 潜在性能问题
    PotentialPerformanceIssue {
        issue: String,
        suggestion: String,
        location: SourceSpan,
    },
    /// 未使用的变量
    UnusedVariable {
        name: String,
        location: SourceSpan,
    },
    /// 冗余的设备传输
    RedundantDeviceTransfer {
        buffer: String,
        device: String,
        location: SourceSpan,
    },
    /// 次优的粒度设置
    SuboptimalGranularity {
        pattern: String,
        granularity: String,
        suggestion: String,
        location: SourceSpan,
    },
}

// ============================================================================
// 数据流信息
// ============================================================================

/// 变量定义信息
#[derive(Debug, Clone)]
struct Definition {
    /// 变量名
    name: String,
    /// 定义位置
    location: SourceSpan,
    /// 是否已初始化
    initialized: bool,
    /// 所在设备
    device: Option<String>,
}

/// 变量使用信息
#[derive(Debug, Clone)]
struct Use {
    /// 变量名
    name: String,
    /// 使用位置
    location: SourceSpan,
    /// 使用类型
    use_type: UseType,
}

/// 使用类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UseType {
    Read,
    Write,
    ReadWrite,
}

/// 数据流信息
#[derive(Debug, Default)]
pub struct DataFlowInfo {
    /// 当前作用域的变量定义
    definitions: HashMap<String, Definition>,
    /// 变量使用列表
    uses: Vec<Use>,
    /// 循环内的 Buffer 访问
    buffer_accesses_in_loop: Vec<BufferAccess>,
}

/// Buffer 访问记录
#[derive(Debug, Clone)]
struct BufferAccess {
    buffer_name: String,
    access_type: UseType,
    loop_var: Option<String>,
    location: SourceSpan,
}

// ============================================================================
// 任务依赖图
// ============================================================================

/// 任务依赖图
#[derive(Debug, Default)]
pub struct TaskDependencyGraph {
    /// 节点：任务名
    nodes: HashSet<String>,
    /// 边：依赖关系 (from, to, buffer)
    edges: Vec<(String, String, String)>,
    /// 任务到 Buffer 的映射
    task_buffers: HashMap<String, HashSet<String>>,
    /// Buffer 所属设备
    buffer_devices: HashMap<String, String>,
}

impl TaskDependencyGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加任务节点
    pub fn add_task(&mut self, name: &str) {
        self.nodes.insert(name.to_string());
    }

    /// 添加依赖边
    pub fn add_dependency(&mut self, from: &str, to: &str, buffer: &str) {
        self.edges.push((from.to_string(), to.to_string(), buffer.to_string()));
    }

    /// 添加任务使用的 Buffer
    pub fn add_task_buffer(&mut self, task: &str, buffer: &str) {
        self.task_buffers
            .entry(task.to_string())
            .or_default()
            .insert(buffer.to_string());
    }

    /// 设置 Buffer 所属设备
    pub fn set_buffer_device(&mut self, buffer: &str, device: &str) {
        self.buffer_devices.insert(buffer.to_string(), device.to_string());
    }

    /// 检测循环依赖（使用 DFS）
    pub fn detect_cycles(&self) -> Option<Vec<String>> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for node in &self.nodes {
            if self.dfs_cycle(node, &mut visited, &mut rec_stack, &mut path) {
                return Some(path);
            }
        }
        None
    }

    fn dfs_cycle(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> bool {
        if rec_stack.contains(node) {
            // 找到循环，截取循环部分
            if let Some(start_idx) = path.iter().position(|n| n == node) {
                *path = path[start_idx..].to_vec();
            }
            path.push(node.to_string());
            return true;
        }
        if visited.contains(node) {
            return false;
        }

        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        path.push(node.to_string());

        for (from, to, _) in &self.edges {
            if from == node {
                if self.dfs_cycle(to, visited, rec_stack, path) {
                    return true;
                }
            }
        }

        rec_stack.remove(node);
        path.pop();
        false
    }

    /// 拓扑排序
    pub fn topological_sort(&self) -> Option<Vec<String>> {
        let mut in_degree: HashMap<String, usize> = self.nodes.iter()
            .map(|n| (n.clone(), 0))
            .collect();

        for (_, to, _) in &self.edges {
            *in_degree.entry(to.clone()).or_insert(0) += 1;
        }

        let mut queue: VecDeque<String> = in_degree.iter()
            .filter(|(_, deg)| **deg == 0)
            .map(|(n, _)| n.clone())
            .collect();

        let mut result = Vec::new();

        while let Some(node) = queue.pop_front() {
            result.push(node.clone());

            for (from, to, _) in &self.edges {
                if from.as_str() == node.as_str() {
                    *in_degree.get_mut(to).unwrap() -= 1;
                    if in_degree[to] == 0 {
                        queue.push_back(to.clone());
                    }
                }
            }
        }

        if result.len() == self.nodes.len() {
            Some(result)
        } else {
            None
        }
    }
}

// ============================================================================
// 设备信息
// ============================================================================

/// 设备能力
#[derive(Debug, Clone)]
pub struct DeviceCapability {
    /// 设备名称
    pub name: String,
    /// 每块最大线程数
    pub max_threads_per_block: usize,
    /// 共享内存大小 (字节)
    pub shared_memory_size: usize,
    /// 是否支持 FP16
    pub supports_fp16: bool,
    /// 是否支持 INT8
    pub supports_int8: bool,
    /// 是否支持 BF16
    pub supports_bf16: bool,
}

impl Default for DeviceCapability {
    fn default() -> Self {
        Self {
            name: "Unknown".to_string(),
            max_threads_per_block: 1024,
            shared_memory_size: 49152,
            supports_fp16: false,
            supports_int8: true,
            supports_bf16: false,
        }
    }
}

/// 设备信息
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// 可用设备列表
    pub available_devices: Vec<String>,
    /// 设备能力映射
    pub capabilities: HashMap<String, DeviceCapability>,
}

impl Default for DeviceInfo {
    fn default() -> Self {
        let mut capabilities = HashMap::new();
        
        // GPU 默认能力
        capabilities.insert("GPU".to_string(), DeviceCapability {
            name: "GPU".to_string(),
            max_threads_per_block: 1024,
            shared_memory_size: 49152,
            supports_fp16: true,
            supports_int8: true,
            supports_bf16: true,
        });

        // CPU 默认能力
        capabilities.insert("CPU".to_string(), DeviceCapability {
            name: "CPU".to_string(),
            max_threads_per_block: 1,
            shared_memory_size: 0,
            supports_fp16: false,
            supports_int8: true,
            supports_bf16: false,
        });

        // Host 默认能力
        capabilities.insert("Host".to_string(), DeviceCapability {
            name: "Host".to_string(),
            max_threads_per_block: 1,
            shared_memory_size: 0,
            supports_fp16: false,
            supports_int8: true,
            supports_bf16: false,
        });

        // NPU 默认能力
        capabilities.insert("NPU".to_string(), DeviceCapability {
            name: "NPU".to_string(),
            max_threads_per_block: 1,
            shared_memory_size: 0,
            supports_fp16: true,
            supports_int8: true,
            supports_bf16: true,
        });

        Self {
            available_devices: vec!["GPU".to_string(), "CPU".to_string(), "Host".to_string(), "NPU".to_string()],
            capabilities,
        }
    }
}

impl DeviceInfo {
    pub fn new() -> Self {
        Self::default()
    }

    /// 检查设备是否可用
    pub fn is_device_available(&self, device: &str) -> bool {
        self.available_devices.contains(&device.to_string())
    }

    /// 获取设备能力
    pub fn get_capability(&self, device: &str) -> Option<&DeviceCapability> {
        self.capabilities.get(device)
    }
}

// ============================================================================
// 语义分析器
// ============================================================================

/// 分析上下文
#[derive(Debug)]
struct AnalysisContext {
    /// 当前文件路径
    file: String,
    /// 当前任务名
    current_task: Option<String>,
    /// 当前函数名
    current_function: Option<String>,
    /// 当前循环变量
    loop_vars: Vec<String>,
    /// 当前设备上下文
    current_device: Option<String>,
}

impl Default for AnalysisContext {
    fn default() -> Self {
        Self {
            file: String::new(),
            current_task: None,
            current_function: None,
            loop_vars: Vec::new(),
            current_device: None,
        }
    }
}

/// 语义分析器
pub struct SemanticAnalyzer {
    /// 设备信息
    device_info: DeviceInfo,
    /// 任务依赖图
    task_graph: TaskDependencyGraph,
    /// 数据流信息
    dataflow: DataFlowInfo,
    /// 分析上下文
    context: AnalysisContext,
    /// 调试级别
    debug_level: usize,
}

impl SemanticAnalyzer {
    /// 创建新的语义分析器
    pub fn new() -> Self {
        Self {
            device_info: DeviceInfo::new(),
            task_graph: TaskDependencyGraph::new(),
            dataflow: DataFlowInfo::default(),
            context: AnalysisContext::default(),
            debug_level: 0,
        }
    }

    /// 设置调试级别
    pub fn with_debug_level(mut self, level: usize) -> Self {
        self.debug_level = level;
        self
    }

    /// 设置文件路径
    pub fn set_file(&mut self, file: impl Into<String>) {
        self.context.file = file.into();
    }

    /// 分析整个程序
    pub fn analyze(&mut self, program: &Program, collector: &mut DiagnosticCollector) {
        if self.debug_level >= 1 {
            println!("[SemanticAnalyzer] Starting analysis...");
        }

        // 1. 收集所有任务
        self.collect_tasks(program);

        // 2. 构建任务依赖图
        self.build_task_graph(program);

        // 3. 检测循环依赖
        if let Some(cycle) = self.task_graph.detect_cycles() {
            collector.add(
                Diagnostic::error(error_codes::TASK_GRAPH_CYCLE)
                    .at_file(&self.context.file)
                    .message("Task graph contains a cycle")
                    .related(&self.context.file, SourceSpan::unknown(), 
                        format!("Cycle: {}", cycle.join(" -> ")))
            );
        }

        // 4. 分析每个任务
        for task in &program.tasks {
            self.analyze_task(task, collector);
        }

        // 5. 分析每个函数
        for func in &program.functions {
            self.analyze_function(func, collector);
        }

        if self.debug_level >= 1 {
            println!("[SemanticAnalyzer] Analysis complete. {} errors, {} warnings",
                collector.error_count(), collector.warning_count());
        }
    }

    /// 收集所有任务
    fn collect_tasks(&mut self, program: &Program) {
        for task in &program.tasks {
            self.task_graph.add_task(&task.name);
        }
    }

    /// 构建任务依赖图
    fn build_task_graph(&mut self, program: &Program) {
        for task in &program.tasks {
            self.context.current_task = Some(task.name.clone());
            self.collect_spawn_dependencies(&task.name, &task.body);
            self.context.current_task = None;
        }
    }

    /// 收集 spawn 依赖
    fn collect_spawn_dependencies(&mut self, current_task: &str, block: &Block) {
        for stmt in &block.statements {
            match stmt {
                Statement::Expr(expr) => {
                    self.collect_spawn_from_expr(current_task, expr);
                }
                Statement::ParallelFor { body, .. } | 
                Statement::For { body, .. } | 
                Statement::While { body, .. } |
                Statement::Loop(body) => {
                    self.collect_spawn_dependencies(current_task, body);
                }
                Statement::If { then_branch, else_branch, .. } => {
                    self.collect_spawn_dependencies(current_task, then_branch);
                    if let Some(else_block) = else_branch {
                        self.collect_spawn_dependencies(current_task, else_block);
                    }
                }
                Statement::Let { init, .. } => {
                    if let Some(expr) = init {
                        self.collect_spawn_from_expr(current_task, expr);
                    }
                }
                _ => {}
            }
        }
    }

    /// 从表达式中收集 spawn 依赖
    fn collect_spawn_from_expr(&mut self, current_task: &str, expr: &Expression) {
        match expr {
            Expression::Spawn { task, .. } => {
                let task_name = self.get_callee_name(task);
                if !task_name.is_empty() && task_name != "unknown" {
                    self.task_graph.add_dependency(current_task, &task_name, "data");
                }
            }
            Expression::Binary { left, right, .. } => {
                self.collect_spawn_from_expr(current_task, left);
                self.collect_spawn_from_expr(current_task, right);
            }
            Expression::Call { func, args } => {
                self.collect_spawn_from_expr(current_task, func);
                for arg in args {
                    self.collect_spawn_from_expr(current_task, arg);
                }
            }
            Expression::MethodCall { obj, args, .. } => {
                self.collect_spawn_from_expr(current_task, obj);
                for arg in args {
                    self.collect_spawn_from_expr(current_task, arg);
                }
            }
            Expression::Index { obj, index } => {
                self.collect_spawn_from_expr(current_task, obj);
                self.collect_spawn_from_expr(current_task, index);
            }
            Expression::FieldAccess { obj, .. } => {
                self.collect_spawn_from_expr(current_task, obj);
            }
            Expression::Await(inner) |
            Expression::MoveTo { expr: inner, .. } |
            Expression::PlaceOn { expr: inner, .. } => {
                self.collect_spawn_from_expr(current_task, inner);
            }
            Expression::Array(elems) => {
                for elem in elems {
                    self.collect_spawn_from_expr(current_task, elem);
                }
            }
            _ => {}
        }
    }

    /// 获取被调用者名称
    fn get_callee_name(&self, expr: &Expression) -> String {
        match expr {
            Expression::Identifier(name) => name.clone(),
            Expression::Path(path) => {
                path.segments.last().map(|s| s.ident.clone()).unwrap_or_default()
            }
            Expression::Call { func, .. } => self.get_callee_name(func),
            Expression::Await(inner) => self.get_callee_name(inner),
            _ => "unknown".to_string(),
        }
    }

    /// 分析任务
    fn analyze_task(&mut self, task: &Task, collector: &mut DiagnosticCollector) {
        self.context.current_task = Some(task.name.clone());
        self.dataflow = DataFlowInfo::default();

        if self.debug_level >= 1 {
            println!("[SemanticAnalyzer] Analyzing task '{}'", task.name);
        }

        // 1. 检查 pattern 语义
        if let Some(pattern) = &task.pattern {
            self.analyze_pattern(pattern, &task.body, collector);
        }

        // 2. 检查 policy 合理性
        if let Some(policy) = &task.policy {
            self.analyze_policy(policy, task.pattern.as_ref(), collector);
        }

        // 3. 分析任务体
        self.analyze_block(&task.body, collector);

        // 4. 检查未使用的变量
        self.check_unused_variables(collector);

        self.context.current_task = None;
    }

    /// 分析函数
    fn analyze_function(&mut self, func: &Function, collector: &mut DiagnosticCollector) {
        self.context.current_function = Some(func.name.clone());
        self.dataflow = DataFlowInfo::default();

        if self.debug_level >= 1 {
            println!("[SemanticAnalyzer] Analyzing function '{}'", func.name);
        }

        // 分析函数体
        self.analyze_block(&func.body, collector);

        // 检查未使用的变量
        self.check_unused_variables(collector);

        self.context.current_function = None;
    }

    /// 分析代码块
    fn analyze_block(&mut self, block: &Block, collector: &mut DiagnosticCollector) {
        for stmt in &block.statements {
            self.analyze_statement(stmt, collector);
        }
    }

    /// 分析语句
    fn analyze_statement(&mut self, stmt: &Statement, collector: &mut DiagnosticCollector) {
        match stmt {
            Statement::Let { name, init, .. } => {
                // 记录变量定义
                self.dataflow.definitions.insert(name.clone(), Definition {
                    name: name.clone(),
                    location: SourceSpan::unknown(),
                    initialized: init.is_some(),
                    device: self.context.current_device.clone(),
                });

                if let Some(expr) = init {
                    self.analyze_expression(expr, collector);
                }
            }

            Statement::Expr(expr) => {
                self.analyze_expression(expr, collector);
            }

            Statement::Return(expr) => {
                if let Some(expr) = expr {
                    self.analyze_expression(expr, collector);
                }
            }

            Statement::ParallelFor { var, range, body } => {
                // 分析范围表达式
                self.analyze_expression(&range.0, collector);
                self.analyze_expression(&range.1, collector);

                // 进入循环作用域
                self.context.loop_vars.push(var.clone());

                // 记录循环内的 Buffer 访问
                self.analyze_loop_body_for_dependencies(body, var, collector);

                // 分析循环体
                self.analyze_block(body, collector);

                self.context.loop_vars.pop();
            }

            Statement::For { var, range, body } => {
                self.analyze_expression(&range.0, collector);
                self.analyze_expression(&range.1, collector);

                self.context.loop_vars.push(var.clone());
                self.analyze_block(body, collector);
                self.context.loop_vars.pop();
            }

            Statement::If { condition, then_branch, else_branch } => {
                self.analyze_expression(condition, collector);
                self.analyze_block(then_branch, collector);
                if let Some(else_block) = else_branch {
                    self.analyze_block(else_block, collector);
                }
            }

            Statement::While { condition, body } => {
                self.analyze_expression(condition, collector);
                self.analyze_block(body, collector);
            }

            Statement::Loop(body) => {
                self.analyze_block(body, collector);
            }

            Statement::Break | Statement::Continue => {}
        }
    }

    /// 分析表达式
    fn analyze_expression(&mut self, expr: &Expression, collector: &mut DiagnosticCollector) {
        match expr {
            Expression::Identifier(name) => {
                // 记录变量使用
                self.dataflow.uses.push(Use {
                    name: name.clone(),
                    location: SourceSpan::unknown(),
                    use_type: UseType::Read,
                });

                // 检查是否未初始化
                if let Some(def) = self.dataflow.definitions.get(name) {
                    if !def.initialized {
                        collector.add(
                            Diagnostic::warning(error_codes::UNINITIALIZED_BUFFER_USE)
                                .at_file(&self.context.file)
                                .message(format!("Variable '{}' may be used before initialization", name))
                                .tag(DiagnosticTag::Correctness)
                        );
                    }
                }
            }

            Expression::Binary { left, right, .. } => {
                self.analyze_expression(left, collector);
                self.analyze_expression(right, collector);
            }

            Expression::Call { func, args } => {
                self.analyze_expression(func, collector);
                for arg in args {
                    self.analyze_expression(arg, collector);
                }
            }

            Expression::MethodCall { obj, method, args } => {
                self.analyze_expression(obj, collector);
                
                // 特殊处理 move_to 和 place_on
                if method == "move_to" {
                    if let Some(device_arg) = args.first() {
                        let target_device = self.extract_device_name(device_arg);
                        if let Some(ref current) = self.context.current_device {
                            if current != &target_device && !target_device.is_empty() {
                                collector.add(
                                    Diagnostic::warning(error_codes::CROSS_DEVICE_TRANSFER)
                                        .at_file(&self.context.file)
                                        .message(format!("Cross-device data transfer: {} -> {}", 
                                            current, target_device))
                                        .tag(DiagnosticTag::Performance)
                                );
                            }
                        }
                    }
                }

                for arg in args {
                    self.analyze_expression(arg, collector);
                }
            }

            Expression::Index { obj, index } => {
                // 记录 Buffer 访问
                if let Expression::Identifier(name) = obj.as_ref() {
                    self.dataflow.buffer_accesses_in_loop.push(BufferAccess {
                        buffer_name: name.clone(),
                        access_type: UseType::ReadWrite,
                        loop_var: self.context.loop_vars.last().cloned(),
                        location: SourceSpan::unknown(),
                    });
                }

                self.analyze_expression(obj, collector);
                self.analyze_expression(index, collector);
            }

            Expression::MoveTo { expr: inner, device } => {
                let target_device = self.extract_device_name(device);
                self.context.current_device = Some(target_device);
                self.analyze_expression(inner, collector);
            }

            Expression::PlaceOn { expr: inner, device } => {
                let target_device = self.extract_device_name(device);
                self.context.current_device = Some(target_device);
                self.analyze_expression(inner, collector);
            }

            Expression::Spawn { device, task, .. } => {
                if let Some(dev) = device {
                    let target_device = self.extract_device_name(dev);
                    
                    // 检查设备是否可用
                    if !target_device.is_empty() && !self.device_info.is_device_available(&target_device) {
                        collector.add(
                            Diagnostic::warning(error_codes::UNSUPPORTED_TARGET_DEVICE)
                                .at_file(&self.context.file)
                                .message(format!("Device '{}' may not be available", target_device))
                                .tag(DiagnosticTag::Portability)
                        );
                    }
                }
                self.analyze_expression(task, collector);
            }

            Expression::Array(elems) => {
                for elem in elems {
                    self.analyze_expression(elem, collector);
                }
            }

            Expression::FieldAccess { obj, .. } => {
                self.analyze_expression(obj, collector);
            }

            Expression::Await(inner) => {
                self.analyze_expression(inner, collector);
            }

            Expression::Path(path) => {
                if path.segments.len() == 1 {
                    let name = &path.segments[0].ident;
                    self.dataflow.uses.push(Use {
                        name: name.clone(),
                        location: SourceSpan::unknown(),
                        use_type: UseType::Read,
                    });
                }
            }

            _ => {}
        }
    }

    /// 分析 Pattern
    fn analyze_pattern(&mut self, pattern: &Pattern, body: &Block, collector: &mut DiagnosticCollector) {
        match pattern.kind.as_str() {
            "For" => {
                // 检查 independent 声明
                for (key, value) in &pattern.fields {
                    if key == "independent" {
                        if let Expression::Bool(true) = value {
                            // 验证循环独立性
                            if !self.verify_loop_independence(body) {
                                collector.add(
                                    Diagnostic::error(error_codes::INDEPENDENT_LOOP_WITH_DEPENDENCY)
                                        .at_file(&self.context.file)
                                        .message(format!(
                                            "Loop in task '{}' declared independent but has cross-iteration dependency",
                                            self.context.current_task.as_deref().unwrap_or("unknown")
                                        ))
                                        .tag(DiagnosticTag::Correctness)
                                        .suggest(
                                            SourceSpan::unknown(),
                                            "independent: false",
                                            "Mark independent as false"
                                        )
                                );
                            }
                        }
                    }
                }
            }
            "Reduce" => {
                // 检查归约操作是否满足交换律和结合律
                self.check_reduce_properties(pattern, body, collector);
            }
            "TaskGraph" => {
                // 任务图依赖已在全局检查
            }
            _ => {}
        }
    }

    /// 分析 Policy
    fn analyze_policy(&mut self, policy: &Policy, pattern: Option<&Pattern>, collector: &mut DiagnosticCollector) {
        for (key, value) in &policy.fields {
            match key.as_str() {
                "device_hint" => {
                    let device = self.extract_device_name(value);
                    if !device.is_empty() && !self.device_info.is_device_available(&device) {
                        collector.add(
                            Diagnostic::warning(error_codes::UNSUPPORTED_TARGET_DEVICE)
                                .at_file(&self.context.file)
                                .message(format!("Device hint '{}' may not be available", device))
                                .tag(DiagnosticTag::Portability)
                        );
                    }
                }
                "granularity" => {
                    if let Some(pat) = pattern {
                        self.check_granularity_match(value, pat, collector);
                    }
                }
                "priority" => {
                    // 检查优先级值是否有效
                    let priority = self.extract_string_value(value);
                    if !matches!(priority.as_str(), "low" | "normal" | "high" | "critical") {
                        collector.add(
                            Diagnostic::warning(error_codes::INVALID_POLICY_VALUE)
                                .at_file(&self.context.file)
                                .message(format!("Invalid priority value: {}", priority))
                                .tag(DiagnosticTag::Correctness)
                        );
                    }
                }
                _ => {}
            }
        }
    }

    /// 验证循环独立性
    fn verify_loop_independence(&self, body: &Block) -> bool {
        // 简化实现：检查是否有跨迭代的 Buffer 读写冲突
        // 完整实现需要数据流分析
        
        // 收集所有 Buffer 访问
        let accesses = &self.dataflow.buffer_accesses_in_loop;
        
        // 检查是否有 RAW/WAR/WAW 依赖
        for (i, access1) in accesses.iter().enumerate() {
            for access2 in accesses.iter().skip(i + 1) {
                if access1.buffer_name == access2.buffer_name {
                    // 同一个 Buffer 的多次访问
                    if access1.loop_var.is_some() && access2.loop_var.is_some() {
                        // 如果两个访问都在循环内，可能存在依赖
                        // 这里简化处理，实际需要更精确的依赖分析
                        match (access1.access_type, access2.access_type) {
                            (UseType::Write, UseType::Read) |
                            (UseType::Read, UseType::Write) |
                            (UseType::Write, UseType::Write) => {
                                return false;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        
        true
    }

    /// 分析循环体以检测依赖
    fn analyze_loop_body_for_dependencies(&mut self, body: &Block, loop_var: &str, _collector: &mut DiagnosticCollector) {
        // 收集循环体内的 Buffer 访问
        for stmt in &body.statements {
            self.collect_buffer_accesses(stmt, loop_var);
        }
    }

    /// 收集 Buffer 访问
    fn collect_buffer_accesses(&mut self, stmt: &Statement, loop_var: &str) {
        match stmt {
            Statement::Let { init, .. } => {
                if let Some(expr) = init {
                    self.collect_buffer_accesses_from_expr(expr, loop_var);
                }
            }
            Statement::Expr(expr) => {
                self.collect_buffer_accesses_from_expr(expr, loop_var);
            }
            Statement::ParallelFor { body, .. } |
            Statement::For { body, .. } => {
                for stmt in &body.statements {
                    self.collect_buffer_accesses(stmt, loop_var);
                }
            }
            _ => {}
        }
    }

    /// 从表达式收集 Buffer 访问
    fn collect_buffer_accesses_from_expr(&mut self, expr: &Expression, loop_var: &str) {
        match expr {
            Expression::Index { obj, index } => {
                if let Expression::Identifier(name) = obj.as_ref() {
                    // 检查索引是否包含循环变量
                    let uses_loop_var = self.expr_uses_var(index, loop_var);
                    
                    self.dataflow.buffer_accesses_in_loop.push(BufferAccess {
                        buffer_name: name.clone(),
                        access_type: UseType::ReadWrite,
                        loop_var: if uses_loop_var { Some(loop_var.to_string()) } else { None },
                        location: SourceSpan::unknown(),
                    });
                }
                self.collect_buffer_accesses_from_expr(index, loop_var);
            }
            Expression::Binary { left, right, .. } => {
                self.collect_buffer_accesses_from_expr(left, loop_var);
                self.collect_buffer_accesses_from_expr(right, loop_var);
            }
            Expression::Call { func, args } => {
                self.collect_buffer_accesses_from_expr(func, loop_var);
                for arg in args {
                    self.collect_buffer_accesses_from_expr(arg, loop_var);
                }
            }
            Expression::MethodCall { obj, args, .. } => {
                self.collect_buffer_accesses_from_expr(obj, loop_var);
                for arg in args {
                    self.collect_buffer_accesses_from_expr(arg, loop_var);
                }
            }
            _ => {}
        }
    }

    /// 检查表达式是否使用变量
    fn expr_uses_var(&self, expr: &Expression, var: &str) -> bool {
        match expr {
            Expression::Identifier(name) => name == var,
            Expression::Binary { left, right, .. } => {
                self.expr_uses_var(left, var) || self.expr_uses_var(right, var)
            }
            Expression::Call { func, args } => {
                self.expr_uses_var(func, var) || args.iter().any(|a| self.expr_uses_var(a, var))
            }
            Expression::Index { obj, index } => {
                self.expr_uses_var(obj, var) || self.expr_uses_var(index, var)
            }
            _ => false,
        }
    }

    /// 检查归约属性
    fn check_reduce_properties(&self, _pattern: &Pattern, _body: &Block, _collector: &mut DiagnosticCollector) {
        // TODO: 实现归约属性检查
    }

    /// 检查粒度匹配
    fn check_granularity_match(&self, granularity: &Expression, pattern: &Pattern, collector: &mut DiagnosticCollector) {
        let gran_str = self.extract_string_value(granularity);
        
        // 根据模式类型检查粒度是否合适
        match pattern.kind.as_str() {
            "Reduce" => {
                if gran_str == "Fine" {
                    collector.add(
                        Diagnostic::warning(error_codes::SUBOPTIMAL_GRANULARITY)
                            .at_file(&self.context.file)
                            .message("Fine granularity may not be optimal for Reduce pattern")
                            .suggest(SourceSpan::unknown(), "Coarse", "Consider using Coarse granularity")
                            .tag(DiagnosticTag::Performance)
                    );
                }
            }
            "For" => {
                // 可以进一步细化检查
            }
            _ => {}
        }
    }

    /// 检查未使用的变量
    fn check_unused_variables(&self, collector: &mut DiagnosticCollector) {
        for (name, _def) in &self.dataflow.definitions {
            let is_used = self.dataflow.uses.iter().any(|u| &u.name == name);
            if !is_used {
                collector.add(
                    Diagnostic::warning(error_codes::UNUSED_VARIABLE)
                        .at_file(&self.context.file)
                        .message(format!("Unused variable: {}", name))
                        .tag(DiagnosticTag::Style)
                );
            }
        }
    }

    /// 从表达式中提取设备名称
    fn extract_device_name(&self, expr: &Expression) -> String {
        match expr {
            Expression::Identifier(name) => name.clone(),
            Expression::Path(path) => {
                path.segments.last().map(|s| s.ident.clone()).unwrap_or_default()
            }
            _ => String::new(),
        }
    }

    /// 从表达式中提取字符串值
    fn extract_string_value(&self, expr: &Expression) -> String {
        match expr {
            Expression::Identifier(name) => name.clone(),
            Expression::String(s) => s.clone(),
            Expression::Path(path) => {
                path.segments.last().map(|s| s.ident.clone()).unwrap_or_default()
            }
            _ => String::new(),
        }
    }
}

impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_dependency_graph_no_cycle() {
        let mut graph = TaskDependencyGraph::new();
        graph.add_task("A");
        graph.add_task("B");
        graph.add_task("C");
        graph.add_dependency("A", "B", "data");
        graph.add_dependency("B", "C", "data");

        assert!(graph.detect_cycles().is_none());
        assert!(graph.topological_sort().is_some());
    }

    #[test]
    fn test_task_dependency_graph_with_cycle() {
        let mut graph = TaskDependencyGraph::new();
        graph.add_task("A");
        graph.add_task("B");
        graph.add_task("C");
        graph.add_dependency("A", "B", "data");
        graph.add_dependency("B", "C", "data");
        graph.add_dependency("C", "A", "data");

        assert!(graph.detect_cycles().is_some());
        assert!(graph.topological_sort().is_none());
    }

    #[test]
    fn test_device_info() {
        let info = DeviceInfo::new();
        
        assert!(info.is_device_available("GPU"));
        assert!(info.is_device_available("CPU"));
        assert!(info.is_device_available("Host"));
        
        let gpu_cap = info.get_capability("GPU").unwrap();
        assert!(gpu_cap.supports_fp16);
        assert!(gpu_cap.supports_int8);
    }

    #[test]
    fn test_semantic_analyzer_simple() {
        let source = r#"
fn main() {
    let x = 42;
}
"#;
        let mut lexer = crate::lexer::Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = crate::parser::Parser::new(tokens);
        let ast = parser.parse_program().expect("Parse failed");

        let mut analyzer = SemanticAnalyzer::new();
        let mut collector = DiagnosticCollector::new();
        analyzer.analyze(&ast, &mut collector);

        // 未使用的变量 x 应该产生警告
        assert_eq!(collector.warning_count(), 1);
        assert!(!collector.has_errors());
    }

    #[test]
    fn test_semantic_analyzer_with_spawn() {
        let source = r#"
task compute {
    body(x: Buffer<f32>) -> Buffer<f32> {
        parallel for i in 0..1024 {
            let y = i;
        }
    }
}

fn main() {
    let a = Buffer::<f32>::zeros([1024]);
    let result = spawn on GPU compute(a).await;
}
"#;
        let mut lexer = crate::lexer::Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = crate::parser::Parser::new(tokens);
        let ast = parser.parse_program().expect("Parse failed");

        let mut analyzer = SemanticAnalyzer::new();
        let mut collector = DiagnosticCollector::new();
        analyzer.analyze(&ast, &mut collector);

        // 不应该有错误
        assert!(!collector.has_errors());
    }
}
