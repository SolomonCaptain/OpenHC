//! HSCIR Rust 绑定模块
//!
//! 本模块提供对 HSCIR C++ 库的安全 Rust 封装，支持：
//! - 类型系统（整数、浮点、Buffer、函数类型）
//! - 操作系统（Operation、Value、Block、Region、Module）
//! - Builder API（IR 构建接口）

pub mod capi;
pub mod types;
pub mod ops;
pub mod builder;

pub use types::{HscirType, TypeKind};
pub use ops::{HscirValue, HscirOperation, HscirBlock, HscirRegion, HscirModule};
pub use builder::HscirBuilder;
