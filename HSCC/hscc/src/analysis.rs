//! Pattern/Policy 一致性检查模块
//!
//! 提供针对 HSCLang 的 pattern 和 policy 语义验证。
//!
//! # 概述
//!
//! HSCLang 的执行模型基于 Pattern（执行模式）和 Policy（执行策略）的组合。
//! 本模块负责验证这些声明的正确性和一致性，确保生成的代码能够正确执行。
//!
//! # Pattern 类型
//!
//! | Pattern | 描述 | 适用设备 |
//! |---------|------|----------|
//! | `For` | 顺序/并行循环 | CPU, GPU, FPGA |
//! | `Reduce` | 归约操作 | CPU, GPU |
//! | `Scan` | 前缀扫描 | CPU, GPU |
//! | `TaskGraph` | 任务图 | All |
//! | `Pipeline` | 流水线 | FPGA |
//! | `DataParallel` | 数据并行 | GPU, NPU |
//!
//! # Policy 字段
//!
//! | 字段 | 描述 | 有效值 |
//! |------|------|--------|
//! | `device_hint` | 目标设备 | CPU, GPU, FPGA, NPU |
//! | `granularity` | 执行粒度 | Fine, Medium, Coarse |
//! | `priority` | 执行优先级 | Low, Normal, High, Critical |
//! | `recursive_split` | 递归分治 | true, false |
//!
//! # 主要组件
//!
//! - [`PatternPolicyAnalyzer`] - 主分析器
//! - [`PatternKind`] - 支持的 Pattern 类型
//! - [`PolicyField`] - Policy 字段定义
//! - [`PatternValidation`] - Pattern 验证结果
//! - [`PolicyValidation`] - Policy 验证结果
//!
//! # 使用示例
//!
//! ```rust
//! use hscc::analysis::PatternPolicyAnalyzer;
//! use hscc::diagnostic::DiagnosticCollector;
//! use hscc::ast::Task;
//!
//! let mut analyzer = PatternPolicyAnalyzer::new();
//! let mut collector = DiagnosticCollector::new();
//!
//! analyzer.validate_task(&task, &mut collector);
//!
//! if collector.has_errors() {
//!     collector.emit();
//! }
//! ```
//!
//! # 验证规则
//!
//! ## Pattern 验证
//! - Pattern 类型必须是已知的
//! - Pattern 参数必须符合该类型的约束
//! - `independent` 声明必须与实际依赖一致
//!
//! ## Policy 验证
//! - 设备提示必须是有效设备
//! - 粒度必须与 Pattern 匹配
//! - 优先级设置必须合理
//!
//! ## 一致性验证
//! - Pattern 和 Policy 的组合必须有效
//! - 目标设备必须支持该 Pattern

use crate::ast::*;
use crate::diagnostic::{Diagnostic, DiagnosticCollector, DiagnosticTag, SourceSpan, error_codes};
use std::collections::HashMap;

// ============================================================================
// Pattern 定义
// ============================================================================

/// 支持的 Pattern 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PatternKind {
    /// 顺序执行
    For,
    /// 归约
    Reduce,
    /// 扫描
    Scan,
    /// 任务图
    TaskGraph,
    /// 流水线
    Pipeline,
    /// 数据并行
    DataParallel,
}

impl PatternKind {
    /// 从字符串解析
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "For" => Some(Self::For),
            "Reduce" => Some(Self::Reduce),
            "Scan" => Some(Self::Scan),
            "TaskGraph" => Some(Self::TaskGraph),
            "Pipeline" => Some(Self::Pipeline),
            "DataParallel" => Some(Self::DataParallel),
            _ => None,
        }
    }

    /// 获取支持的字段
    pub fn supported_fields(&self) -> Vec<&'static str> {
        match self {
            Self::For => vec!["independent", "dynamic", "tile_size"],
            Self::Reduce => vec!["op", "initial_value", "dynamic"],
            Self::Scan => vec!["op", "initial_value", "direction"],
            Self::TaskGraph => vec!["dynamic", "lazy"],
            Self::Pipeline => vec!["stages", "buffer_size", "dynamic"],
            Self::DataParallel => vec!["chunks", "dynamic"],
        }
    }

    /// 获取推荐的设备类型
    pub fn recommended_devices(&self) -> Vec<&'static str> {
        match self {
            Self::For => vec!["GPU", "CPU"],
            Self::Reduce => vec!["GPU", "NPU"],
            Self::Scan => vec!["GPU"],
            Self::TaskGraph => vec!["CPU", "GPU"],
            Self::Pipeline => vec!["FPGA", "GPU"],
            Self::DataParallel => vec!["GPU", "NPU"],
        }
    }

    /// 检查是否支持指定设备
    pub fn supports_device(&self, device: &str) -> bool {
        self.recommended_devices().iter().any(|d| *d == device)
    }
}

// ============================================================================
// Policy 定义
// ============================================================================

/// 支持的 Policy 字段
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PolicyField {
    /// 设备提示
    DeviceHint,
    /// 执行粒度
    Granularity,
    /// 优先级
    Priority,
    /// 是否递归拆分
    RecursiveSplit,
    /// 最大并行度
    MaxParallelism,
    /// 内存策略
    MemoryStrategy,
}

impl PolicyField {
    /// 从字符串解析
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "device_hint" => Some(Self::DeviceHint),
            "granularity" => Some(Self::Granularity),
            "priority" => Some(Self::Priority),
            "recursive_split" => Some(Self::RecursiveSplit),
            "max_parallelism" => Some(Self::MaxParallelism),
            "memory_strategy" => Some(Self::MemoryStrategy),
            _ => None,
        }
    }

    /// 获取有效值
    pub fn valid_values(&self) -> Vec<&'static str> {
        match self {
            Self::DeviceHint => vec!["GPU", "CPU", "NPU", "FPGA", "Auto"],
            Self::Granularity => vec!["Fine", "Medium", "Coarse", "Auto"],
            Self::Priority => vec!["Low", "Normal", "High", "Critical"],
            Self::RecursiveSplit => vec!["true", "false"],
            Self::MaxParallelism => vec![], // 整数值，无固定选项
            Self::MemoryStrategy => vec!["Streaming", "InPlace", "DoubleBuffer"],
        }
    }
}

// ============================================================================
// 检查规则
// ============================================================================

/// Pattern/Policy 检查规则
pub struct PatternPolicyChecker {
    /// 设备能力
    device_capabilities: HashMap<String, DeviceCapability>,
    /// 检查严格程度
    strict_mode: bool,
}

/// 设备能力描述
#[derive(Debug, Clone)]
pub struct DeviceCapability {
    /// 设备名称
    pub name: String,
    /// 支持的 Pattern
    pub supported_patterns: Vec<PatternKind>,
    /// 最大并行度
    pub max_parallelism: usize,
    /// 支持的数据类型
    pub supported_types: Vec<String>,
}

impl PatternPolicyChecker {
    /// 创建新的检查器
    pub fn new() -> Self {
        let mut capabilities = HashMap::new();
        
        // GPU 能力
        capabilities.insert("GPU".to_string(), DeviceCapability {
            name: "GPU".to_string(),
            supported_patterns: vec![
                PatternKind::For,
                PatternKind::Reduce,
                PatternKind::Scan,
                PatternKind::DataParallel,
            ],
            max_parallelism: 1000000,
            supported_types: vec!["f32", "f64", "i32", "i64", "f16", "bf16"].iter().map(|s| s.to_string()).collect(),
        });

        // CPU 能力
        capabilities.insert("CPU".to_string(), DeviceCapability {
            name: "CPU".to_string(),
            supported_patterns: vec![
                PatternKind::For,
                PatternKind::TaskGraph,
            ],
            max_parallelism: 128,
            supported_types: vec!["f32", "f64", "i32", "i64"].iter().map(|s| s.to_string()).collect(),
        });

        // NPU 能力
        capabilities.insert("NPU".to_string(), DeviceCapability {
            name: "NPU".to_string(),
            supported_patterns: vec![
                PatternKind::Reduce,
                PatternKind::DataParallel,
            ],
            max_parallelism: 10000,
            supported_types: vec!["f32", "f16", "i8", "bf16"].iter().map(|s| s.to_string()).collect(),
        });

        // FPGA 能力
        capabilities.insert("FPGA".to_string(), DeviceCapability {
            name: "FPGA".to_string(),
            supported_patterns: vec![
                PatternKind::Pipeline,
                PatternKind::For,
            ],
            max_parallelism: 1000,
            supported_types: vec!["f32", "i32", "i8"].iter().map(|s| s.to_string()).collect(),
        });

        Self {
            device_capabilities: capabilities,
            strict_mode: false,
        }
    }

    /// 设置严格模式
    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.strict_mode = strict;
        self
    }

    /// 检查 Pattern
    pub fn check_pattern(&self, pattern: &Pattern, collector: &mut DiagnosticCollector) {
        // 检查 Pattern 类型是否有效
        let kind = match PatternKind::from_str(&pattern.kind) {
            Some(k) => k,
            None => {
                collector.add(
                    Diagnostic::error(error_codes::UNSUPPORTED_PATTERN_FOR_TARGET)
                        .message(format!("Unknown pattern type: {}", pattern.kind))
                        .tag(DiagnosticTag::Correctness)
                );
                return;
            }
        };

        // 检查字段是否有效
        let supported_fields = kind.supported_fields();
        for (field_name, _) in &pattern.fields {
            if !supported_fields.contains(&field_name.as_str()) {
                collector.add(
                    Diagnostic::warning(error_codes::PATTERN_POLICY_MISMATCH)
                        .message(format!(
                            "Field '{}' is not supported by pattern '{}' (supported: {})",
                            field_name,
                            pattern.kind,
                            supported_fields.join(", ")
                        ))
                        .tag(DiagnosticTag::Correctness)
                );
            }
        }

        // 特定 Pattern 的检查
        match kind {
            PatternKind::For => self.check_for_pattern(pattern, collector),
            PatternKind::Reduce => self.check_reduce_pattern(pattern, collector),
            PatternKind::Scan => self.check_scan_pattern(pattern, collector),
            PatternKind::Pipeline => self.check_pipeline_pattern(pattern, collector),
            _ => {}
        }
    }

    /// 检查 For Pattern
    fn check_for_pattern(&self, pattern: &Pattern, collector: &mut DiagnosticCollector) {
        for (field_name, value) in &pattern.fields {
            match field_name.as_str() {
                "independent" => {
                    // independent 必须是布尔值
                    if !matches!(value, Expression::Bool(_)) {
                        collector.add(
                            Diagnostic::warning(error_codes::INVALID_POLICY_VALUE)
                                .message("Field 'independent' should be a boolean value")
                                .tag(DiagnosticTag::Correctness)
                        );
                    }
                }
                "tile_size" => {
                    // tile_size 应该是正整数
                    if let Expression::Integer(n) = value {
                        if *n <= 0 {
                            collector.add(
                                Diagnostic::warning(error_codes::INVALID_POLICY_VALUE)
                                    .message("Field 'tile_size' should be a positive integer")
                                    .tag(DiagnosticTag::Correctness)
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// 检查 Reduce Pattern
    fn check_reduce_pattern(&self, pattern: &Pattern, collector: &mut DiagnosticCollector) {
        let has_op = pattern.fields.iter().any(|(k, _)| k == "op");
        if !has_op && self.strict_mode {
            collector.add(
                Diagnostic::warning(error_codes::PATTERN_POLICY_MISMATCH)
                    .message("Reduce pattern should specify 'op' field")
                    .tag(DiagnosticTag::Style)
            );
        }
    }

    /// 检查 Scan Pattern
    fn check_scan_pattern(&self, pattern: &Pattern, collector: &mut DiagnosticCollector) {
        // 检查 direction 字段
        for (field_name, value) in &pattern.fields {
            if field_name == "direction" {
                let valid = matches!(value, Expression::Identifier(s) if s == "Forward" || s == "Backward");
                if !valid {
                    collector.add(
                        Diagnostic::warning(error_codes::INVALID_POLICY_VALUE)
                            .message("Field 'direction' should be 'Forward' or 'Backward'")
                            .tag(DiagnosticTag::Correctness)
                    );
                }
            }
        }
    }

    /// 检查 Pipeline Pattern
    fn check_pipeline_pattern(&self, pattern: &Pattern, collector: &mut DiagnosticCollector) {
        // 检查 stages 字段
        for (field_name, value) in &pattern.fields {
            if field_name == "stages" {
                if let Expression::Integer(n) = value {
                    if *n <= 0 {
                        collector.add(
                            Diagnostic::warning(error_codes::INVALID_POLICY_VALUE)
                                .message("Field 'stages' should be a positive integer")
                                .tag(DiagnosticTag::Correctness)
                        );
                    }
                }
            }
            if field_name == "buffer_size" {
                if let Expression::Integer(n) = value {
                    if *n <= 0 {
                        collector.add(
                            Diagnostic::warning(error_codes::INVALID_POLICY_VALUE)
                                .message("Field 'buffer_size' should be a positive integer")
                                .tag(DiagnosticTag::Correctness)
                        );
                    }
                }
            }
        }
    }

    /// 检查 Policy
    pub fn check_policy(&self, policy: &Policy, collector: &mut DiagnosticCollector) {
        for (field_name, value) in &policy.fields {
            let field = match PolicyField::from_str(field_name) {
                Some(f) => f,
                None => {
                    collector.add(
                        Diagnostic::warning(error_codes::INVALID_POLICY_VALUE)
                            .message(format!("Unknown policy field: {}", field_name))
                            .tag(DiagnosticTag::Correctness)
                    );
                    continue;
                }
            };

            // 检查字段值是否有效
            self.check_policy_field(field, value, collector);
        }
    }

    /// 检查 Policy 字段值
    fn check_policy_field(&self, field: PolicyField, value: &Expression, collector: &mut DiagnosticCollector) {
        match field {
            PolicyField::DeviceHint => {
                let device = self.extract_string_value(value);
                if !self.device_capabilities.contains_key(&device) && device != "Auto" {
                    collector.add(
                        Diagnostic::warning(error_codes::UNSUPPORTED_TARGET_DEVICE)
                            .message(format!(
                                "Unknown device hint: {} (available: {})",
                                device,
                                self.device_capabilities.keys().cloned().collect::<Vec<_>>().join(", ")
                            ))
                            .tag(DiagnosticTag::Portability)
                    );
                }
            }
            PolicyField::Granularity => {
                let gran = self.extract_string_value(value);
                if !["Fine", "Medium", "Coarse", "Auto"].contains(&gran.as_str()) {
                    collector.add(
                        Diagnostic::warning(error_codes::INVALID_POLICY_VALUE)
                            .message(format!("Invalid granularity: {} (expected: Fine, Medium, Coarse, Auto)", gran))
                            .tag(DiagnosticTag::Correctness)
                    );
                }
            }
            PolicyField::Priority => {
                let priority = self.extract_string_value(value);
                if !["Low", "Normal", "High", "Critical"].contains(&priority.as_str()) {
                    collector.add(
                        Diagnostic::warning(error_codes::INVALID_POLICY_VALUE)
                            .message(format!("Invalid priority: {} (expected: Low, Normal, High, Critical)", priority))
                            .tag(DiagnosticTag::Correctness)
                    );
                }
            }
            PolicyField::RecursiveSplit => {
                if !matches!(value, Expression::Bool(_)) {
                    collector.add(
                        Diagnostic::warning(error_codes::INVALID_POLICY_VALUE)
                            .message("Field 'recursive_split' should be a boolean")
                            .tag(DiagnosticTag::Correctness)
                    );
                }
            }
            PolicyField::MaxParallelism => {
                if let Expression::Integer(n) = value {
                    if *n <= 0 {
                        collector.add(
                            Diagnostic::warning(error_codes::INVALID_POLICY_VALUE)
                                .message("Field 'max_parallelism' should be a positive integer")
                                .tag(DiagnosticTag::Correctness)
                        );
                    }
                }
            }
            PolicyField::MemoryStrategy => {
                let strategy = self.extract_string_value(value);
                if !["Streaming", "InPlace", "DoubleBuffer"].contains(&strategy.as_str()) {
                    collector.add(
                        Diagnostic::warning(error_codes::INVALID_POLICY_VALUE)
                            .message(format!("Invalid memory_strategy: {}", strategy))
                            .tag(DiagnosticTag::Correctness)
                    );
                }
            }
        }
    }

    /// 检查 Pattern 与 Policy 的一致性
    pub fn check_pattern_policy_consistency(
        &self,
        pattern: &Pattern,
        policy: &Policy,
        collector: &mut DiagnosticCollector,
    ) {
        let pattern_kind = match PatternKind::from_str(&pattern.kind) {
            Some(k) => k,
            None => return,
        };

        // 检查 device_hint 与 pattern 的兼容性
        for (field_name, value) in &policy.fields {
            if field_name == "device_hint" {
                let device = self.extract_string_value(value);
                if !pattern_kind.supports_device(&device) {
                    collector.add(
                        Diagnostic::warning(error_codes::PATTERN_POLICY_MISMATCH)
                            .message(format!(
                                "Pattern '{}' is not recommended for device '{}' (recommended: {})",
                                pattern.kind,
                                device,
                                pattern_kind.recommended_devices().join(", ")
                            ))
                            .tag(DiagnosticTag::Performance)
                    );
                }
            }

            // 检查 granularity 与 pattern 的匹配
            if field_name == "granularity" {
                let gran = self.extract_string_value(value);
                self.check_granularity_pattern_match(&pattern_kind, &gran, collector);
            }
        }
    }

    /// 检查粒度与 Pattern 的匹配
    fn check_granularity_pattern_match(
        &self,
        pattern: &PatternKind,
        granularity: &str,
        collector: &mut DiagnosticCollector,
    ) {
        match pattern {
            PatternKind::Reduce => {
                if granularity == "Fine" {
                    collector.add(
                        Diagnostic::warning(error_codes::SUBOPTIMAL_GRANULARITY)
                            .message("Fine granularity may generate too many small tasks for Reduce pattern")
                            .suggest(SourceSpan::unknown(), "Coarse", "Consider using Coarse granularity for better performance")
                            .tag(DiagnosticTag::Performance)
                    );
                }
            }
            PatternKind::Scan => {
                if granularity == "Coarse" {
                    collector.add(
                        Diagnostic::warning(error_codes::SUBOPTIMAL_GRANULARITY)
                            .message("Coarse granularity may limit parallelism for Scan pattern")
                            .suggest(SourceSpan::unknown(), "Medium", "Consider using Medium granularity for better parallelism")
                            .tag(DiagnosticTag::Performance)
                    );
                }
            }
            PatternKind::Pipeline => {
                if granularity == "Fine" {
                    collector.add(
                        Diagnostic::warning(error_codes::SUBOPTIMAL_GRANULARITY)
                            .message("Fine granularity may add overhead for Pipeline pattern")
                            .suggest(SourceSpan::unknown(), "Coarse", "Consider using Coarse granularity to minimize pipeline overhead")
                            .tag(DiagnosticTag::Performance)
                    );
                }
            }
            _ => {}
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
            Expression::Bool(b) => b.to_string(),
            Expression::Integer(n) => n.to_string(),
            Expression::Float(n) => n.to_string(),
            _ => String::new(),
        }
    }
    
    /// 分析整个程序
    pub fn analyze(&mut self, program: &Program, collector: &mut DiagnosticCollector) {
        // 分析所有任务
        for task in &program.tasks {
            if let Some(pattern) = &task.pattern {
                self.check_pattern(pattern, collector);
            }
            if let Some(policy) = &task.policy {
                self.check_policy(policy, collector);
            }
        }
        
        // 分析所有函数
        for func in &program.functions {
            // 函数通常没有 pattern/policy，但可以检查内部的任务调用
            self.analyze_block(&func.body, collector);
        }
    }
    
    /// 分析代码块
    fn analyze_block(&self, block: &Block, collector: &mut DiagnosticCollector) {
        for stmt in &block.statements {
            match stmt {
                Statement::ParallelFor { body, .. } | Statement::For { body, .. } => {
                    self.analyze_block(body, collector);
                }
                Statement::If { then_branch, else_branch, .. } => {
                    self.analyze_block(then_branch, collector);
                    if let Some(else_b) = else_branch {
                        self.analyze_block(else_b, collector);
                    }
                }
                Statement::While { body, .. } | Statement::Loop(body) => {
                    self.analyze_block(body, collector);
                }
                _ => {}
            }
        }
    }
}

impl Default for PatternPolicyChecker {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 任务分析器
// ============================================================================

/// 任务分析结果
#[derive(Debug, Default)]
pub struct TaskAnalysisResult {
    /// Pattern 类型
    pub pattern_kind: Option<PatternKind>,
    /// 设备提示
    pub device_hint: Option<String>,
    /// 粒度
    pub granularity: Option<String>,
    /// 优先级
    pub priority: Option<String>,
    /// 是否有问题
    pub has_issues: bool,
}

/// 分析任务
pub fn analyze_task(task: &Task, collector: &mut DiagnosticCollector) -> TaskAnalysisResult {
    let checker = PatternPolicyChecker::new();
    let mut result = TaskAnalysisResult::default();

    // 检查 Pattern
    if let Some(pattern) = &task.pattern {
        result.pattern_kind = PatternKind::from_str(&pattern.kind);
        checker.check_pattern(pattern, collector);
    }

    // 检查 Policy
    if let Some(policy) = &task.policy {
        for (field_name, value) in &policy.fields {
            match field_name.as_str() {
                "device_hint" => result.device_hint = Some(checker.extract_string_value(value)),
                "granularity" => result.granularity = Some(checker.extract_string_value(value)),
                "priority" => result.priority = Some(checker.extract_string_value(value)),
                _ => {}
            }
        }
        checker.check_policy(policy, collector);

        // 检查一致性
        if let Some(pattern) = &task.pattern {
            checker.check_pattern_policy_consistency(pattern, policy, collector);
        }
    }

    result.has_issues = collector.has_errors() || collector.has_warnings();
    result
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_kind_from_str() {
        assert_eq!(PatternKind::from_str("For"), Some(PatternKind::For));
        assert_eq!(PatternKind::from_str("Reduce"), Some(PatternKind::Reduce));
        assert_eq!(PatternKind::from_str("Unknown"), None);
    }

    #[test]
    fn test_pattern_supported_fields() {
        let for_pattern = PatternKind::For;
        let fields = for_pattern.supported_fields();
        assert!(fields.contains(&"independent"));
        assert!(fields.contains(&"tile_size"));
    }

    #[test]
    fn test_pattern_recommended_devices() {
        let reduce = PatternKind::Reduce;
        let devices = reduce.recommended_devices();
        assert!(devices.contains(&"GPU"));
        assert!(devices.contains(&"NPU"));
    }

    #[test]
    fn test_policy_field_from_str() {
        assert_eq!(PolicyField::from_str("device_hint"), Some(PolicyField::DeviceHint));
        assert_eq!(PolicyField::from_str("granularity"), Some(PolicyField::Granularity));
        assert_eq!(PolicyField::from_str("unknown"), None);
    }

    #[test]
    fn test_policy_field_valid_values() {
        let device_hint = PolicyField::DeviceHint;
        let values = device_hint.valid_values();
        assert!(values.contains(&"GPU"));
        assert!(values.contains(&"CPU"));
        assert!(values.contains(&"Auto"));
    }

    #[test]
    fn test_pattern_policy_checker() {
        let checker = PatternPolicyChecker::new();
        let mut collector = DiagnosticCollector::new();

        let pattern = Pattern {
            kind: "For".to_string(),
            fields: vec![
                ("independent".to_string(), Expression::Bool(true)),
                ("tile_size".to_string(), Expression::Integer(32)),
            ],
        };

        checker.check_pattern(&pattern, &mut collector);
        assert!(!collector.has_errors());
    }

    #[test]
    fn test_invalid_pattern_field() {
        let checker = PatternPolicyChecker::new();
        let mut collector = DiagnosticCollector::new();

        let pattern = Pattern {
            kind: "For".to_string(),
            fields: vec![
                ("invalid_field".to_string(), Expression::Bool(true)),
            ],
        };

        checker.check_pattern(&pattern, &mut collector);
        assert!(collector.has_warnings());
    }

    #[test]
    fn test_device_pattern_compatibility() {
        let checker = PatternPolicyChecker::new();
        let mut collector = DiagnosticCollector::new();

        let pattern = Pattern {
            kind: "Pipeline".to_string(),
            fields: vec![],
        };

        let policy = Policy {
            kind: "default".to_string(),
            fields: vec![
                ("device_hint".to_string(), Expression::Identifier("NPU".to_string())),
            ],
        };

        checker.check_pattern_policy_consistency(&pattern, &policy, &mut collector);
        
        // Pipeline 不推荐在 NPU 上运行
        assert!(collector.has_warnings());
    }
}
