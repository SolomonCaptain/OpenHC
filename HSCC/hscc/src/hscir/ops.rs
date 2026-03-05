//! HSCIR 操作系统的 Rust 封装

use super::capi;
use super::types::HscirType;
use std::ptr;

/// HSCIR 值的安全封装
#[derive(Debug, Clone)]
pub struct HscirValue {
    pub(crate) raw: *mut capi::HscirValue,
}

impl HscirValue {
    /// 从原始指针创建
    pub(crate) unsafe fn from_raw(raw: *mut capi::HscirValue) -> Self {
        Self { raw }
    }

    /// 获取值的类型
    pub fn get_type(&self, ctx: *mut capi::HscirContext) -> HscirType {
        unsafe {
            let ty_ptr = capi::hscir_value_get_type(self.raw);
            HscirType::from_raw(ty_ptr, ctx)
        }
    }

    /// 获取原始指针
    pub fn as_ptr(&self) -> *mut capi::HscirValue {
        self.raw
    }

    /// 检查是否为空
    pub fn is_null(&self) -> bool {
        self.raw.is_null()
    }
}

/// HSCIR 操作的安全封装
#[derive(Debug)]
pub struct HscirOperation {
    pub(crate) raw: *mut capi::HscirValue,
}

impl HscirOperation {
    /// 从值创建操作
    pub(crate) fn from_value(value: HscirValue) -> Self {
        Self { raw: value.raw }
    }

    /// 获取原始指针
    pub fn as_ptr(&self) -> *mut capi::HscirValue {
        self.raw
    }
}

/// HSCIR 基本块的安全封装
#[derive(Debug, Clone)]
pub struct HscirBlock {
    pub(crate) raw: *mut capi::HscirBlock,
}

impl HscirBlock {
    /// 从原始指针创建
    pub(crate) unsafe fn from_raw(raw: *mut capi::HscirBlock) -> Self {
        Self { raw }
    }

    /// 获取块参数
    pub fn get_argument(&self, index: usize) -> HscirValue {
        unsafe {
            let val = capi::hscir_block_get_argument(self.raw, index);
            HscirValue::from_raw(val)
        }
    }

    /// 获取原始指针
    pub fn as_ptr(&self) -> *mut capi::HscirBlock {
        self.raw
    }
}

/// HSCIR 区域的安全封装
#[derive(Debug)]
pub struct HscirRegion {
    pub(crate) raw: *mut capi::HscirRegion,
}

impl HscirRegion {
    /// 从原始指针创建
    pub(crate) unsafe fn from_raw(raw: *mut capi::HscirRegion) -> Self {
        Self { raw }
    }

    /// 获取原始指针
    pub fn as_ptr(&mut self) -> *mut capi::HscirRegion {
        self.raw
    }

    /// 消费自己，返回原始指针（用于转移所有权）
    pub fn into_raw(self) -> *mut capi::HscirRegion {
        let raw = self.raw;
        std::mem::forget(self);
        raw
    }
}

/// HSCIR 模块的安全封装
#[derive(Debug)]
pub struct HscirModule {
    pub(crate) raw: *mut capi::HscirModule,
    pub(crate) ctx: *mut capi::HscirContext,
}

impl Clone for HscirModule {
    fn clone(&self) -> Self {
        Self {
            raw: self.raw,
            ctx: self.ctx,
        }
    }
}

impl HscirModule {
    /// 创建新模块
    pub fn new(ctx: *mut capi::HscirContext, name: &str) -> Self {
        unsafe {
            let name_c = capi::to_c_string(name);
            let raw = capi::hscir_module_create(ctx, name_c.as_ptr());
            Self { raw, ctx }
        }
    }

    /// 添加操作到模块
    pub fn add_operation(&mut self, op: HscirOperation) {
        unsafe {
            capi::hscir_module_add_operation(self.raw, op.raw);
        }
    }

    /// 打印模块
    pub fn print(&self) -> String {
        unsafe {
            let mut out_str: *mut i8 = ptr::null_mut();
            capi::hscir_module_print(self.raw, &mut out_str);
            if out_str.is_null() {
                return String::from("(empty module)");
            }
            let result = capi::from_c_string(out_str);
            capi::hscir_string_free(out_str);
            result
        }
    }

    /// 获取原始指针
    pub fn as_ptr(&self) -> *mut capi::HscirModule {
        self.raw
    }
}

impl Drop for HscirModule {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe {
                capi::hscir_module_destroy(self.raw);
            }
        }
    }
}
