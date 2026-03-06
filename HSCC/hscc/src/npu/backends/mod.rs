//! NPU 后端抽象
//!
//! 定义 `NpuBackend` trait 和相关的设备抽象，
//! 支持多种 NPU 设备的统一接口。

pub mod intel_npu;

// TODO: 其他 NPU 后端实现
// mod tpu;
// mod ascend;
// mod cambrian;
// mod generic;

pub use intel_npu::{IntelNpuBackend, IntelNpuGeneration, IntelNpuDevice};

use std::time::Duration;
use std::collections::HashMap;
use super::types::{NpuType, NpuTypeKind, TensorLayout};
use super::graph::{NpuGraph, NpuOperation, NpuOpType};

/// NPU 设备类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpuDevice {
    /// Intel NPU (Meteor Lake, Lunar Lake, Arrow Lake)
    IntelNPU(IntelNpuGeneration),
    /// Google TPU
    TPU(TpuGeneration),
    /// 华为昇腾
    Ascend(AscendSoc),
    /// 寒武纪
    Cambrian(CambrianGeneration),
    /// 地平线 BPU
    Horizon(HorizonGeneration),
    /// 通用 NPU（通过 ONNX/OpenVINO 支持更多）
    Generic,
}

/// TPU 代次
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TpuGeneration {
    V2,    // Cloud TPU v2
    V3,    // Cloud TPU v3
    V4,    // Cloud TPU v4
    V5,    // Cloud TPU v5
    Edge,  // Edge TPU
}

/// 昇腾 SoC
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AscendSoc {
    Ascend310,   // 推理
    Ascend310P,  // 推理增强
    Ascend910,   // 训练
    Ascend910B,  // 训练增强
}

/// 寒武纪代次
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CambrianGeneration {
    MLU100,
    MLU270,
    MLU290,
    MLU370,
}

/// 地平线代次
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HorizonGeneration {
    BPU0,
    BPU1,
    BPU2,
    BPU3,
}

/// NPU 硬件规格
#[derive(Debug, Clone)]
pub struct NpuHardwareSpec {
    /// 设备类型
    pub device: NpuDevice,
    /// 设备名称
    pub device_name: String,
    /// 计算核心数量
    pub num_cores: u32,
    /// 矩阵单元规格
    pub matrix_unit: MatrixUnitSpec,
    /// 向量单元规格
    pub vector_unit: VectorUnitSpec,
    /// 片上 SRAM 大小 (KB)
    pub local_memory_kb: u32,
    /// HBM 大小 (GB)
    pub hbm_size_gb: u32,
    /// HBM 带宽 (GB/s)
    pub memory_bandwidth: f64,
    /// 支持的数据类型
    pub supported_dtypes: Vec<NpuTypeKind>,
    /// 量化支持
    pub quant_support: QuantSupport,
    /// 稀疏计算支持
    pub sparse_support: bool,
    /// 首选内存布局
    pub preferred_layout: TensorLayout,
}

/// 矩阵单元规格
#[derive(Debug, Clone)]
pub struct MatrixUnitSpec {
    /// 单次矩阵乘形状 (M, N, K)
    pub systolic_array: (u32, u32, u32),
    /// 支持的数据类型组合
    pub supported_combinations: Vec<(NpuTypeKind, NpuTypeKind, NpuTypeKind)>,
    /// 峰值算力 (TOPS)
    pub peak_tops: f64,
}

/// 向量单元规格
#[derive(Debug, Clone)]
pub struct VectorUnitSpec {
    /// 向量宽度
    pub width: u32,
    /// 支持的操作
    pub supported_ops: Vec<VectorOp>,
}

/// 量化支持级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantSupport {
    None,
    Int8,
    Int4Int8,
    FullDynamic,
}

/// 向量操作
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorOp {
    Add, Sub, Mul, Div,
    Exp, Log, Sqrt,
    ReLU, Sigmoid, Tanh,
    Max, Min,
}

/// NPU 后端抽象 trait
///
/// 所有 NPU 后端必须实现此 trait，提供：
/// - 设备信息查询
/// - 图优化
/// - 内存规划
/// - 代码生成
pub trait NpuBackend: Send + Sync {
    /// 后端名称
    fn name(&self) -> &str;

    /// 支持的设备列表
    fn supported_devices(&self) -> Vec<NpuDevice>;

    /// 获取硬件规格
    fn hardware_spec(&self, device: NpuDevice) -> NpuHardwareSpec;

    /// 检查操作是否支持
    fn is_op_supported(&self, op: &NpuOpType, spec: &NpuHardwareSpec) -> bool;

    /// 获取操作的性能估计
    fn estimate_op_latency(
        &self,
        op: &NpuOperation,
        spec: &NpuHardwareSpec,
    ) -> Duration;

    /// 优化计算图
    fn optimize_graph(
        &self,
        graph: &mut NpuGraph,
        spec: &NpuHardwareSpec,
    ) -> Result<(), NpuError>;

    /// 内存规划
    fn plan_memory(
        &self,
        graph: &mut NpuGraph,
        spec: &NpuHardwareSpec,
    ) -> Result<super::memory::MemoryPlan, NpuError>;

    /// 生成设备代码
    fn generate_code(
        &self,
        graph: &NpuGraph,
        spec: &NpuHardwareSpec,
    ) -> Result<NpuCode, NpuError>;

    /// 生成运行时配置
    fn generate_runtime_config(
        &self,
        graph: &NpuGraph,
        spec: &NpuHardwareSpec,
    ) -> Result<RuntimeConfig, NpuError>;
}

/// 生成的 NPU 代码
#[derive(Debug)]
pub enum NpuCode {
    /// ONNX 模型
    OnnxModel(Vec<u8>),
    /// ONNX 文本格式（用于调试）
    OnnxText(String),
    /// TensorFlow Lite 模型
    TFLiteModel(Vec<u8>),
    /// 厂商格式模型
    VendorModel {
        format: String,
        data: Vec<u8>,
    },
    /// OpenVINO IR 格式
    OpenVINOIR {
        xml: String,
        bin: Vec<u8>,
    },
    /// C++ 运行时代码
    CppCode {
        header: String,
        source: String,
    },
    /// Python 运行时代码
    PythonCode(String),
    /// JSON 图定义
    JsonGraph(String),
}

/// 运行时配置
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// 输入张量描述
    pub inputs: Vec<TensorDesc>,
    /// 输出张量描述
    pub outputs: Vec<TensorDesc>,
    /// 执行配置
    pub execution: ExecutionConfig,
    /// 内存池配置
    pub memory_pool: MemoryPoolConfig,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            inputs: Vec::new(),
            outputs: Vec::new(),
            execution: ExecutionConfig::default(),
            memory_pool: MemoryPoolConfig::default(),
        }
    }
}

/// 张量描述
#[derive(Debug, Clone)]
pub struct TensorDesc {
    /// 名称
    pub name: String,
    /// 数据类型
    pub dtype: NpuType,
    /// 形状
    pub shape: Vec<i64>,
    /// 内存布局
    pub layout: TensorLayout,
}

/// 执行配置
#[derive(Debug, Clone)]
pub struct ExecutionConfig {
    /// 性能提示
    pub performance_hint: PerformanceHint,
    /// 并发请求数
    pub num_requests: u32,
    /// 是否启用 Turbo 模式
    pub turbo: bool,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            performance_hint: PerformanceHint::default(),
            num_requests: 1,
            turbo: false,
        }
    }
}

/// 性能提示
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerformanceHint {
    /// 低延迟优先
    Latency,
    /// 高吞吐优先
    Throughput,
    /// 低功耗优先
    PowerEfficient,
}

impl Default for PerformanceHint {
    fn default() -> Self {
        PerformanceHint::Latency
    }
}

/// 内存池配置
#[derive(Debug, Clone)]
pub struct MemoryPoolConfig {
    /// 内存池大小 (MB)
    pub pool_size_mb: u32,
    /// 是否启用内存复用
    pub enable_reuse: bool,
    /// 是否延迟加载权重
    pub defer_weights_load: bool,
}

impl Default for MemoryPoolConfig {
    fn default() -> Self {
        Self {
            pool_size_mb: 1024,
            enable_reuse: true,
            defer_weights_load: false,
        }
    }
}

/// NPU 错误类型
#[derive(Debug, Clone)]
pub enum NpuError {
    /// 不支持的操作
    UnsupportedOp {
        op: String,
        reason: String,
    },
    /// 不支持的数据类型
    UnsupportedDataType {
        dtype: String,
        reason: String,
    },
    /// 动态形状不支持
    UnsupportedDynamicShape {
        tensor: String,
        reason: String,
    },
    /// 内存规划失败
    MemoryPlanningFailed {
        reason: String,
    },
    /// 代码生成失败
    CodeGenerationFailed {
        reason: String,
    },
    /// 图验证失败
    GraphValidationFailed {
        reason: String,
    },
    /// 配置错误
    ConfigError {
        field: String,
        reason: String,
    },
    /// IO 错误
    IoError {
        operation: String,
        reason: String,
    },
}

impl std::fmt::Display for NpuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NpuError::UnsupportedOp { op, reason } => {
                write!(f, "Unsupported operation '{}': {}", op, reason)
            }
            NpuError::UnsupportedDataType { dtype, reason } => {
                write!(f, "Unsupported data type '{}': {}", dtype, reason)
            }
            NpuError::UnsupportedDynamicShape { tensor, reason } => {
                write!(f, "Dynamic shape not supported for tensor '{}': {}", tensor, reason)
            }
            NpuError::MemoryPlanningFailed { reason } => {
                write!(f, "Memory planning failed: {}", reason)
            }
            NpuError::CodeGenerationFailed { reason } => {
                write!(f, "Code generation failed: {}", reason)
            }
            NpuError::GraphValidationFailed { reason } => {
                write!(f, "Graph validation failed: {}", reason)
            }
            NpuError::ConfigError { field, reason } => {
                write!(f, "Configuration error for '{}': {}", field, reason)
            }
            NpuError::IoError { operation, reason } => {
                write!(f, "IO error during '{}': {}", operation, reason)
            }
        }
    }
}

impl std::error::Error for NpuError {}

/// 创建 NPU 后端工厂函数
pub fn create_npu_backend(device: NpuDevice) -> Box<dyn NpuBackend> {
    match device {
        NpuDevice::IntelNPU(_) => Box::new(IntelNpuBackend::new()),
        // TODO: 实现其他后端
        NpuDevice::TPU(_) => {
            // TODO: 实现 TPU 后端
            unimplemented!("TPU backend not yet implemented")
        }
        NpuDevice::Ascend(_) => {
            // TODO: 实现昇腾后端
            unimplemented!("Ascend backend not yet implemented")
        }
        NpuDevice::Cambrian(_) => {
            // TODO: 实现寒武纪后端
            unimplemented!("Cambrian backend not yet implemented")
        }
        NpuDevice::Horizon(_) => {
            // TODO: 实现地平线后端
            unimplemented!("Horizon backend not yet implemented")
        }
        NpuDevice::Generic => {
            // 默认使用 Intel NPU 后端（通过 OpenVINO）
            Box::new(IntelNpuBackend::new())
        }
    }
}

/// 从配置字符串解析 NPU 设备
pub fn parse_npu_device(s: &str) -> NpuDevice {
    let binding = s.to_lowercase();
    let parts: Vec<&str> = binding.split('_').collect();
    match parts.get(0).copied() {
        Some("intel") | Some("npu") => {
            let generation = match parts.get(1).copied() {
                Some("meteor" | "meteorlake") => IntelNpuGeneration::MeteorLake,
                Some("lunar" | "lunarlake") => IntelNpuGeneration::LunarLake,
                Some("arrow" | "arrowlake") => IntelNpuGeneration::ArrowLake,
                _ => IntelNpuGeneration::MeteorLake, // 默认
            };
            NpuDevice::IntelNPU(generation)
        }
        Some("tpu") => {
            let generation = match parts.get(1).copied() {
                Some("v2") => TpuGeneration::V2,
                Some("v3") => TpuGeneration::V3,
                Some("v4") => TpuGeneration::V4,
                Some("v5") => TpuGeneration::V5,
                Some("edge") => TpuGeneration::Edge,
                _ => TpuGeneration::V4, // 默认
            };
            NpuDevice::TPU(generation)
        }
        Some("ascend") => {
            let soc = match parts.get(1).copied() {
                Some("310") => AscendSoc::Ascend310,
                Some("310p") => AscendSoc::Ascend310P,
                Some("910") => AscendSoc::Ascend910,
                Some("910b") => AscendSoc::Ascend910B,
                _ => AscendSoc::Ascend310, // 默认
            };
            NpuDevice::Ascend(soc)
        }
        Some("cambrian") | Some("mlu") => {
            let generation = match parts.get(1).copied() {
                Some("100") => CambrianGeneration::MLU100,
                Some("270") => CambrianGeneration::MLU270,
                Some("290") => CambrianGeneration::MLU290,
                Some("370") => CambrianGeneration::MLU370,
                _ => CambrianGeneration::MLU370, // 默认
            };
            NpuDevice::Cambrian(generation)
        }
        Some("horizon") | Some("bpu") => {
            let generation = match parts.get(1).copied() {
                Some("0") => HorizonGeneration::BPU0,
                Some("1") => HorizonGeneration::BPU1,
                Some("2") => HorizonGeneration::BPU2,
                Some("3") => HorizonGeneration::BPU3,
                _ => HorizonGeneration::BPU2, // 默认
            };
            NpuDevice::Horizon(generation)
        }
        _ => NpuDevice::Generic,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_intel_npu() {
        let device = parse_npu_device("intel_meteorlake");
        assert!(matches!(device, NpuDevice::IntelNPU(IntelNpuGeneration::MeteorLake)));

        let device = parse_npu_device("npu_lunar");
        assert!(matches!(device, NpuDevice::IntelNPU(IntelNpuGeneration::LunarLake)));
    }

    #[test]
    fn test_parse_ascend() {
        let device = parse_npu_device("ascend_910b");
        assert!(matches!(device, NpuDevice::Ascend(AscendSoc::Ascend910B)));
    }

    #[test]
    fn test_parse_tpu() {
        let device = parse_npu_device("tpu_v4");
        assert!(matches!(device, NpuDevice::TPU(TpuGeneration::V4)));
    }

    #[test]
    fn test_create_intel_backend() {
        let backend = create_npu_backend(NpuDevice::IntelNPU(IntelNpuGeneration::MeteorLake));
        assert_eq!(backend.name(), "intel_npu");
    }
}
