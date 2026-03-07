//! HSCIR Rust 绑定模块
//!
//! 本模块提供对 HSCIR C++ 库的安全 Rust 封装，支持：
//! - 类型系统（整数、浮点、Buffer、函数类型）
//! - 操作系统（Operation、Value、Block、Region、Module）
//! - Builder API（IR 构建接口）
//! - Pass 管理器和分析 Pass

pub mod capi;
pub mod types;
pub mod ops;
pub mod builder;
pub mod pass;

pub use types::{HscirType, TypeKind};
pub use ops::{HscirValue, HscirOperation, HscirBlock, HscirRegion, HscirModule};
pub use builder::HscirBuilder;

// Pass 相关导出
pub use pass::{
    Pass, PassKind, PassResult, PassContext, PassManager, PassStatistics,
    // 分析 Pass
    DataFlowAnalysisPass, DataFlowAnalysisData,
    DependenceAnalysisPass, DependenceAnalysisData, LoopCarriedDep, DepType,
    DeviceAffinityAnalysisPass, DeviceAffinityData, CrossDeviceTransfer,
    // 工具 Pass
    VerificationPass, PrintPass,
    // 诊断
    PassDiagnostic, DiagnosticLevel,
};
