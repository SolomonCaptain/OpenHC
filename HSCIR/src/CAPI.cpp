#include "hscir/CAPI.h"
#include "hscir/HSCIR.h"
#include <cstring>
#include <sstream>

#include "hscir/Builder.h"

using namespace hscir;

// 辅助函数：将 std::shared_ptr<Type> 转换为 hscir_type_t*
#define TO_TYPE(ptr) reinterpret_cast<hscir_type_t*>(ptr.get())
#define FROM_TYPE(ptr) reinterpret_cast<std::shared_ptr<Type>*>(ptr)

// 上下文
struct hscir_context_t
{

};

hscir_context_t* hscir_context_create()
{
    return new hscir_context_t;
}

void hscir_context_destroy(hscir_context_t* ctx)
{
    delete ctx;
}

// 模块
struct hscir_module_t
{
    std::unique_ptr<Module> mod;
};

hscir_module_t* hscir_module_create(hscir_context_t* ctx, const char* name)
{
    auto mod = std::make_unique<Module>(name);
    return new hscir_module_t(std::move(mod));
}

void hscir_module_destroy(hscir_module_t* mod)
{
    delete mod;
}

void hscir_module_print(hscir_module_t* mod, char** out_str)
{
    std::ostringstream oss;
    mod->mod->print(oss);
    std::string s = oss.str();
    *out_str = static_cast<char*>(malloc(s.size() + 1));
    std::strcpy(*out_str, s.c_str());
}

// 类型
hscir_type_t* hscir_type_get_i32(hscir_context_t* ctx)
{
    auto type = TypeManager::getInstance().getIntegerType(32, true);
    return reinterpret_cast<hscir_type_t*>(new std::shared_ptr<Type>(type));
}

hscir_type_t* hscir_type_get_f32(hscir_context_t* ctx)
{
    auto type = TypeManager::getInstance().getFloatType(32);
    return reinterpret_cast<hscir_type_t*>(new std::shared_ptr<Type>(type));
}

hscir_type_t* hscir_type_get_buffer(hscir_context_t* ctx, hscir_type_t* elem_type, size_t rank, const int64_t* shape)
{
    auto elem = *reinterpret_cast<std::shared_ptr<Type>*>(elem_type);
    std::vector<int64_t> shapeVec(shape, shape + rank);
    auto type = TypeManager::getInstance().getBufferType(elem, shapeVec);
    return reinterpret_cast<hscir_type_t*>(new std::shared_ptr<Type>(type));
}

hscir_type_t* hscir_type_get_function(hscir_context_t* ctx, hscir_type_t** inputs, size_t n_inputs, hscir_type_t** outputs, size_t n_outputs)
{
    std::vector<std::shared_ptr<Type>> inVec, outVec;
    for (size_t i = 0; i < n_inputs; ++i)
    {
        inVec.push_back(*reinterpret_cast<std::shared_ptr<Type>*>(inputs[i]));
    }
    for (size_t i = 0; i < n_outputs; ++i)
    {
        outVec.push_back(*reinterpret_cast<std::shared_ptr<Type>*>(outputs[i]));
    }
    auto type = TypeManager::getInstance().getFunctionType(inVec, outVec);
    return reinterpret_cast<hscir_type_t*>(new std::shared_ptr<Type>(type));
}

// 构建器
struct hscir_builder_t
{
    std::unique_ptr<Builder> builder;
};

hscir_builder_t* hscir_builder_create(hscir_context_t* ctx)
{
    auto builder = std::make_unique<Builder>();
    return new hscir_builder_t(std::move(builder));
}

void hscir_builder_destroy(hscir_builder_t* builder)
{
    delete builder;
}

void hscir_builder_set_insertion_point_to_start(hscir_builder_t* builder, hscir_block_t* block)
{
    builder->builder->setInsertionPointToStart(reinterpret_cast<Block*>(block));
}

void hscir_builder_set_insertion_point_to_end(hscir_builder_t* builder, hscir_block_t* block)
{
    builder->builder->setInsertionPointToEnd(reinterpret_cast<Block*>(block));
}

hscir_region_t* hscir_builder_create_region(hscir_builder_t* builder)
{
    auto region = builder->builder->createRegion();
    return reinterpret_cast<hscir_region_t*>(region.release());
}

hscir_block_t* hscir_builder_create_block(hscir_builder_t* builder, hscir_region_t* region, hscir_type_t** arg_types, size_t n_args)
{
    auto reg = reinterpret_cast<Region*>(region);
    std::vector<std::shared_ptr<Type>> argVec;
    for (size_t i = 0; i < n_args; ++i)
    {
        argVec.push_back(*reinterpret_cast<std::shared_ptr<Type>*>(arg_types[i]));
    }
    auto block = std::make_unique<Block>();
    for (auto& t : argVec)
    {
        block->addArgument(t);
    }
    Block* ptr = block.get();
    reg->addBlock(std::move(block));
    return reinterpret_cast<hscir_block_t*>(ptr);
}

hscir_value_t* hscir_builder_create_constant_i32(hscir_builder_t* builder, int32_t value)
{
    // 实现常量创建并插入，返回结果值
    auto type = TypeManager::getInstance().getIntegerType(32, true);
    auto op = std::make_unique<Operation>("constant");
    op->addResultType(type);
    op->setAttribute("value", std::make_unique<IntegerAttr>(value));
    // 插入当前块
    builder->builder->insert(std::move(op));
    return nullptr;
}

hscir_value_t* hscir_builder_create_func(hscir_builder_t* builder, const char* name, hscir_type_t* func_type, hscir_region_t* body)
{
    auto funcTy = *reinterpret_cast<std::shared_ptr<Type>*>(func_type);
    auto region = std::unique_ptr<Region>(reinterpret_cast<Region*>(body));
    auto op = builder->builder->createFuncOp(name, {}, {}, std::move(region));
    builder->builder->insert(std::move(op));
    return nullptr;
}

hscir_value_t* hscir_builder_create_task(hscir_builder_t* builder, const char* name, hscir_type_t* func_type, hscir_region_t* body)
{
    auto funcTy = *reinterpret_cast<std::shared_ptr<Type>*>(func_type);
    auto region = std::unique_ptr<Region>(reinterpret_cast<Region*>(body));
    auto op = builder->builder->createTaskOp(name, {}, {}, std::move(region));
    builder->builder->insert(std::move(op));
    return nullptr;
}

hscir_value_t* hscir_builder_create_parallel_for(hscir_builder_t* builder, hscir_value_t* lb, hscir_value_t* ub, hscir_value_t* step, hscir_region_t* body)
{
    auto lbVal = reinterpret_cast<Value*>(lb);
    auto ubVal = reinterpret_cast<Value*>(ub);
    auto stepVal = reinterpret_cast<Value*>(step);
    auto region = std::unique_ptr<Region>(reinterpret_cast<Region*>(body));
    auto op = builder->builder->createParallelForOp(std::shared_ptr<Value>(lbVal), std::shared_ptr<Value>(ubVal), std::shared_ptr<Value>(stepVal), std::move(region));
    builder->builder->insert(std::move(op));
    return nullptr;
}

hscir_value_t* hscir_builder_create_spawn(hscir_builder_t* builder, hscir_value_t* task, hscir_value_t** args, size_t n_args, int await)
{
    std::vector<std::shared_ptr<Value>> argVec;
    for (size_t i = 0; i < n_args; ++i)
    {
        argVec.push_back(std::shared_ptr<Value>(reinterpret_cast<Value*>(args[i])));
    }
    auto op = builder->builder->createSpawnOp(std::shared_ptr<Value>(reinterpret_cast<Value*>(task)), argVec, await != 0);
    builder->builder->insert(std::move(op));
    return nullptr;
}

hscir_value_t* hscir_builder_create_place_on(hscir_builder_t* builder, hscir_value_t* buffer, hscir_value_t* device)
{
    auto op = builder->builder->createPlaceOnOp(std::shared_ptr<Value>(reinterpret_cast<Value*>(buffer)), std::shared_ptr<Value>(reinterpret_cast<Value*>(device)));
    builder->builder->insert(std::move(op));
    return nullptr;
}

void hscir_builder_insert_operation(hscir_builder_t* builder, hscir_value_t* op)
{
    // TODO: 添加 hscir_operation_t 类型来代表操作
    (void)builder;
    (void)op;
}