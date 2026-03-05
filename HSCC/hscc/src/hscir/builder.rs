//! HSCIR Builder 的 Rust 封装

use super::capi;
use super::types::HscirType;
use super::ops::{HscirValue, HscirBlock, HscirRegion, HscirOperation};
use crate::ast::BinaryOp;

/// HSCIR Builder 的安全封装
pub struct HscirBuilder {
    pub(crate) raw: *mut capi::HscirBuilder,
    pub(crate) ctx: *mut capi::HscirContext,
    /// 当前块指针（用于跟踪插入点）
    current_block: Option<*mut capi::HscirBlock>,
}

impl HscirBuilder {
    /// 创建新的 Builder
    pub fn new(ctx: *mut capi::HscirContext) -> Self {
        unsafe {
            let raw = capi::hscir_builder_create(ctx);
            Self {
                raw,
                ctx,
                current_block: None,
            }
        }
    }

    // ========== 插入点管理 ==========

    /// 设置插入点到块的开始
    pub fn set_insertion_point_to_start(&mut self, block: &HscirBlock) {
        unsafe {
            capi::hscir_builder_set_insertion_point_to_start(self.raw, block.raw);
            self.current_block = Some(block.raw);
        }
    }

    /// 设置插入点到块的末尾
    pub fn set_insertion_point_to_end(&mut self, block: &HscirBlock) {
        unsafe {
            capi::hscir_builder_set_insertion_point_to_end(self.raw, block.raw);
            self.current_block = Some(block.raw);
        }
    }

    // ========== 区域和块创建 ==========

    /// 创建新区域
    pub fn create_region(&mut self) -> HscirRegion {
        unsafe {
            let raw = capi::hscir_builder_create_region(self.raw);
            HscirRegion::from_raw(raw)
        }
    }

    /// 在区域中创建块
    pub fn create_block(&mut self, region: &mut HscirRegion, arg_types: &[&HscirType]) -> HscirBlock {
        unsafe {
            let type_ptrs: Vec<*mut capi::HscirType> = arg_types.iter().map(|t| t.raw).collect();
            let raw = capi::hscir_builder_create_block(
                self.raw,
                region.raw,
                type_ptrs.as_ptr() as *mut _,
                arg_types.len(),
            );
            HscirBlock::from_raw(raw)
        }
    }

    // ========== 常量创建 ==========

    /// 创建 i32 常量
    pub fn create_constant_i32(&mut self, value: i32) -> HscirValue {
        unsafe {
            let val = capi::hscir_builder_create_constant_i32(self.raw, value);
            HscirValue::from_raw(val)
        }
    }

    /// 创建 i64 常量
    pub fn create_constant_i64(&mut self, value: i64) -> HscirValue {
        unsafe {
            let val = capi::hscir_builder_create_constant_i64(self.raw, value);
            HscirValue::from_raw(val)
        }
    }

    /// 创建 f32 常量
    pub fn create_constant_f32(&mut self, value: f32) -> HscirValue {
        unsafe {
            let val = capi::hscir_builder_create_constant_f32(self.raw, value);
            HscirValue::from_raw(val)
        }
    }

    /// 创建 f64 常量
    pub fn create_constant_f64(&mut self, value: f64) -> HscirValue {
        unsafe {
            let val = capi::hscir_builder_create_constant_f64(self.raw, value);
            HscirValue::from_raw(val)
        }
    }

    /// 创建 bool 常量
    pub fn create_constant_bool(&mut self, value: bool) -> HscirValue {
        unsafe {
            let val = capi::hscir_builder_create_constant_bool(self.raw, if value { 1 } else { 0 });
            HscirValue::from_raw(val)
        }
    }

    /// 创建通用常量
    pub fn create_constant(&mut self, ty: &HscirType, value: i64) -> HscirValue {
        unsafe {
            let val = capi::hscir_builder_create_constant(self.raw, ty.raw, value);
            HscirValue::from_raw(val)
        }
    }

    // ========== 二元操作 ==========

    /// 创建二元操作
    pub fn create_binary_op(&mut self, op: BinaryOp, lhs: &HscirValue, rhs: &HscirValue) -> HscirValue {
        let op_name = match op {
            BinaryOp::Add => "hsc.add",
            BinaryOp::Sub => "hsc.sub",
            BinaryOp::Mul => "hsc.mul",
            BinaryOp::Div => "hsc.div",
            BinaryOp::Eq => "hsc.eq",
            BinaryOp::Ne => "hsc.ne",
            BinaryOp::Lt => "hsc.lt",
            BinaryOp::Le => "hsc.le",
            BinaryOp::Gt => "hsc.gt",
            BinaryOp::Ge => "hsc.ge",
            BinaryOp::And => "hsc.and",
            BinaryOp::Or => "hsc.or",
        };
        unsafe {
            let op_name_c = capi::to_c_string(op_name);
            let val = capi::hscir_builder_create_binary_op(self.raw, op_name_c.as_ptr(), lhs.raw, rhs.raw);
            HscirValue::from_raw(val)
        }
    }

    // ========== 函数/任务创建 ==========

    /// 创建函数操作
    pub fn create_func_op(
        &mut self,
        name: &str,
        func_type: &HscirType,
        body: HscirRegion,
    ) -> HscirOperation {
        unsafe {
            let name_c = capi::to_c_string(name);
            let val = capi::hscir_builder_create_func(self.raw, name_c.as_ptr(), func_type.raw, body.raw);
            HscirOperation::from_value(HscirValue::from_raw(val))
        }
    }

    /// 创建任务操作
    pub fn create_task_op(
        &mut self,
        name: &str,
        func_type: &HscirType,
        body: HscirRegion,
    ) -> HscirOperation {
        unsafe {
            let name_c = capi::to_c_string(name);
            let val = capi::hscir_builder_create_task(self.raw, name_c.as_ptr(), func_type.raw, body.raw);
            HscirOperation::from_value(HscirValue::from_raw(val))
        }
    }

    // ========== 并行循环 ==========

    /// 创建并行循环操作
    pub fn create_parallel_for_op(
        &mut self,
        lb: &HscirValue,
        ub: &HscirValue,
        step: &HscirValue,
        body: HscirRegion,
    ) -> HscirOperation {
        unsafe {
            let val = capi::hscir_builder_create_parallel_for(
                self.raw,
                lb.raw,
                ub.raw,
                step.raw,
                body.raw,
            );
            HscirOperation::from_value(HscirValue::from_raw(val))
        }
    }

    // ========== 异构操作 ==========

    /// 创建 spawn 操作
    pub fn create_spawn_op(
        &mut self,
        task: &HscirValue,
        args: &[&HscirValue],
        await_: bool,
    ) -> HscirValue {
        unsafe {
            let arg_ptrs: Vec<*mut capi::HscirValue> = args.iter().map(|v| v.raw).collect();
            let val = capi::hscir_builder_create_spawn(
                self.raw,
                task.raw,
                arg_ptrs.as_ptr() as *mut _,
                args.len(),
                if await_ { 1 } else { 0 },
            );
            HscirValue::from_raw(val)
        }
    }

    /// 创建 place_on 操作
    pub fn create_place_on_op(&mut self, buffer: &HscirValue, device: &HscirValue) -> HscirValue {
        unsafe {
            let val = capi::hscir_builder_create_place_on(self.raw, buffer.raw, device.raw);
            HscirValue::from_raw(val)
        }
    }

    /// 创建 move_to 操作
    pub fn create_move_to_op(&mut self, buffer: &HscirValue, device: &HscirValue) -> HscirValue {
        unsafe {
            let val = capi::hscir_builder_create_move_to(self.raw, buffer.raw, device.raw);
            HscirValue::from_raw(val)
        }
    }

    // ========== 控制流 ==========

    /// 创建 return 操作
    pub fn create_return_op(&mut self, value: Option<&HscirValue>) {
        unsafe {
            match value {
                Some(v) => capi::hscir_builder_create_return(self.raw, v.raw),
                None => capi::hscir_builder_create_return(self.raw, std::ptr::null_mut()),
            }
        }
    }

    /// 创建 call 操作
    pub fn create_call_op(&mut self, func_name: &str, args: &[&HscirValue]) -> HscirValue {
        unsafe {
            let name_c = capi::to_c_string(func_name);
            let arg_ptrs: Vec<*mut capi::HscirValue> = args.iter().map(|v| v.raw).collect();
            let val = capi::hscir_builder_create_call(
                self.raw,
                name_c.as_ptr(),
                arg_ptrs.as_ptr() as *mut _,
                args.len(),
            );
            HscirValue::from_raw(val)
        }
    }

    // ========== 内存操作 ==========

    /// 创建 load 操作
    pub fn create_load_op(&mut self, buffer: &HscirValue, index: &HscirValue) -> HscirValue {
        unsafe {
            let val = capi::hscir_builder_create_load(self.raw, buffer.raw, index.raw);
            HscirValue::from_raw(val)
        }
    }

    /// 创建 store 操作
    pub fn create_store_op(&mut self, value: &HscirValue, buffer: &HscirValue, index: &HscirValue) {
        unsafe {
            capi::hscir_builder_create_store(self.raw, value.raw, buffer.raw, index.raw);
        }
    }

    /// 创建 alloc 操作
    pub fn create_alloc_op(&mut self, elem_type: &HscirType, shape: &[i64]) -> HscirValue {
        unsafe {
            let val = capi::hscir_builder_create_alloc(self.raw, elem_type.raw, shape.as_ptr(), shape.len());
            HscirValue::from_raw(val)
        }
    }

    /// 获取上下文指针
    pub fn context(&self) -> *mut capi::HscirContext {
        self.ctx
    }
}

impl Drop for HscirBuilder {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe {
                capi::hscir_builder_destroy(self.raw);
            }
        }
    }
}
