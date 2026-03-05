//! HSCIR 类型系统的 Rust 封装

use super::capi;
use std::ffi::CString;
use std::ptr;

/// 类型种类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeKind {
    Integer,
    Float,
    Buffer,
    Function,
    None,
}

/// HSCIR 类型的安全封装
#[derive(Debug)]
pub struct HscirType {
    pub(crate) raw: *mut capi::HscirType,
    pub(crate) ctx: *mut capi::HscirContext,
    pub(crate) owned: bool,
}

impl HscirType {
    /// 从原始指针创建类型（不拥有所有权）
    pub(crate) unsafe fn from_raw(raw: *mut capi::HscirType, ctx: *mut capi::HscirContext) -> Self {
        Self {
            raw,
            ctx,
            owned: false,
        }
    }

    /// 创建 i32 类型
    pub fn i32(ctx: *mut capi::HscirContext) -> Self {
        unsafe {
            let raw = capi::hscir_type_get_i32(ctx);
            Self {
                raw,
                ctx,
                owned: true,
            }
        }
    }

    /// 创建 i64 类型
    pub fn i64(ctx: *mut capi::HscirContext) -> Self {
        unsafe {
            let raw = capi::hscir_type_get_i64(ctx);
            Self {
                raw,
                ctx,
                owned: true,
            }
        }
    }

    /// 创建 f32 类型
    pub fn f32(ctx: *mut capi::HscirContext) -> Self {
        unsafe {
            let raw = capi::hscir_type_get_f32(ctx);
            Self {
                raw,
                ctx,
                owned: true,
            }
        }
    }

    /// 创建 f64 类型
    pub fn f64(ctx: *mut capi::HscirContext) -> Self {
        unsafe {
            let raw = capi::hscir_type_get_f64(ctx);
            Self {
                raw,
                ctx,
                owned: true,
            }
        }
    }

    /// 创建 bool 类型 (i1)
    pub fn bool(ctx: *mut capi::HscirContext) -> Self {
        unsafe {
            let raw = capi::hscir_type_get_i1(ctx);
            Self {
                raw,
                ctx,
                owned: true,
            }
        }
    }

    /// 创建 i8 类型
    pub fn i8(ctx: *mut capi::HscirContext) -> Self {
        unsafe {
            let raw = capi::hscir_type_get_i8(ctx);
            Self {
                raw,
                ctx,
                owned: true,
            }
        }
    }

    /// 创建 i16 类型
    pub fn i16(ctx: *mut capi::HscirContext) -> Self {
        unsafe {
            let raw = capi::hscir_type_get_i16(ctx);
            Self {
                raw,
                ctx,
                owned: true,
            }
        }
    }

    /// 创建整数类型
    pub fn integer(ctx: *mut capi::HscirContext, width: u32, is_signed: bool) -> Self {
        unsafe {
            let raw = capi::hscir_type_get_integer(ctx, width, if is_signed { 1 } else { 0 });
            Self {
                raw,
                ctx,
                owned: true,
            }
        }
    }

    /// 创建 Buffer 类型
    pub fn buffer(ctx: *mut capi::HscirContext, elem_type: &HscirType, shape: &[i64]) -> Self {
        unsafe {
            let raw = capi::hscir_type_get_buffer(ctx, elem_type.raw, shape.len(), shape.as_ptr());
            Self {
                raw,
                ctx,
                owned: true,
            }
        }
    }

    /// 创建函数类型
    pub fn function(
        ctx: *mut capi::HscirContext,
        inputs: &[&HscirType],
        outputs: &[&HscirType],
    ) -> Self {
        unsafe {
            let input_ptrs: Vec<*mut capi::HscirType> = inputs.iter().map(|t| t.raw).collect();
            let output_ptrs: Vec<*mut capi::HscirType> = outputs.iter().map(|t| t.raw).collect();

            let raw = capi::hscir_type_get_function(
                ctx,
                input_ptrs.as_ptr() as *mut _,
                inputs.len(),
                output_ptrs.as_ptr() as *mut _,
                outputs.len(),
            );
            Self {
                raw,
                ctx,
                owned: true,
            }
        }
    }

    /// 获取类型的字符串表示
    pub fn to_string(&self) -> String {
        unsafe {
            let mut out_str: *mut i8 = ptr::null_mut();
            capi::hscir_type_to_string(self.raw, &mut out_str);
            if out_str.is_null() {
                return String::from("(unknown type)");
            }
            let result = capi::from_c_string(out_str);
            capi::hscir_string_free(out_str);
            result
        }
    }

    /// 获取原始指针
    pub fn as_ptr(&self) -> *mut capi::HscirType {
        self.raw
    }
}

impl Drop for HscirType {
    fn drop(&mut self) {
        if self.owned && !self.raw.is_null() {
            unsafe {
                capi::hscir_type_destroy(self.raw);
            }
        }
    }
}

impl Clone for HscirType {
    fn clone(&self) -> Self {
        // 类型的克隆需要重新获取（因为 TypeManager 保证类型唯一）
        Self {
            raw: self.raw,
            ctx: self.ctx,
            owned: false, // 克隆的不拥有所有权
        }
    }
}
