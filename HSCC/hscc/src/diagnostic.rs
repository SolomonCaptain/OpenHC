//! 诊断系统模块
//!
//! 提供统一的错误、警告、提示信息收集与报告机制。
//!
//! # 概述
//!
//! 本模块实现了一个结构化的诊断系统，用于收集、组织和报告编译过程中的各种诊断信息。
//! 支持多种诊断级别（错误、警告、提示、帮助）和详细的源代码位置信息。
//!
//! # 主要组件
//!
//! - [`Diagnostic`] - 单条诊断信息，包含错误码、级别、位置、消息和修复建议
//! - [`DiagnosticCollector`] - 诊断收集器，用于累积和管理诊断信息
//! - [`DiagnosticLevel`] - 诊断级别枚举
//! - [`SourceSpan`] - 源代码位置范围
//! - [`error_codes`] - 预定义的错误码常量
//!
//! # 使用示例
//!
//! ```rust
//! use hscc::diagnostic::{Diagnostic, DiagnosticCollector, error_codes};
//!
//! let mut collector = DiagnosticCollector::new();
//!
//! // 创建一个错误诊断
//! let diag = Diagnostic::error(error_codes::UNDEFINED_VARIABLE)
//!     .at_file("main.hl")
//!     .at_point("main.hl", 10, 5)
//!     .message("Undefined variable 'x'")
//!     .suggest(SourceSpan::single(10, 5), "y", "Did you mean 'y'?");
//!
//! collector.add(diag);
//!
//! // 输出诊断
//! collector.emit();
//! ```
//!
//! # 错误码规范
//!
//! 错误码格式为 `HSCXXXX`，其中：
//! - `HSC0xxx` - 语法错误
//! - `HSC1xxx` - 类型错误
//! - `HSC2xxx` - 语义错误
//! - `HSC3xxx` - 性能警告
//! - `HSC4xxx` - 目标特定错误
//! - `HSC5xxx` - 性能分析与优化

use std::fmt;

/// 诊断级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLevel {
    /// 错误 - 阻止编译继续
    Error,
    /// 警告 - 潜在问题
    Warning,
    /// 提示 - 附加信息
    Note,
    /// 帮助 - 修复建议
    Help,
}

impl fmt::Display for DiagnosticLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiagnosticLevel::Error => write!(f, "error"),
            DiagnosticLevel::Warning => write!(f, "warning"),
            DiagnosticLevel::Note => write!(f, "note"),
            DiagnosticLevel::Help => write!(f, "help"),
        }
    }
}

/// 源代码位置
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpan {
    /// 起始行 (1-based)
    pub start_line: usize,
    /// 起始列 (1-based)
    pub start_column: usize,
    /// 结束行 (1-based)
    pub end_line: usize,
    /// 结束列 (1-based)
    pub end_column: usize,
}

impl SourceSpan {
    pub fn new(start_line: usize, start_column: usize, end_line: usize, end_column: usize) -> Self {
        Self {
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }

    pub fn single(line: usize, column: usize) -> Self {
        Self {
            start_line: line,
            start_column: column,
            end_line: line,
            end_column: column + 1,
        }
    }

    pub fn unknown() -> Self {
        Self {
            start_line: 0,
            start_column: 0,
            end_line: 0,
            end_column: 0,
        }
    }
}

/// 修复建议
#[derive(Debug, Clone)]
pub struct FixSuggestion {
    /// 替换范围
    pub span: SourceSpan,
    /// 替换文本
    pub replacement: String,
    /// 说明
    pub description: String,
}

impl FixSuggestion {
    pub fn new(span: SourceSpan, replacement: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            span,
            replacement: replacement.into(),
            description: description.into(),
        }
    }
}

/// 相关信息条目
#[derive(Debug, Clone)]
pub struct RelatedInfo {
    /// 文件路径
    pub file: String,
    /// 位置
    pub span: SourceSpan,
    /// 消息
    pub message: String,
}

impl RelatedInfo {
    pub fn new(file: impl Into<String>, span: SourceSpan, message: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            span,
            message: message.into(),
        }
    }
}

/// 诊断信息
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// 错误码（如 HSC1001）
    pub code: String,
    /// 诊断级别
    pub level: DiagnosticLevel,
    /// 文件路径
    pub file: String,
    /// 源代码位置
    pub span: SourceSpan,
    /// 主要消息
    pub message: String,
    /// 相关信息（附加说明）
    pub related: Vec<RelatedInfo>,
    /// 建议修复
    pub suggestions: Vec<FixSuggestion>,
    /// 标签（用于分类）
    pub tags: Vec<DiagnosticTag>,
}

/// 诊断标签
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticTag {
    /// 不必要的代码
    Unnecessary,
    /// 已弃用的代码
    Deprecated,
    /// 性能问题
    Performance,
    /// 安全问题
    Security,
    /// 正确性问题
    Correctness,
    /// 可移植性问题
    Portability,
    /// 风格问题
    Style,
}

impl Diagnostic {
    /// 创建一个新的诊断
    pub fn new(level: DiagnosticLevel, code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            level,
            file: String::new(),
            span: SourceSpan::unknown(),
            message: String::new(),
            related: Vec::new(),
            suggestions: Vec::new(),
            tags: Vec::new(),
        }
    }

    /// 创建错误级别诊断
    pub fn error(code: impl Into<String>) -> Self {
        Self::new(DiagnosticLevel::Error, code)
    }

    /// 创建警告级别诊断
    pub fn warning(code: impl Into<String>) -> Self {
        Self::new(DiagnosticLevel::Warning, code)
    }

    /// 创建提示级别诊断
    pub fn note(code: impl Into<String>) -> Self {
        Self::new(DiagnosticLevel::Note, code)
    }

    /// 创建帮助级别诊断
    pub fn help(code: impl Into<String>) -> Self {
        Self::new(DiagnosticLevel::Help, code)
    }

    /// 设置文件路径
    pub fn at_file(mut self, file: impl Into<String>) -> Self {
        self.file = file.into();
        self
    }

    /// 设置位置
    pub fn at(mut self, file: impl Into<String>, span: SourceSpan) -> Self {
        self.file = file.into();
        self.span = span;
        self
    }

    /// 设置位置（单点）
    pub fn at_point(mut self, file: impl Into<String>, line: usize, column: usize) -> Self {
        self.file = file.into();
        self.span = SourceSpan::single(line, column);
        self
    }

    /// 设置消息
    pub fn message(mut self, msg: impl Into<String>) -> Self {
        self.message = msg.into();
        self
    }

    /// 添加相关信息
    pub fn related(mut self, file: impl Into<String>, span: SourceSpan, msg: impl Into<String>) -> Self {
        self.related.push(RelatedInfo::new(file, span, msg));
        self
    }

    /// 添加修复建议
    pub fn suggest(mut self, span: SourceSpan, replacement: impl Into<String>, desc: impl Into<String>) -> Self {
        self.suggestions.push(FixSuggestion::new(span, replacement, desc));
        self
    }

    /// 添加标签
    pub fn tag(mut self, tag: DiagnosticTag) -> Self {
        self.tags.push(tag);
        self
    }

    /// 添加备注信息
    pub fn with_note(mut self, msg: impl Into<String>) -> Self {
        self.related.push(RelatedInfo::new(
            self.file.clone(),
            self.span,
            msg,
        ));
        self
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 格式: file:line:column: level[code]: message
        if !self.file.is_empty() {
            write!(f, "{}:", self.file)?;
            if self.span.start_line > 0 {
                write!(f, "{}:{}", self.span.start_line, self.span.start_column)?;
            }
            write!(f, ": ")?;
        }

        write!(f, "{}[{}]: {}", self.level, self.code, self.message)?;

        // 添加相关信息
        for info in &self.related {
            writeln!(f)?;
            write!(f, "  --> {}:{}:{}: {}", 
                info.file, info.span.start_line, info.span.start_column, info.message)?;
        }

        // 添加修复建议
        for suggestion in &self.suggestions {
            writeln!(f)?;
            write!(f, "  help: {}", suggestion.description)?;
            if !suggestion.replacement.is_empty() {
                write!(f, "\n        replace with: `{}`", suggestion.replacement)?;
            }
        }

        Ok(())
    }
}

/// 诊断收集器
#[derive(Debug, Default)]
pub struct DiagnosticCollector {
    diagnostics: Vec<Diagnostic>,
}

impl DiagnosticCollector {
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加诊断
    pub fn add(&mut self, diag: Diagnostic) {
        self.diagnostics.push(diag);
    }

    /// 添加错误
    pub fn error(&mut self, code: impl Into<String>, message: impl Into<String>) {
        self.add(Diagnostic::error(code).message(message));
    }

    /// 添加警告
    pub fn warning(&mut self, code: impl Into<String>, message: impl Into<String>) {
        self.add(Diagnostic::warning(code).message(message));
    }

    /// 是否有错误
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.level == DiagnosticLevel::Error)
    }

    /// 是否有警告
    pub fn has_warnings(&self) -> bool {
        self.diagnostics.iter().any(|d| d.level == DiagnosticLevel::Warning)
    }

    /// 获取所有诊断
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// 获取错误数量
    pub fn error_count(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.level == DiagnosticLevel::Error).count()
    }

    /// 获取警告数量
    pub fn warning_count(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.level == DiagnosticLevel::Warning).count()
    }

    /// 消费并获取所有诊断
    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }

    /// 输出所有诊断到 stderr
    pub fn emit(&self) {
        for diag in &self.diagnostics {
            eprintln!("{}", diag);
        }
    }

    /// 输出摘要
    pub fn emit_summary(&self) {
        let errors = self.error_count();
        let warnings = self.warning_count();
        
        if errors > 0 || warnings > 0 {
            eprintln!();
            eprintln!("Compilation {} with {} error(s) and {} warning(s)", 
                if errors > 0 { "failed" } else { "finished" },
                errors, warnings);
        }
    }

    /// 清空诊断
    pub fn clear(&mut self) {
        self.diagnostics.clear();
    }

    /// 按文件分组诊断
    pub fn group_by_file(&self) -> std::collections::HashMap<String, Vec<&Diagnostic>> {
        let mut groups = std::collections::HashMap::new();
        for diag in &self.diagnostics {
            groups.entry(diag.file.clone())
                .or_insert_with(Vec::new)
                .push(diag);
        }
        groups
    }
}

// ============================================================================
// 预定义错误码
// ============================================================================

/// 错误码定义
pub mod error_codes {
    // === 语法错误 (HSC0xxx) ===
    pub const SYNTAX_ERROR: &str = "HSC0001";
    pub const UNEXPECTED_TOKEN: &str = "HSC0002";
    pub const UNCLOSED_STRING: &str = "HSC0003";
    pub const UNCLOSED_COMMENT: &str = "HSC0004";

    // === 类型错误 (HSC1xxx) ===
    pub const TYPE_MISMATCH: &str = "HSC1001";
    pub const UNDEFINED_VARIABLE: &str = "HSC1002";
    pub const UNDEFINED_FUNCTION: &str = "HSC1003";
    pub const UNDEFINED_TYPE: &str = "HSC1004";
    pub const BUFFER_DIMENSION_MISMATCH: &str = "HSC1005";
    pub const BUFFER_ELEMENT_TYPE_MISMATCH: &str = "HSC1006";
    pub const INVALID_BINARY_OPERATION: &str = "HSC1007";
    pub const RETURN_TYPE_ERROR: &str = "HSC1008";
    pub const CONDITION_NOT_BOOL: &str = "HSC1009";
    pub const ASSIGNMENT_TYPE_MISMATCH: &str = "HSC1010";
    pub const FUNCTION_ARG_COUNT_MISMATCH: &str = "HSC1011";
    pub const FUNCTION_ARG_TYPE_MISMATCH: &str = "HSC1012";

    // === 语义错误 (HSC2xxx) ===
    pub const INDEPENDENT_LOOP_WITH_DEPENDENCY: &str = "HSC2001";
    pub const TASK_GRAPH_CYCLE: &str = "HSC2002";
    pub const DEVICE_PLACEMENT_CONFLICT: &str = "HSC2003";
    pub const PATTERN_POLICY_MISMATCH: &str = "HSC2004";
    pub const CROSS_DEVICE_TRANSFER: &str = "HSC2005";
    pub const UNINITIALIZED_BUFFER_USE: &str = "HSC2006";
    pub const UNSUPPORTED_PATTERN_FOR_TARGET: &str = "HSC2007";
    pub const INVALID_POLICY_VALUE: &str = "HSC2008";
    pub const UNSUPPORTED_TARGET_DEVICE: &str = "HSC2009";

    // === 性能警告 (HSC3xxx) ===
    pub const INEFFICIENT_DATA_TRANSFER: &str = "HSC3001";
    pub const POTENTIAL_PERFORMANCE_ISSUE: &str = "HSC3002";
    pub const UNUSED_VARIABLE: &str = "HSC3003";
    pub const DEAD_CODE: &str = "HSC3004";
    pub const REDUNDANT_DEVICE_TRANSFER: &str = "HSC3005";
    pub const SUBOPTIMAL_GRANULARITY: &str = "HSC3006";

    // === 目标特定错误 (HSC4xxx) ===
    // GPU
    pub const GPU_THREAD_LIMIT_EXCEEDED: &str = "HSC4001";
    pub const GPU_SHARED_MEMORY_EXCEEDED: &str = "HSC4002";
    pub const GPU_SYNC_IN_CONDITIONAL: &str = "HSC4003";
    pub const GPU_UNCOALESCED_ACCESS: &str = "HSC4004";

    // FPGA
    pub const FPGA_PIPELINE_DEPENDENCY: &str = "HSC4101";
    pub const FPGA_RESOURCE_EXCEEDED: &str = "HSC4102";
    pub const FPGA_UNSUPPORTED_OPERATION: &str = "HSC4103";

    // NPU
    pub const NPU_UNSUPPORTED_OPERATOR: &str = "HSC4201";
    pub const NPU_DATA_LAYOUT_MISMATCH: &str = "HSC4202";
    pub const NPU_PRECISION_NOT_SUPPORTED: &str = "HSC4203";

    // === 性能分析与优化 (HSC5xxx) ===
    pub const PERFORMANCE_ISSUE: &str = "HSC5001";
    pub const OPTIMIZATION_SUGGESTION: &str = "HSC5002";
    pub const MEMORY_BOUND_KERNEL: &str = "HSC5003";
    pub const LOW_PARALLELISM: &str = "HSC5004";
    pub const HIGH_TRANSFER_OVERHEAD: &str = "HSC5005";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_creation() {
        let diag = Diagnostic::error(error_codes::TYPE_MISMATCH)
            .at_file("test.hl")
            .message("Type mismatch: expected i32, found f32");

        assert_eq!(diag.level, DiagnosticLevel::Error);
        assert_eq!(diag.code, "HSC1001");
        assert_eq!(diag.file, "test.hl");
        assert_eq!(diag.message, "Type mismatch: expected i32, found f32");
    }

    #[test]
    fn test_diagnostic_display() {
        let diag = Diagnostic::error(error_codes::UNDEFINED_VARIABLE)
            .at_point("test.hl", 10, 5)
            .message("Undefined variable: x")
            .suggest(
                SourceSpan::single(10, 5),
                "y",
                "Did you mean 'y'?"
            );

        let output = format!("{}", diag);
        assert!(output.contains("test.hl:10:5"));
        assert!(output.contains("error[HSC1002]"));
        assert!(output.contains("Undefined variable: x"));
        assert!(output.contains("Did you mean 'y'?"));
    }

    #[test]
    fn test_diagnostic_collector() {
        let mut collector = DiagnosticCollector::new();

        collector.add(Diagnostic::error("HSC0001").message("Test error"));
        collector.add(Diagnostic::warning("HSC3001").message("Test warning"));

        assert!(collector.has_errors());
        assert!(collector.has_warnings());
        assert_eq!(collector.error_count(), 1);
        assert_eq!(collector.warning_count(), 1);
        assert_eq!(collector.diagnostics().len(), 2);
    }

    #[test]
    fn test_diagnostic_tags() {
        let diag = Diagnostic::warning(error_codes::INEFFICIENT_DATA_TRANSFER)
            .message("Inefficient data transfer")
            .tag(DiagnosticTag::Performance);

        assert!(diag.tags.contains(&DiagnosticTag::Performance));
    }
}
