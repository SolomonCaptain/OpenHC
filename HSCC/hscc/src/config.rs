use serde::Deserialize;
use std::fs;
use anyhow::Result;

/// 后端类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// CUDA C++ 后端
    Cuda,
    /// Triton Python 后端
    Triton,
    /// HIP (AMD GPU) 后端
    Hip,
    /// NPU 后端
    Npu,
    /// CPU 后端 (多线程 C++)
    Cpu,
}

impl Default for Backend {
    fn default() -> Self {
        Backend::Cuda
    }
}

impl Backend {
    /// 从字符串解析后端类型
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "cuda" | "gpu" => Backend::Cuda,
            "triton" => Backend::Triton,
            "hip" | "rocm" | "amd" => Backend::Hip,
            "npu" | "intel_npu" | "openvino" => Backend::Npu,
            "cpu" | "host" => Backend::Cpu,
            _ => Backend::Cuda,
        }
    }

    /// 获取后端名称
    pub fn name(&self) -> &str {
        match self {
            Backend::Cuda => "cuda",
            Backend::Triton => "triton",
            Backend::Hip => "hip",
            Backend::Npu => "npu",
            Backend::Cpu => "cpu",
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub package: Package,
    pub target: Target,
    /// 可选的编译后端配置
    #[serde(default)]
    pub backend: Option<BackendConfig>,
    /// 分析配置
    #[serde(default)]
    pub analysis: Option<AnalysisConfig>,
}

#[derive(Debug, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub edition: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Target {
    pub device: String,
    pub arch: Option<String>,
}

/// 后端配置
#[derive(Debug, Deserialize, Default)]
pub struct BackendConfig {
    /// 后端类型: cuda, triton, hip
    #[serde(default)]
    pub kind: Option<String>,
    /// Triton 特定配置
    #[serde(default)]
    pub triton: Option<TritonConfig>,
}

/// Triton 后端配置
#[derive(Debug, Deserialize, Clone)]
pub struct TritonConfig {
    /// 默认块大小
    #[serde(default = "default_block_size")]
    pub block_size: u32,
    /// 使用的 warp 数量
    #[serde(default = "default_num_warps")]
    pub num_warps: u32,
    /// 流水线阶段数
    #[serde(default = "default_num_stages")]
    pub num_stages: u32,
}

fn default_block_size() -> u32 { 1024 }
fn default_num_warps() -> u32 { 4 }
fn default_num_stages() -> u32 { 2 }

impl Default for TritonConfig {
    fn default() -> Self {
        Self {
            block_size: default_block_size(),
            num_warps: default_num_warps(),
            num_stages: default_num_stages(),
        }
    }
}

// ============================================================================
// 分析配置
// ============================================================================

/// 分析配置
#[derive(Debug, Deserialize, Clone)]
pub struct AnalysisConfig {
    /// 是否启用性能分析
    #[serde(default)]
    pub performance: Option<PerformanceAnalysisConfig>,
    /// 是否启用静态分析
    #[serde(default)]
    pub static_analysis: Option<StaticAnalysisConfig>,
    /// 报告输出配置
    #[serde(default)]
    pub report: Option<ReportConfig>,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            performance: Some(PerformanceAnalysisConfig::default()),
            static_analysis: Some(StaticAnalysisConfig::default()),
            report: Some(ReportConfig::default()),
        }
    }
}

/// 性能分析配置
#[derive(Debug, Deserialize, Clone)]
pub struct PerformanceAnalysisConfig {
    /// 是否启用性能分析
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 是否估算执行时间
    #[serde(default = "default_true")]
    pub estimate_time: bool,
    /// 是否识别瓶颈
    #[serde(default = "default_true")]
    pub identify_bottlenecks: bool,
    /// 是否生成优化建议
    #[serde(default = "default_true")]
    pub generate_recommendations: bool,
    /// 瓶颈严重程度阈值 (0.0-1.0)
    #[serde(default = "default_bottleneck_threshold")]
    pub bottleneck_threshold: f64,
    /// 目标设备（用于性能建模）
    #[serde(default)]
    pub target_device: Option<String>,
}

fn default_true() -> bool { true }
fn default_bottleneck_threshold() -> f64 { 0.5 }

impl Default for PerformanceAnalysisConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            estimate_time: true,
            identify_bottlenecks: true,
            generate_recommendations: true,
            bottleneck_threshold: default_bottleneck_threshold(),
            target_device: None,
        }
    }
}

/// 静态分析配置
#[derive(Debug, Deserialize, Clone)]
pub struct StaticAnalysisConfig {
    /// 是否启用静态分析
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 是否检查循环独立性
    #[serde(default = "default_true")]
    pub check_loop_independence: bool,
    /// 是否检查任务图循环
    #[serde(default = "default_true")]
    pub check_task_graph_cycles: bool,
    /// 是否检查设备放置一致性
    #[serde(default = "default_true")]
    pub check_device_placement: bool,
    /// 是否检查 pattern/policy 一致性
    #[serde(default = "default_true")]
    pub check_pattern_policy: bool,
    /// 启用的检查规则
    #[serde(default)]
    pub enabled_rules: Option<Vec<String>>,
    /// 禁用的检查规则
    #[serde(default)]
    pub disabled_rules: Option<Vec<String>>,
}

impl Default for StaticAnalysisConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_loop_independence: true,
            check_task_graph_cycles: true,
            check_device_placement: true,
            check_pattern_policy: true,
            enabled_rules: None,
            disabled_rules: None,
        }
    }
}

/// 报告输出配置
#[derive(Debug, Deserialize, Clone)]
pub struct ReportConfig {
    /// 报告格式: text, json, html, markdown
    #[serde(default = "default_report_format")]
    pub format: String,
    /// 输出目录
    #[serde(default)]
    pub output_dir: Option<String>,
    /// 是否输出到文件
    #[serde(default)]
    pub output_to_file: bool,
    /// 是否在控制台显示
    #[serde(default = "default_true")]
    pub show_in_console: bool,
    /// 是否包含详细信息
    #[serde(default = "default_true")]
    pub verbose: bool,
}

fn default_report_format() -> String { "text".to_string() }

impl Default for ReportConfig {
    fn default() -> Self {
        Self {
            format: default_report_format(),
            output_dir: None,
            output_to_file: false,
            show_in_console: true,
            verbose: true,
        }
    }
}

impl Config {
    pub fn from_file(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
    
    /// 获取编译后端
    pub fn get_backend(&self) -> Backend {
        // 优先使用 backend.kind
        if let Some(ref backend_cfg) = self.backend {
            if let Some(ref kind) = backend_cfg.kind {
                return Backend::from_str(kind);
            }
        }
        
        // 兼容旧的 target.device 字段
        Backend::from_str(&self.target.device)
    }
    
    /// 获取 Triton 配置
    pub fn get_triton_config(&self) -> TritonConfig {
        self.backend
            .as_ref()
            .and_then(|b| b.triton.clone())
            .unwrap_or_default()
    }
    
    /// 检查是否使用 Triton 后端
    pub fn is_triton(&self) -> bool {
        self.get_backend() == Backend::Triton
    }
    
    /// 获取分析配置
    pub fn get_analysis_config(&self) -> AnalysisConfig {
        self.analysis.clone().unwrap_or_default()
    }
    
    /// 获取性能分析配置
    pub fn get_performance_config(&self) -> PerformanceAnalysisConfig {
        self.analysis
            .as_ref()
            .and_then(|a| a.performance.clone())
            .unwrap_or_default()
    }
    
    /// 获取静态分析配置
    pub fn get_static_analysis_config(&self) -> StaticAnalysisConfig {
        self.analysis
            .as_ref()
            .and_then(|a| a.static_analysis.clone())
            .unwrap_or_default()
    }
    
    /// 获取报告配置
    pub fn get_report_config(&self) -> ReportConfig {
        self.analysis
            .as_ref()
            .and_then(|a| a.report.clone())
            .unwrap_or_default()
    }
}

// ============================================================================
// 命令行选项
// ============================================================================

/// 命令行选项
#[derive(Debug, Clone, Default)]
pub struct CliOptions {
    /// 是否启用性能分析
    pub analyze_performance: bool,
    /// 是否启用静态分析
    pub analyze_static: bool,
    /// 是否启用 IR 分析
    pub analyze_ir: bool,
    /// 报告格式
    pub report_format: Option<String>,
    /// 输出目录
    pub output_dir: Option<String>,
    /// 详细模式
    pub verbose: bool,
    /// 仅分析模式（不生成代码）
    pub analyze_only: bool,
    /// IR 分析 Pass 列表
    pub ir_passes: Vec<String>,
}

impl CliOptions {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// 从命令行参数解析
    pub fn parse_args(args: &[String]) -> Self {
        let mut options = Self::new();
        
        for arg in args.iter().skip(1) {
            if arg == "--analyze-performance" || arg == "-p" {
                options.analyze_performance = true;
            } else if arg == "--analyze-static" || arg == "-s" {
                options.analyze_static = true;
            } else if arg == "--analyze-ir" {
                options.analyze_ir = true;
            } else if arg == "--analyze" || arg == "-a" {
                options.analyze_performance = true;
                options.analyze_static = true;
                options.analyze_ir = true;
            } else if arg == "--analyze-only" {
                options.analyze_only = true;
            } else if arg == "--verbose" || arg == "-v" {
                options.verbose = true;
            } else if arg.starts_with("--report-format=") {
                options.report_format = Some(
                    arg.strip_prefix("--report-format=").unwrap().to_string()
                );
            } else if arg.starts_with("--output-dir=") {
                options.output_dir = Some(
                    arg.strip_prefix("--output-dir=").unwrap().to_string()
                );
            } else if arg.starts_with("--ir-passes=") {
                options.ir_passes = arg
                    .strip_prefix("--ir-passes=").unwrap()
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();
            }
        }
        
        options
    }
    
    /// 合并配置文件和命令行选项
    pub fn merge_with_config(&mut self, config: &Config) {
        let analysis_cfg = config.get_analysis_config();
        
        // 如果配置文件启用了分析但命令行未指定，则使用配置文件的设置
        if let Some(ref perf) = analysis_cfg.performance {
            if perf.enabled && !self.analyze_performance {
                self.analyze_performance = true;
            }
        }
        
        if let Some(ref static_cfg) = analysis_cfg.static_analysis {
            if static_cfg.enabled && !self.analyze_static {
                self.analyze_static = true;
            }
        }
        
        if let Some(ref report_cfg) = analysis_cfg.report {
            if self.report_format.is_none() {
                self.report_format = Some(report_cfg.format.clone());
            }
            if self.output_dir.is_none() {
                self.output_dir = report_cfg.output_dir.clone();
            }
        }
    }
}