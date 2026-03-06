//! Triton DSL 后端模块
//!
//! 本模块实现 HSCIR 到 Triton Python 代码的转换，支持：
//! - 类型映射：HSCIR Type → Triton Type
//! - 操作映射：HSCIR Operations → Triton Python API
//! - 内核生成：生成 @triton.jit 装饰的内核函数
//! - 宿主代码：生成调用 Triton 内核的启动代码
//! - 算子融合：自动融合多个操作到单个内核
//! - 自动调优：根据硬件特性选择最优配置
//!
//! ## 架构
//!
//! ```text
//! HSCIR (HSCIR Operations)
//!     │
//!     ↓ TritonLowering
//! Triton IR (TritonModule)
//!     │
//!     ├─→ FusionOptimizer (算子融合)
//!     │
//!     ├─→ AutoTuner (自动调优)
//!     │
//!     ↓ TritonCodeGenerator
//! Python Code (Triton Kernels)
//! ```

pub mod types;
pub mod kernel;
pub mod codegen;
pub mod lowering;
pub mod templates;
pub mod fusion;
pub mod autotuner;
pub mod hip_backend;
pub mod tile_ir;

pub use types::{TritonType, TritonTypeKind};
pub use kernel::{TritonKernel, TritonModule, TritonStatement, TritonExpr, TritonConfig};
pub use codegen::TritonCodeGenerator;
pub use lowering::TritonLowering;
pub use templates::{VectorKernels, ReduceKernels, MatmulKernels, SoftmaxKernels, KernelRegistry};
pub use fusion::{FusionOptimizer, FusionPattern, FusionGroup, FusibleOp, FusionResult, FusedKernel};
pub use autotuner::{AutoTuner, HardwareSpec, TuningParams, DataType, KernelType, ProblemSize};
pub use hip_backend::{ROCmBackend, ROCmConfig, AMDArchitecture, get_amd_optimized_config};
pub use tile_ir::{TileIRBackend, TileIRConfig, TileIRVersion, get_h100_optimized_config};
