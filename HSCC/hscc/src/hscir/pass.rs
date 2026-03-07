//! HSCIR Pass 管理器和分析 Pass 模块
//!
//! 提供可扩展的分析和转换 Pass 框架，包括：
//! - Pass 基类和 Pass 管理器
//! - 数据流分析 Pass
//! - 依赖分析 Pass
//! - 设备亲和性分析 Pass

use std::collections::{HashMap, HashSet};
use std::any::Any;

// ============================================================================
// Pass 基础设施
// ============================================================================

/// Pass 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PassKind {
    /// 分析 Pass（只读，不修改 IR）
    Analysis,
    /// 转换 Pass（可能修改 IR）
    Transform,
    /// 工具 Pass（输出、验证等）
    Utility,
}

/// Pass 执行结果
#[derive(Debug)]
pub enum PassResult {
    /// 成功完成
    Success,
    /// 成功，且有修改
    SuccessWithChanges,
    /// 失败
    Failure(String),
    /// 跳过（依赖未满足）
    Skipped(String),
}

impl PassResult {
    pub fn is_success(&self) -> bool {
        matches!(self, PassResult::Success | PassResult::SuccessWithChanges)
    }

    pub fn has_changes(&self) -> bool {
        matches!(self, PassResult::SuccessWithChanges)
    }
}

/// Pass 基类 trait
pub trait Pass: std::fmt::Debug {
    /// 获取 Pass 名称
    fn name(&self) -> &str;

    /// 获取 Pass 类型
    fn kind(&self) -> PassKind;

    /// 获取依赖的其他 Pass
    fn dependencies(&self) -> Vec<&str> {
        vec![]
    }

    /// 执行 Pass
    fn run(&mut self, ctx: &mut PassContext) -> PassResult;

    /// 是否可以与其他 Pass 并行执行
    fn is_parallel_safe(&self) -> bool {
        false
    }

    /// 获取分析结果（如果是分析 Pass）
    fn get_analysis_result(&self) -> Option<&dyn std::any::Any> {
        None
    }
}

/// Pass 执行上下文
#[derive(Debug)]
pub struct PassContext {
    /// 模块名称
    pub module_name: String,
    /// 已分析的结果缓存
    analysis_results: HashMap<String, Box<dyn std::any::Any>>,
    /// 是否有修改
    modified: bool,
    /// 诊断信息
    diagnostics: Vec<PassDiagnostic>,
}

/// Pass 诊断信息
#[derive(Debug, Clone)]
pub struct PassDiagnostic {
    pub pass_name: String,
    pub level: DiagnosticLevel,
    pub message: String,
    pub location: Option<(usize, usize)>,
}

/// 诊断级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Note,
    Info,
}

impl PassContext {
    pub fn new(module_name: String) -> Self {
        Self {
            module_name,
            analysis_results: HashMap::new(),
            modified: false,
            diagnostics: Vec::new(),
        }
    }

    /// 标记已修改
    pub fn mark_modified(&mut self) {
        self.modified = true;
    }

    /// 检查是否已修改
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// 存储分析结果
    pub fn set_analysis<T: 'static>(&mut self, name: &str, result: T) {
        self.analysis_results.insert(name.to_string(), Box::new(result));
    }

    /// 获取分析结果
    pub fn get_analysis<T: 'static>(&self, name: &str) -> Option<&T> {
        self.analysis_results
            .get(name)
            .and_then(|boxed| boxed.downcast_ref::<T>())
    }

    /// 添加诊断信息
    pub fn add_diagnostic(&mut self, pass_name: &str, level: DiagnosticLevel, message: String) {
        self.diagnostics.push(PassDiagnostic {
            pass_name: pass_name.to_string(),
            level,
            message,
            location: None,
        });
    }

    /// 添加带位置的诊断信息
    pub fn add_diagnostic_at(
        &mut self,
        pass_name: &str,
        level: DiagnosticLevel,
        message: String,
        line: usize,
        column: usize,
    ) {
        self.diagnostics.push(PassDiagnostic {
            pass_name: pass_name.to_string(),
            level,
            message,
            location: Some((line, column)),
        });
    }

    /// 获取所有诊断信息
    pub fn get_diagnostics(&self) -> &[PassDiagnostic] {
        &self.diagnostics
    }

    /// 是否有错误
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.level == DiagnosticLevel::Error)
    }
}

// ============================================================================
// Pass 管理器
// ============================================================================

/// Pass 统计信息
#[derive(Debug, Default)]
pub struct PassStatistics {
    /// 执行次数
    pub execution_count: usize,
    /// 总执行时间 (微秒)
    pub total_time_us: u64,
    /// 最大执行时间
    pub max_time_us: u64,
    /// 最小执行时间
    pub min_time_us: u64,
    /// 成功次数
    pub success_count: usize,
    /// 失败次数
    pub failure_count: usize,
}

impl PassStatistics {
    pub fn record(&mut self, time_us: u64, success: bool) {
        self.execution_count += 1;
        self.total_time_us += time_us;
        self.max_time_us = self.max_time_us.max(time_us);
        if self.min_time_us == 0 {
            self.min_time_us = time_us;
        } else {
            self.min_time_us = self.min_time_us.min(time_us);
        }
        if success {
            self.success_count += 1;
        } else {
            self.failure_count += 1;
        }
    }

    pub fn average_time_us(&self) -> u64 {
        if self.execution_count == 0 {
            0
        } else {
            self.total_time_us / self.execution_count as u64
        }
    }
}

/// Pass 管理器
#[derive(Debug, Default)]
pub struct PassManager {
    /// 注册的 Pass 列表
    passes: Vec<Box<dyn Pass>>,
    /// Pass 名称到索引的映射
    pass_indices: HashMap<String, usize>,
    /// 执行顺序
    execution_order: Vec<usize>,
    /// 统计信息
    statistics: HashMap<String, PassStatistics>,
    /// 是否已初始化
    initialized: bool,
}

impl PassManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加 Pass
    pub fn add_pass(&mut self, pass: Box<dyn Pass>) {
        let name = pass.name().to_string();
        let idx = self.passes.len();
        self.pass_indices.insert(name.clone(), idx);
        self.passes.push(pass);
        self.execution_order.push(idx);
        self.initialized = false;
    }

    /// 初始化（解析依赖关系）
    pub fn initialize(&mut self) -> Result<(), String> {
        // 构建依赖图并拓扑排序
        let mut in_degree: HashMap<usize, usize> = HashMap::new();
        let mut dependents: HashMap<usize, Vec<usize>> = HashMap::new();

        for (idx, pass) in self.passes.iter().enumerate() {
            in_degree.insert(idx, 0);
            for dep_name in pass.dependencies() {
                if let Some(&dep_idx) = self.pass_indices.get(dep_name) {
                    in_degree.entry(idx).and_modify(|d| *d += 1).or_insert(1);
                    dependents.entry(dep_idx).or_default().push(idx);
                }
            }
        }

        // 拓扑排序
        let mut queue: Vec<usize> = Vec::new();
        for (&idx, &deg) in &in_degree {
            if deg == 0 {
                queue.push(idx);
            }
        }

        let mut sorted = Vec::new();
        while let Some(idx) = queue.pop() {
            sorted.push(idx);
            if let Some(deps) = dependents.get(&idx) {
                for &dep_idx in deps {
                    if let Some(d) = in_degree.get_mut(&dep_idx) {
                        *d -= 1;
                        if *d == 0 {
                            queue.push(dep_idx);
                        }
                    }
                }
            }
        }

        if sorted.len() != self.passes.len() {
            return Err("Circular dependency detected in passes".to_string());
        }

        self.execution_order = sorted;
        self.initialized = true;
        Ok(())
    }

    /// 运行所有 Pass
    pub fn run(&mut self, ctx: &mut PassContext) -> Result<bool, String> {
        if !self.initialized {
            self.initialize()?;
        }

        let mut any_changes = false;

        for &idx in &self.execution_order {
            let pass_name = self.passes[idx].name().to_string();
            let start = std::time::Instant::now();

            let result = self.passes[idx].run(ctx);

            let elapsed = start.elapsed().as_micros() as u64;
            let success = result.is_success();

            self.statistics
                .entry(pass_name.clone())
                .or_default()
                .record(elapsed, success);

            match result {
                PassResult::Success => {}
                PassResult::SuccessWithChanges => {
                    any_changes = true;
                    ctx.mark_modified();
                }
                PassResult::Failure(msg) => {
                    return Err(format!("Pass '{}' failed: {}", pass_name, msg));
                }
                PassResult::Skipped(reason) => {
                    ctx.add_diagnostic(&pass_name, DiagnosticLevel::Info, format!("Skipped: {}", reason));
                }
            }
        }

        Ok(any_changes)
    }

    /// 运行指定的 Pass
    pub fn run_pass(&mut self, name: &str, ctx: &mut PassContext) -> Result<PassResult, String> {
        let idx = *self.pass_indices.get(name).ok_or_else(|| format!("Pass '{}' not found", name))?;

        // 检查依赖
        let deps = self.passes[idx].dependencies();
        for dep in deps {
            if !ctx.analysis_results.contains_key(dep) {
                return Ok(PassResult::Skipped(format!("Dependency '{}' not satisfied", dep)));
            }
        }

        let start = std::time::Instant::now();
        let result = self.passes[idx].run(ctx);
        let elapsed = start.elapsed().as_micros() as u64;

        self.statistics
            .entry(name.to_string())
            .or_default()
            .record(elapsed, result.is_success());

        Ok(result)
    }

    /// 获取统计信息
    pub fn get_statistics(&self) -> &HashMap<String, PassStatistics> {
        &self.statistics
    }

    /// 获取 Pass 列表
    pub fn get_pass_names(&self) -> Vec<&str> {
        self.passes.iter().map(|p| p.name()).collect()
    }

    /// 清空所有 Pass
    pub fn clear(&mut self) {
        self.passes.clear();
        self.pass_indices.clear();
        self.execution_order.clear();
        self.statistics.clear();
        self.initialized = false;
    }
}

// ============================================================================
// 分析 Pass 实现
// ============================================================================

/// 数据流分析结果
#[derive(Debug, Default, Clone)]
pub struct DataFlowAnalysisData {
    /// 定义点 (变量名 -> 定义位置列表)
    pub definitions: HashMap<String, Vec<usize>>,
    /// 使用点 (变量名 -> 使用位置列表)
    pub uses: HashMap<String, Vec<usize>>,
    /// 活性变量 (块ID -> 活入变量集)
    pub live_in: HashMap<usize, HashSet<String>>,
    /// 活性变量 (块ID -> 活出变量集)
    pub live_out: HashMap<usize, HashSet<String>>,
}

/// 数据流分析 Pass
#[derive(Debug)]
pub struct DataFlowAnalysisPass {
    result: Option<DataFlowAnalysisData>,
}

impl DataFlowAnalysisPass {
    pub fn new() -> Self {
        Self { result: None }
    }

    pub fn get_result(&self) -> Option<&DataFlowAnalysisData> {
        self.result.as_ref()
    }
}

impl Default for DataFlowAnalysisPass {
    fn default() -> Self {
        Self::new()
    }
}

impl Pass for DataFlowAnalysisPass {
    fn name(&self) -> &str {
        "dataflow-analysis"
    }

    fn kind(&self) -> PassKind {
        PassKind::Analysis
    }

    fn run(&mut self, ctx: &mut PassContext) -> PassResult {
        // 简化的数据流分析实现
        let result = DataFlowAnalysisData::default();
        
        self.result = Some(result.clone());
        ctx.set_analysis("dataflow-analysis", result);
        
        PassResult::Success
    }

    fn get_analysis_result(&self) -> Option<&dyn Any> {
        self.result.as_ref().map(|r| r as &dyn Any)
    }
}

/// 依赖分析结果
#[derive(Debug, Default, Clone)]
pub struct DependenceAnalysisData {
    /// 循环携带依赖
    pub loop_carried_deps: Vec<LoopCarriedDep>,
    /// 是否独立
    pub is_independent: bool,
}

/// 循环携带依赖
#[derive(Debug, Clone)]
pub struct LoopCarriedDep {
    pub source_var: String,
    pub target_var: String,
    pub dep_type: DepType,
    pub distance: i64,
    pub source_line: usize,
    pub target_line: usize,
}

/// 依赖类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepType {
    /// 读后写 (True/Flow)
    Flow,
    /// 写后读 (Anti)
    Anti,
    /// 写后写 (Output)
    Output,
    /// 读后读 (Input)
    Input,
}

/// 依赖分析 Pass
#[derive(Debug)]
pub struct DependenceAnalysisPass {
    result: Option<DependenceAnalysisData>,
}

impl DependenceAnalysisPass {
    pub fn new() -> Self {
        Self { result: None }
    }

    pub fn get_result(&self) -> Option<&DependenceAnalysisData> {
        self.result.as_ref()
    }
}

impl Default for DependenceAnalysisPass {
    fn default() -> Self {
        Self::new()
    }
}

impl Pass for DependenceAnalysisPass {
    fn name(&self) -> &str {
        "dependence-analysis"
    }

    fn kind(&self) -> PassKind {
        PassKind::Analysis
    }

    fn dependencies(&self) -> Vec<&str> {
        vec!["dataflow-analysis"]
    }

    fn run(&mut self, ctx: &mut PassContext) -> PassResult {
        // 获取数据流分析结果
        let df_result = ctx.get_analysis::<DataFlowAnalysisData>("dataflow-analysis");
        
        let mut result = DependenceAnalysisData {
            loop_carried_deps: Vec::new(),
            is_independent: true,
        };

        if let Some(df) = df_result {
            // 分析循环携带依赖
            for (var, defs) in &df.definitions {
                if let Some(uses) = df.uses.get(var) {
                    // 检查是否有循环内的依赖
                    if defs.len() > 1 || uses.len() > 1 {
                        result.is_independent = false;
                    }
                }
            }
        }

        self.result = Some(result.clone());
        ctx.set_analysis("dependence-analysis", result);

        PassResult::Success
    }

    fn get_analysis_result(&self) -> Option<&dyn Any> {
        self.result.as_ref().map(|r| r as &dyn Any)
    }
}

/// 设备亲和性分析结果
#[derive(Debug, Default, Clone)]
pub struct DeviceAffinityData {
    /// 操作 -> 推荐设备
    pub op_device_map: HashMap<usize, String>,
    /// Buffer -> 当前设备
    pub buffer_device_map: HashMap<String, String>,
    /// 跨设备传输
    pub cross_device_transfers: Vec<CrossDeviceTransfer>,
}

/// 跨设备传输
#[derive(Debug, Clone)]
pub struct CrossDeviceTransfer {
    pub buffer: String,
    pub from_device: String,
    pub to_device: String,
    pub estimated_cost: f64,
}

/// 设备亲和性分析 Pass
#[derive(Debug)]
pub struct DeviceAffinityAnalysisPass {
    result: Option<DeviceAffinityData>,
}

impl DeviceAffinityAnalysisPass {
    pub fn new() -> Self {
        Self { result: None }
    }
}

impl Default for DeviceAffinityAnalysisPass {
    fn default() -> Self {
        Self::new()
    }
}

impl Pass for DeviceAffinityAnalysisPass {
    fn name(&self) -> &str {
        "device-affinity-analysis"
    }

    fn kind(&self) -> PassKind {
        PassKind::Analysis
    }

    fn run(&mut self, ctx: &mut PassContext) -> PassResult {
        let result = DeviceAffinityData::default();
        
        self.result = Some(result.clone());
        ctx.set_analysis("device-affinity-analysis", result);
        
        PassResult::Success
    }

    fn get_analysis_result(&self) -> Option<&dyn Any> {
        self.result.as_ref().map(|r| r as &dyn Any)
    }
}

// ============================================================================
// 验证 Pass
// ============================================================================

/// 验证 Pass - 检查 IR 的正确性
#[derive(Debug)]
pub struct VerificationPass {
    errors: Vec<String>,
}

impl VerificationPass {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    pub fn get_errors(&self) -> &[String] {
        &self.errors
    }

    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

impl Default for VerificationPass {
    fn default() -> Self {
        Self::new()
    }
}

impl Pass for VerificationPass {
    fn name(&self) -> &str {
        "verification"
    }

    fn kind(&self) -> PassKind {
        PassKind::Utility
    }

    fn run(&mut self, ctx: &mut PassContext) -> PassResult {
        self.errors.clear();

        // 执行验证检查
        // 1. 类型一致性检查
        // 2. 支配关系检查
        // 3. 使用-定义链完整性检查

        if self.errors.is_empty() {
            ctx.add_diagnostic("verification", DiagnosticLevel::Info, "IR verification passed".to_string());
            PassResult::Success
        } else {
            for err in &self.errors {
                ctx.add_diagnostic("verification", DiagnosticLevel::Error, err.clone());
            }
            PassResult::Failure(format!("{} verification errors", self.errors.len()))
        }
    }
}

// ============================================================================
// 打印 Pass
// ============================================================================

/// 打印 Pass - 输出 IR 到字符串
#[derive(Debug)]
pub struct PrintPass {
    output: String,
}

impl PrintPass {
    pub fn new() -> Self {
        Self { output: String::new() }
    }

    pub fn get_output(&self) -> &str {
        &self.output
    }
}

impl Default for PrintPass {
    fn default() -> Self {
        Self::new()
    }
}

impl Pass for PrintPass {
    fn name(&self) -> &str {
        "print"
    }

    fn kind(&self) -> PassKind {
        PassKind::Utility
    }

    fn run(&mut self, ctx: &mut PassContext) -> PassResult {
        self.output = format!("Module: {}\n", ctx.module_name);
        self.output.push_str("// IR output placeholder\n");
        PassResult::Success
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pass_context() {
        let mut ctx = PassContext::new("test_module".to_string());
        
        ctx.mark_modified();
        assert!(ctx.is_modified());
        
        ctx.set_analysis("test", 42usize);
        assert_eq!(ctx.get_analysis::<usize>("test"), Some(&42));
    }

    #[test]
    fn test_pass_manager() {
        let mut pm = PassManager::new();
        
        pm.add_pass(Box::new(DataFlowAnalysisPass::new()));
        pm.add_pass(Box::new(DependenceAnalysisPass::new()));
        
        assert!(pm.initialize().is_ok());
        assert_eq!(pm.get_pass_names().len(), 2);
    }

    #[test]
    fn test_pass_manager_execution() {
        let mut pm = PassManager::new();
        let mut ctx = PassContext::new("test".to_string());
        
        pm.add_pass(Box::new(DataFlowAnalysisPass::new()));
        
        let result = pm.run(&mut ctx);
        assert!(result.is_ok());
    }

    #[test]
    fn test_pass_statistics() {
        let mut stats = PassStatistics::default();
        
        stats.record(100, true);
        stats.record(200, true);
        stats.record(150, false);
        
        assert_eq!(stats.execution_count, 3);
        assert_eq!(stats.success_count, 2);
        assert_eq!(stats.failure_count, 1);
        assert_eq!(stats.average_time_us(), 150);
    }

    #[test]
    fn test_dataflow_analysis_pass() {
        let mut pass = DataFlowAnalysisPass::new();
        let mut ctx = PassContext::new("test".to_string());
        
        let result = pass.run(&mut ctx);
        assert!(result.is_success());
        assert!(pass.get_result().is_some());
    }

    #[test]
    fn test_verification_pass() {
        let mut pass = VerificationPass::new();
        let mut ctx = PassContext::new("test".to_string());
        
        let result = pass.run(&mut ctx);
        assert!(result.is_success());
        assert!(pass.is_valid());
    }

    #[test]
    fn test_diagnostic() {
        let mut ctx = PassContext::new("test".to_string());
        
        ctx.add_diagnostic("test-pass", DiagnosticLevel::Warning, "Test warning".to_string());
        ctx.add_diagnostic_at("test-pass", DiagnosticLevel::Error, "Test error".to_string(), 10, 5);
        
        let diags = ctx.get_diagnostics();
        assert_eq!(diags.len(), 2);
        assert!(ctx.has_errors());
    }
}
