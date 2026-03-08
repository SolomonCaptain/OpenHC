//! CPU 后端模块
//!
//! 本模块实现 HSCIR 到 CPU C++ 代码的转换，支持：
//! - 类型映射：HSCIR Type → C++ Type
//! - 操作映射：HSCIR Operations → C++ 代码
//! - 并行化：使用 OpenMP 进行多线程并行
//! - 向量化：使用 SIMD 指令优化
//!
//! ## 架构
//!
//! ```text
//! HSCIR / AST
//!     │
//!     ↓ CpuLowering
//! CPU IR (CpuModule)
//!     │
//!     ├─→ Parallelizer (并行化优化)
//!     │
//!     ├─→ Vectorizer (向量化优化)
//!     │
//!     ↓ CpuCodeGenerator
//! C++ Code (.cpp)
//! ```

pub mod types;
pub mod codegen;
pub mod parallel;
pub mod lowering;
pub mod runtime;

pub use types::{CpuType, CpuTypeKind};
pub use codegen::{CpuCodeGenerator, generate_cpu_code, generate_cpu_code_with_config};
pub use parallel::{Parallelizer, ParallelConfig, ThreadSchedule};
pub use lowering::{CpuLowering, CpuLoweringContext, CpuModule};
pub use runtime::{CpuRuntime, RuntimeConfig};
