//! HSCIR C API 的 FFI 绑定
//!
//! 本模块提供对 libhscir.dll 中 C API 的原始 FFI 绑定。

use std::os::raw::{c_char, c_int, c_void};
use std::ffi::CString;
use std::ptr;

/// 不透明句柄类型定义
#[repr(C)]
pub struct HscirContext {
    _private: [u8; 0],
}

#[repr(C)]
pub struct HscirModule {
    _private: [u8; 0],
}

#[repr(C)]
pub struct HscirType {
    _private: [u8; 0],
}

#[repr(C)]
pub struct HscirValue {
    _private: [u8; 0],
}

#[repr(C)]
pub struct HscirBuilder {
    _private: [u8; 0],
}

#[repr(C)]
pub struct HscirBlock {
    _private: [u8; 0],
}

#[repr(C)]
pub struct HscirRegion {
    _private: [u8; 0],
}

// 链接到 HSCIR 库
#[link(name = "hscir", kind = "dylib")]
unsafe extern "C" {
    // ========== 上下文管理 ==========
    pub fn hscir_context_create() -> *mut HscirContext;
    pub fn hscir_context_destroy(ctx: *mut HscirContext);

    // ========== 模块管理 ==========
    pub fn hscir_module_create(ctx: *mut HscirContext, name: *const c_char) -> *mut HscirModule;
    pub fn hscir_module_destroy(module: *mut HscirModule);
    pub fn hscir_module_print(module: *mut HscirModule, out_str: *mut *mut c_char);

    // ========== 类型获取 ==========
    pub fn hscir_type_get_i32(ctx: *mut HscirContext) -> *mut HscirType;
    pub fn hscir_type_get_i64(ctx: *mut HscirContext) -> *mut HscirType;
    pub fn hscir_type_get_f32(ctx: *mut HscirContext) -> *mut HscirType;
    pub fn hscir_type_get_f64(ctx: *mut HscirContext) -> *mut HscirType;
    pub fn hscir_type_get_i1(ctx: *mut HscirContext) -> *mut HscirType;
    pub fn hscir_type_get_i8(ctx: *mut HscirContext) -> *mut HscirType;
    pub fn hscir_type_get_i16(ctx: *mut HscirContext) -> *mut HscirType;
    
    /// 创建整数类型
    pub fn hscir_type_get_integer(
        ctx: *mut HscirContext,
        width: u32,
        is_signed: c_int,
    ) -> *mut HscirType;

    /// 创建 Buffer 类型
    pub fn hscir_type_get_buffer(
        ctx: *mut HscirContext,
        elem_type: *mut HscirType,
        rank: usize,
        shape: *const i64,
    ) -> *mut HscirType;

    /// 创建函数类型
    pub fn hscir_type_get_function(
        ctx: *mut HscirContext,
        inputs: *mut *mut HscirType,
        n_inputs: usize,
        outputs: *mut *mut HscirType,
        n_outputs: usize,
    ) -> *mut HscirType;

    /// 销毁类型
    pub fn hscir_type_destroy(ty: *mut HscirType);

    // ========== 构建器 ==========
    pub fn hscir_builder_create(ctx: *mut HscirContext) -> *mut HscirBuilder;
    pub fn hscir_builder_destroy(builder: *mut HscirBuilder);

    pub fn hscir_builder_set_insertion_point_to_start(builder: *mut HscirBuilder, block: *mut HscirBlock);
    pub fn hscir_builder_set_insertion_point_to_end(builder: *mut HscirBuilder, block: *mut HscirBlock);

    // ========== 区域和块 ==========
    pub fn hscir_builder_create_region(builder: *mut HscirBuilder) -> *mut HscirRegion;
    pub fn hscir_builder_create_block(
        builder: *mut HscirBuilder,
        region: *mut HscirRegion,
        arg_types: *mut *mut HscirType,
        n_args: usize,
    ) -> *mut HscirBlock;

    /// 获取块的参数
    pub fn hscir_block_get_argument(block: *mut HscirBlock, index: usize) -> *mut HscirValue;

    // ========== 操作创建 ==========
    /// 创建整数常量
    pub fn hscir_builder_create_constant_i32(builder: *mut HscirBuilder, value: i32) -> *mut HscirValue;
    pub fn hscir_builder_create_constant_i64(builder: *mut HscirBuilder, value: i64) -> *mut HscirValue;
    pub fn hscir_builder_create_constant_f32(builder: *mut HscirBuilder, value: f32) -> *mut HscirValue;
    pub fn hscir_builder_create_constant_f64(builder: *mut HscirBuilder, value: f64) -> *mut HscirValue;
    pub fn hscir_builder_create_constant_bool(builder: *mut HscirBuilder, value: c_int) -> *mut HscirValue;

    /// 创建通用常量
    pub fn hscir_builder_create_constant(
        builder: *mut HscirBuilder,
        ty: *mut HscirType,
        value: i64,
    ) -> *mut HscirValue;

    /// 创建二元操作
    pub fn hscir_builder_create_binary_op(
        builder: *mut HscirBuilder,
        op_name: *const c_char,
        lhs: *mut HscirValue,
        rhs: *mut HscirValue,
    ) -> *mut HscirValue;

    /// 创建函数操作
    pub fn hscir_builder_create_func(
        builder: *mut HscirBuilder,
        name: *const c_char,
        func_type: *mut HscirType,
        body: *mut HscirRegion,
    ) -> *mut HscirValue;

    /// 创建任务操作
    pub fn hscir_builder_create_task(
        builder: *mut HscirBuilder,
        name: *const c_char,
        func_type: *mut HscirType,
        body: *mut HscirRegion,
    ) -> *mut HscirValue;

    /// 创建并行循环操作
    pub fn hscir_builder_create_parallel_for(
        builder: *mut HscirBuilder,
        lb: *mut HscirValue,
        ub: *mut HscirValue,
        step: *mut HscirValue,
        body: *mut HscirRegion,
    ) -> *mut HscirValue;

    /// 创建 spawn 操作
    pub fn hscir_builder_create_spawn(
        builder: *mut HscirBuilder,
        task: *mut HscirValue,
        args: *mut *mut HscirValue,
        n_args: usize,
        await_: c_int,
    ) -> *mut HscirValue;

    /// 创建 place_on 操作
    pub fn hscir_builder_create_place_on(
        builder: *mut HscirBuilder,
        buffer: *mut HscirValue,
        device: *mut HscirValue,
    ) -> *mut HscirValue;

    /// 创建 move_to 操作
    pub fn hscir_builder_create_move_to(
        builder: *mut HscirBuilder,
        buffer: *mut HscirValue,
        device: *mut HscirValue,
    ) -> *mut HscirValue;

    /// 创建 return 操作
    pub fn hscir_builder_create_return(
        builder: *mut HscirBuilder,
        value: *mut HscirValue,
    );

    /// 创建 call 操作
    pub fn hscir_builder_create_call(
        builder: *mut HscirBuilder,
        func: *const c_char,
        args: *mut *mut HscirValue,
        n_args: usize,
    ) -> *mut HscirValue;

    /// 创建 load 操作
    pub fn hscir_builder_create_load(
        builder: *mut HscirBuilder,
        buffer: *mut HscirValue,
        index: *mut HscirValue,
    ) -> *mut HscirValue;

    /// 创建 store 操作
    pub fn hscir_builder_create_store(
        builder: *mut HscirBuilder,
        value: *mut HscirValue,
        buffer: *mut HscirValue,
        index: *mut HscirValue,
    );

    /// 创建 alloc 操作
    pub fn hscir_builder_create_alloc(
        builder: *mut HscirBuilder,
        elem_type: *mut HscirType,
        shape: *const i64,
        rank: usize,
    ) -> *mut HscirValue;

    /// 将操作添加到模块
    pub fn hscir_module_add_operation(module: *mut HscirModule, op: *mut HscirValue);

    /// 获取值的类型
    pub fn hscir_value_get_type(value: *mut HscirValue) -> *mut HscirType;

    /// 类型转换为字符串
    pub fn hscir_type_to_string(ty: *mut HscirType, out_str: *mut *mut c_char);

    /// 销毁字符串（由 C 分配）
    pub fn hscir_string_free(s: *mut c_char);
}

// ========== 辅助函数 ==========

/// 安全地将 Rust 字符串转换为 C 字符串
pub fn to_c_string(s: &str) -> CString {
    CString::new(s).expect("String contains null byte")
}

/// 安全地从 C 字符串获取 Rust 字符串
/// 
/// # Safety
/// 调用者必须确保指针有效且以 null 结尾
pub unsafe fn from_c_string(s: *const c_char) -> String {
    if s.is_null() {
        return String::new();
    }
    std::ffi::CStr::from_ptr(s).to_string_lossy().into_owned()
}
