//! NPU (Neural Processing Unit) 后端模块
//!
//! 本模块实现 HSCIR 到 NPU 计算图的转换，支持多种 NPU 设备：
//! - Intel NPU (Meteor Lake, Lunar Lake) - 通过 OpenVINO ✅
//! - Google TPU (v2-v5, Edge TPU) - TODO
//! - 华为昇腾 (Ascend 310/910) - TODO
//! - 寒武纪 (MLU 系列) - TODO
//!
//! ## 架构
//!
//! ```text
//! HSCLang AST
//!     │
//!     ↓ NpuLowering
//! NpuGraph (计算图)
//!     │
//!     ├─→ NpuBackend.optimize_graph() (算子融合)
//!     │
//!     ├─→ MemoryPlanner (内存规划)
//!     │
//!     ├─→ NpuAutoTuner (自动调优)
//!     │
//!     ↓ NpuBackend.generate_code()
//! ONNX Model / OpenVINO IR
//!     │
//!     ↓ RuntimeGenerator
//! Python/C++ Runtime
//! ```
//!
//! ## 设备无关设计
//!
//! 参考 Triton DSL 的设备无关设计：
//! - 类型层: `NpuType` 统一类型表示，支持量化
//! - 设备层: `NpuDevice` + `NpuHardwareSpec` 硬件规格抽象
//! - 后端层: `NpuBackend` trait，策略模式支持多厂商
//! - 图层: `NpuGraph` 计算图统一表示

pub mod types;
pub mod graph;
pub mod lowering;
pub mod codegen;
pub mod memory;
pub mod autotuner;
pub mod runtime;
pub mod fusion;
pub mod backends;
pub mod onnx;

// 重导出常用类型
pub use types::{NpuType, NpuTypeKind, QuantBase, TensorLayout, SparseFormat};
pub use graph::{NpuGraph, NpuOperation, NpuOpType, NpuValue, NpuTensor, OpHints, Padding};
pub use lowering::NpuLowering;
pub use codegen::NpuCodeGenerator;
pub use memory::{MemoryPlanner, MemoryPlan, MemoryAllocator};
pub use autotuner::{NpuAutoTuner, NpuTuningParams};
pub use runtime::{RuntimeGenerator, RuntimeCode, RuntimeTarget};
pub use backends::{
    NpuBackend, NpuDevice, NpuHardwareSpec, NpuCode, NpuError,
    create_npu_backend, parse_npu_device,
    RuntimeConfig, TensorDesc, ExecutionConfig, PerformanceHint, MemoryPoolConfig,
};

// ONNX 支持
pub use onnx::{OnnxBuilder, OnnxModel, OnnxGraph, OnnxNode, OnnxDataType};

// Intel NPU 后端（已实现）
pub use backends::intel_npu::{
    IntelNpuBackend, IntelNpuGeneration, IntelNpuDevice, IntelNpuConfig,
};
