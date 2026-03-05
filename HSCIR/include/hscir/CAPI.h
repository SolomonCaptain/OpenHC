#ifndef HSCIR_CAPI_H
#define HSCIR_CAPI_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

    // 不透明句柄
    typedef struct hscir_context_t hscir_context_t;
    typedef struct hscir_module_t hscir_module_t;
    typedef struct hscir_type_t hscir_type_t;
    typedef struct hscir_value_t hscir_value_t;
    typedef struct hscir_builder_t hscir_builder_t;
    typedef struct hscir_block_t hscir_block_t;
    typedef struct hscir_region_t hscir_region_t;

    // ========== 上下文管理 ==========
    hscir_context_t* hscir_context_create();
    void hscir_context_destroy(hscir_context_t* ctx);

    // ========== 模块管理 ==========
    hscir_module_t* hscir_module_create(hscir_context_t* ctx, const char* name);
    void hscir_module_destroy(hscir_module_t* mod);
    void hscir_module_print(hscir_module_t* mod, char** out_str); // 调用者需使用 free() 释放字符串
    void hscir_module_add_operation(hscir_module_t* mod, hscir_value_t* op);

    // ========== 类型获取（通过上下文确保唯一性）==========
    hscir_type_t* hscir_type_get_i1(hscir_context_t* ctx);
    hscir_type_t* hscir_type_get_i8(hscir_context_t* ctx);
    hscir_type_t* hscir_type_get_i16(hscir_context_t* ctx);
    hscir_type_t* hscir_type_get_i32(hscir_context_t* ctx);
    hscir_type_t* hscir_type_get_i64(hscir_context_t* ctx);
    hscir_type_t* hscir_type_get_i128(hscir_context_t* ctx);
    hscir_type_t* hscir_type_get_f32(hscir_context_t* ctx);
    hscir_type_t* hscir_type_get_f64(hscir_context_t* ctx);

    /// 创建整数类型（通用接口）
    hscir_type_t* hscir_type_get_integer(hscir_context_t* ctx, unsigned width, int is_signed);

    /// 创建 Buffer 类型
    hscir_type_t* hscir_type_get_buffer(hscir_context_t* ctx, hscir_type_t* elem_type, size_t rank, const int64_t* shape);

    /// 创建函数类型
    hscir_type_t* hscir_type_get_function(hscir_context_t* ctx, hscir_type_t** inputs, size_t n_inputs, hscir_type_t** outputs, size_t n_outputs);

    /// 销毁类型
    void hscir_type_destroy(hscir_type_t* ty);

    /// 类型转字符串
    void hscir_type_to_string(hscir_type_t* ty, char** out_str);

    // ========== 构建器 ==========
    hscir_builder_t* hscir_builder_create(hscir_context_t* ctx);
    void hscir_builder_destroy(hscir_builder_t* builder);
    void hscir_builder_set_insertion_point_to_start(hscir_builder_t* builder, hscir_block_t* block);
    void hscir_builder_set_insertion_point_to_end(hscir_builder_t* builder, hscir_block_t* block);

    // ========== 区域和块 ==========
    hscir_region_t* hscir_builder_create_region(hscir_builder_t* builder);
    hscir_block_t* hscir_builder_create_block(hscir_builder_t* builder, hscir_region_t* region, hscir_type_t** arg_types, size_t n_args);
    hscir_value_t* hscir_block_get_argument(hscir_block_t* block, size_t index);

    // ========== 常量创建 ==========
    hscir_value_t* hscir_builder_create_constant_i32(hscir_builder_t* builder, int32_t value);
    hscir_value_t* hscir_builder_create_constant_i64(hscir_builder_t* builder, int64_t value);
    hscir_value_t* hscir_builder_create_constant_f32(hscir_builder_t* builder, float value);
    hscir_value_t* hscir_builder_create_constant_f64(hscir_builder_t* builder, double value);
    hscir_value_t* hscir_builder_create_constant_bool(hscir_builder_t* builder, int value);
    hscir_value_t* hscir_builder_create_constant(hscir_builder_t* builder, hscir_type_t* ty, int64_t value);

    // ========== 二元操作 ==========
    hscir_value_t* hscir_builder_create_binary_op(hscir_builder_t* builder, const char* op_name, hscir_value_t* lhs, hscir_value_t* rhs);

    // ========== 函数/任务操作 ==========
    hscir_value_t* hscir_builder_create_func(hscir_builder_t* builder, const char* name, hscir_type_t* func_type, hscir_region_t* body);
    hscir_value_t* hscir_builder_create_task(hscir_builder_t* builder, const char* name, hscir_type_t* func_type, hscir_region_t* body);

    // ========== 并行循环 ==========
    hscir_value_t* hscir_builder_create_parallel_for(hscir_builder_t* builder, hscir_value_t* lb, hscir_value_t* ub, hscir_value_t* step, hscir_region_t* body);

    // ========== 异构操作 ==========
    hscir_value_t* hscir_builder_create_spawn(hscir_builder_t* builder, hscir_value_t* task, hscir_value_t** args, size_t n_args, int await);
    hscir_value_t* hscir_builder_create_place_on(hscir_builder_t* builder, hscir_value_t* buffer, hscir_value_t* device);
    hscir_value_t* hscir_builder_create_move_to(hscir_builder_t* builder, hscir_value_t* buffer, hscir_value_t* device);

    // ========== 控制流 ==========
    void hscir_builder_create_return(hscir_builder_t* builder, hscir_value_t* value);
    hscir_value_t* hscir_builder_create_call(hscir_builder_t* builder, const char* func_name, hscir_value_t** args, size_t n_args);

    // ========== 内存操作 ==========
    hscir_value_t* hscir_builder_create_load(hscir_builder_t* builder, hscir_value_t* buffer, hscir_value_t* index);
    void hscir_builder_create_store(hscir_builder_t* builder, hscir_value_t* value, hscir_value_t* buffer, hscir_value_t* index);
    hscir_value_t* hscir_builder_create_alloc(hscir_builder_t* builder, hscir_type_t* elem_type, const int64_t* shape, size_t rank);

    // ========== 值操作 ==========
    hscir_type_t* hscir_value_get_type(hscir_value_t* value);

    // ========== 工具函数 ==========
    void hscir_string_free(char* s);
    void hscir_builder_insert_operation(hscir_builder_t* builder, hscir_value_t* op);

#ifdef __cplusplus
}
#endif

#endif //HSCIR_CAPI_H