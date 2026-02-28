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

    // 上下文管理
    hscir_context_t* hscir_context_create();
    void hscir_context_destroy(hscir_context_t* ctx);

    // 模块管理
    hscir_module_t* hscir_module_create(hscir_context_t* ctx, const char* name);
    void hscir_module_destroy(hscir_module_t* mod);
    void hscir_module_print(hscir_module_t* mod, char** out_str); // 调用者需使用 free() 释放字符串

    // 类型获取（通过上下文确保唯一性）
    hscir_type_t* hscir_type_get_i32(hscir_context_t* ctx);
    hscir_type_t* hscir_type_get_f32(hscir_context_t* ctx);
    hscir_type_t* hscir_type_get_buffer(hscir_context_t* ctx, hscir_type_t* elem_type, size_t rank, const int64_t* shape);
    hscir_type_t* hscir_type_get_function(hscir_context_t* ctx, hscir_type_t** inputs, size_t n_inputs, hscir_type_t** outputs, size_t n_outputs);

    // 构建器
    hscir_builder_t* hscir_builder_create(hscir_context_t* ctx);
    void hscir_builder_destroy(hscir_builder_t* builder);
    void hscir_builder_set_insertion_point_to_start(hscir_builder_t* builder, hscir_block_t* block);
    void hscir_builder_set_insertion_point_to_end(hscir_builder_t* builder, hscir_block_t* block);

    // 创建区域和块
    hscir_region_t* hscir_builder_create_region(hscir_builder_t* builder);
    hscir_block_t* hscir_builder_create_block(hscir_builder_t* builder, hscir_region_t* region, hscir_type_t** arg_types, size_t n_args);

    // 创建操作并插入
    hscir_value_t* hscir_builder_create_constant_i32(hscir_builder_t* builder, int32_t value);
    hscir_value_t* hscir_builder_create_func(hscir_builder_t* builder, const char* name, hscir_type_t* func_type, hscir_region_t* body);
    hscir_value_t* hscir_builder_create_task(hscir_builder_t* builder, const char* name, hscir_type_t* func_type, hscir_region_t* body);
    hscir_value_t* hscir_builder_create_parallel_for(hscir_builder_t* builder, hscir_value_t* lb, hscir_value_t* ub, hscir_value_t* step, hscir_region_t* body);
    hscir_value_t* hscir_builder_create_spawn(hscir_builder_t* builder, hscir_value_t* task, hscir_value_t** args, size_t n_args, int await);
    hscir_value_t* hscir_builder_create_place_on(hscir_builder_t* builder, hscir_value_t* buffer, hscir_value_t* device);

    // 将模块添加到构建器当前插入点
    void hscir_builder_insert_operation(hscir_builder_t* builder, hscir_value_t* op);

#ifdef __cplusplus
}
#endif

#endif //HSCIR_CAPI_H