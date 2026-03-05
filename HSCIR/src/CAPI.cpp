#include "hscir/CAPI.h"
#include "hscir/HSCIR.h"
#include "hscir/Builder.h"
#include <cstring>
#include <sstream>
#include <cstdlib>

using namespace hscir;

// ========== 上下文 ==========
struct hscir_context_t
{
    // 上下文可以包含全局状态
};

hscir_context_t* hscir_context_create()
{
    return new hscir_context_t;
}

void hscir_context_destroy(hscir_context_t* ctx)
{
    delete ctx;
}

// ========== 模块 ==========
struct hscir_module_t
{
    std::unique_ptr<Module> mod;
    std::vector<std::unique_ptr<Operation>> ops;
};

hscir_module_t* hscir_module_create(hscir_context_t* ctx, const char* name)
{
    auto m = new hscir_module_t();
    m->mod = std::make_unique<Module>(name);
    return m;
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

void hscir_module_add_operation(hscir_module_t* mod, hscir_value_t* op)
{
    // TODO: 实现将操作添加到模块
}

// ========== 类型 ==========
hscir_type_t* hscir_type_get_i1(hscir_context_t* ctx)
{
    auto type = TypeManager::getInstance().getIntegerType(1, false);
    return reinterpret_cast<hscir_type_t*>(new std::shared_ptr<Type>(type));
}

hscir_type_t* hscir_type_get_i8(hscir_context_t* ctx)
{
    auto type = TypeManager::getInstance().getIntegerType(8, true);
    return reinterpret_cast<hscir_type_t*>(new std::shared_ptr<Type>(type));
}

hscir_type_t* hscir_type_get_i16(hscir_context_t* ctx)
{
    auto type = TypeManager::getInstance().getIntegerType(16, true);
    return reinterpret_cast<hscir_type_t*>(new std::shared_ptr<Type>(type));
}

hscir_type_t* hscir_type_get_i32(hscir_context_t* ctx)
{
    auto type = TypeManager::getInstance().getIntegerType(32, true);
    return reinterpret_cast<hscir_type_t*>(new std::shared_ptr<Type>(type));
}

hscir_type_t* hscir_type_get_i64(hscir_context_t* ctx)
{
    auto type = TypeManager::getInstance().getIntegerType(64, true);
    return reinterpret_cast<hscir_type_t*>(new std::shared_ptr<Type>(type));
}

hscir_type_t* hscir_type_get_i128(hscir_context_t* ctx)
{
    auto type = TypeManager::getInstance().getIntegerType(128, true);
    return reinterpret_cast<hscir_type_t*>(new std::shared_ptr<Type>(type));
}

hscir_type_t* hscir_type_get_f32(hscir_context_t* ctx)
{
    auto type = TypeManager::getInstance().getFloatType(32);
    return reinterpret_cast<hscir_type_t*>(new std::shared_ptr<Type>(type));
}

hscir_type_t* hscir_type_get_f64(hscir_context_t* ctx)
{
    auto type = TypeManager::getInstance().getFloatType(64);
    return reinterpret_cast<hscir_type_t*>(new std::shared_ptr<Type>(type));
}

hscir_type_t* hscir_type_get_integer(hscir_context_t* ctx, unsigned width, int is_signed)
{
    auto type = TypeManager::getInstance().getIntegerType(width, is_signed != 0);
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

void hscir_type_destroy(hscir_type_t* ty)
{
    if (ty)
    {
        delete reinterpret_cast<std::shared_ptr<Type>*>(ty);
    }
}

void hscir_type_to_string(hscir_type_t* ty, char** out_str)
{
    auto typePtr = reinterpret_cast<std::shared_ptr<Type>*>(ty);
    if (typePtr && *typePtr)
    {
        std::string s = (*typePtr)->toString();
        *out_str = static_cast<char*>(malloc(s.size() + 1));
        std::strcpy(*out_str, s.c_str());
    }
    else
    {
        *out_str = static_cast<char*>(malloc(8));
        std::strcpy(*out_str, "(null)");
    }
}

// ========== 构建器 ==========
struct hscir_builder_t
{
    std::unique_ptr<Builder> builder;
    std::vector<std::unique_ptr<Region>> regions;
    std::vector<std::unique_ptr<Operation>> operations;
    std::vector<std::shared_ptr<Value>> values;
};

hscir_builder_t* hscir_builder_create(hscir_context_t* ctx)
{
    auto b = new hscir_builder_t();
    b->builder = std::make_unique<Builder>();
    return b;
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

// ========== 区域和块 ==========
hscir_region_t* hscir_builder_create_region(hscir_builder_t* builder)
{
    auto region = std::make_unique<Region>();
    auto ptr = region.get();
    builder->regions.push_back(std::move(region));
    return reinterpret_cast<hscir_region_t*>(ptr);
}

hscir_block_t* hscir_builder_create_block(hscir_builder_t* builder, hscir_region_t* region, hscir_type_t** arg_types, size_t n_args)
{
    auto reg = reinterpret_cast<Region*>(region);
    auto block = std::make_unique<Block>();
    for (size_t i = 0; i < n_args; ++i)
    {
        auto typePtr = reinterpret_cast<std::shared_ptr<Type>*>(arg_types[i]);
        if (typePtr && *typePtr)
        {
            block->addArgument(*typePtr);
        }
    }
    Block* ptr = block.get();
    reg->addBlock(std::move(block));
    return reinterpret_cast<hscir_block_t*>(ptr);
}

hscir_value_t* hscir_block_get_argument(hscir_block_t* block, size_t index)
{
    auto blk = reinterpret_cast<Block*>(block);
    if (blk && index < blk->getArguments().size())
    {
        return reinterpret_cast<hscir_value_t*>(blk->getArguments()[index].get());
    }
    return nullptr;
}

// ========== 常量创建 ==========
static hscir_value_t* create_and_store_value(hscir_builder_t* builder, std::shared_ptr<Value> val)
{
    builder->values.push_back(val);
    return reinterpret_cast<hscir_value_t*>(val.get());
}

hscir_value_t* hscir_builder_create_constant_i32(hscir_builder_t* builder, int32_t value)
{
    auto type = TypeManager::getInstance().getIntegerType(32, true);
    auto val = builder->builder->createConstant(type, value);
    return create_and_store_value(builder, val);
}

hscir_value_t* hscir_builder_create_constant_i64(hscir_builder_t* builder, int64_t value)
{
    auto type = TypeManager::getInstance().getIntegerType(64, true);
    auto val = builder->builder->createConstant(type, value);
    return create_and_store_value(builder, val);
}

hscir_value_t* hscir_builder_create_constant_f32(hscir_builder_t* builder, float value)
{
    auto type = TypeManager::getInstance().getFloatType(32);
    // 使用 bit_cast 保持精度
    int64_t bits;
    static_assert(sizeof(float) == sizeof(int32_t), "float size mismatch");
    std::memcpy(&bits, &value, sizeof(float));
    auto val = builder->builder->createConstant(type, bits);
    return create_and_store_value(builder, val);
}

hscir_value_t* hscir_builder_create_constant_f64(hscir_builder_t* builder, double value)
{
    auto type = TypeManager::getInstance().getFloatType(64);
    int64_t bits;
    static_assert(sizeof(double) == sizeof(int64_t), "double size mismatch");
    std::memcpy(&bits, &value, sizeof(double));
    auto val = builder->builder->createConstant(type, bits);
    return create_and_store_value(builder, val);
}

hscir_value_t* hscir_builder_create_constant_bool(hscir_builder_t* builder, int value)
{
    auto type = TypeManager::getInstance().getIntegerType(1, false);
    auto val = builder->builder->createConstant(type, value ? 1 : 0);
    return create_and_store_value(builder, val);
}

hscir_value_t* hscir_builder_create_constant(hscir_builder_t* builder, hscir_type_t* ty, int64_t value)
{
    auto typePtr = reinterpret_cast<std::shared_ptr<Type>*>(ty);
    auto val = builder->builder->createConstant(typePtr ? *typePtr : nullptr, value);
    return create_and_store_value(builder, val);
}

// ========== 二元操作 ==========
hscir_value_t* hscir_builder_create_binary_op(hscir_builder_t* builder, const char* op_name, hscir_value_t* lhs, hscir_value_t* rhs)
{
    auto op = std::make_unique<Operation>(op_name);
    auto lhsVal = reinterpret_cast<Value*>(lhs);
    auto rhsVal = reinterpret_cast<Value*>(rhs);

    if (lhsVal)
    {
        auto sharedLhs = std::shared_ptr<Value>(lhsVal, [](Value*){}); // non-owning
        op->addOperation(sharedLhs);
    }
    if (rhsVal)
    {
        auto sharedRhs = std::shared_ptr<Value>(rhsVal, [](Value*){}); // non-owning
        op->addOperation(sharedRhs);
    }

    // 结果类型与操作数相同
    if (lhsVal)
    {
        op->addResultType(lhsVal->getType());
    }

    auto result = std::make_shared<OpResult>(op->getResultTypes().empty() ? nullptr : op->getResultTypes()[0], op.get(), 0);
    builder->operations.push_back(std::move(op));
    return create_and_store_value(builder, result);
}

// ========== 函数/任务操作 ==========
hscir_value_t* hscir_builder_create_func(hscir_builder_t* builder, const char* name, hscir_type_t* func_type, hscir_region_t* body)
{
    auto funcTy = func_type ? *reinterpret_cast<std::shared_ptr<Type>*>(func_type) : nullptr;
    auto region = reinterpret_cast<Region*>(body);

    // 从函数类型提取输入输出
    std::vector<std::shared_ptr<Type>> inputs, outputs;
    if (funcTy && funcTy->getKind() == Type::Kind::Function)
    {
        auto ft = std::dynamic_pointer_cast<FunctionType>(funcTy);
        inputs = ft->getInputs();
        outputs = ft->getOutputs();
    }

    // 创建新区域（转移所有权）
    auto bodyRegion = std::make_unique<Region>();
    if (region)
    {
        // 复制块
        for (const auto& block : region->getBlocks())
        {
            auto newBlock = std::make_unique<Block>();
            // TODO: 深拷贝块内容
            bodyRegion->addBlock(std::move(newBlock));
        }
    }

    auto op = builder->builder->createFuncOp(name, inputs, outputs, std::move(bodyRegion));
    builder->operations.push_back(std::move(op));
    return nullptr; // 函数定义通常不需要返回值
}

hscir_value_t* hscir_builder_create_task(hscir_builder_t* builder, const char* name, hscir_type_t* func_type, hscir_region_t* body)
{
    auto funcTy = func_type ? *reinterpret_cast<std::shared_ptr<Type>*>(func_type) : nullptr;
    auto region = reinterpret_cast<Region*>(body);

    std::vector<std::shared_ptr<Type>> inputs, outputs;
    if (funcTy && funcTy->getKind() == Type::Kind::Function)
    {
        auto ft = std::dynamic_pointer_cast<FunctionType>(funcTy);
        inputs = ft->getInputs();
        outputs = ft->getOutputs();
    }

    auto bodyRegion = std::make_unique<Region>();
    if (region)
    {
        for (const auto& block : region->getBlocks())
        {
            auto newBlock = std::make_unique<Block>();
            bodyRegion->addBlock(std::move(newBlock));
        }
    }

    auto op = builder->builder->createTaskOp(name, inputs, outputs, std::move(bodyRegion));
    builder->operations.push_back(std::move(op));
    return nullptr;
}

// ========== 并行循环 ==========
hscir_value_t* hscir_builder_create_parallel_for(hscir_builder_t* builder, hscir_value_t* lb, hscir_value_t* ub, hscir_value_t* step, hscir_region_t* body)
{
    auto lbVal = std::shared_ptr<Value>(reinterpret_cast<Value*>(lb), [](Value*){});
    auto ubVal = std::shared_ptr<Value>(reinterpret_cast<Value*>(ub), [](Value*){});
    auto stepVal = std::shared_ptr<Value>(reinterpret_cast<Value*>(step), [](Value*){});

    auto region = reinterpret_cast<Region*>(body);
    auto bodyRegion = std::make_unique<Region>();
    if (region)
    {
        for (const auto& block : region->getBlocks())
        {
            auto newBlock = std::make_unique<Block>();
            bodyRegion->addBlock(std::move(newBlock));
        }
    }

    auto op = builder->builder->createParallelForOp(lbVal, ubVal, stepVal, std::move(bodyRegion));
    builder->operations.push_back(std::move(op));
    return nullptr;
}

// ========== 异构操作 ==========
hscir_value_t* hscir_builder_create_spawn(hscir_builder_t* builder, hscir_value_t* task, hscir_value_t** args, size_t n_args, int await)
{
    std::vector<std::shared_ptr<Value>> argVec;
    for (size_t i = 0; i < n_args; ++i)
    {
        argVec.push_back(std::shared_ptr<Value>(reinterpret_cast<Value*>(args[i]), [](Value*){}));
    }

    auto taskVal = std::shared_ptr<Value>(reinterpret_cast<Value*>(task), [](Value*){});
    auto op = builder->builder->createSpawnOp(taskVal, argVec, await != 0);
    builder->operations.push_back(std::move(op));
    return nullptr;
}

hscir_value_t* hscir_builder_create_place_on(hscir_builder_t* builder, hscir_value_t* buffer, hscir_value_t* device)
{
    auto bufVal = std::shared_ptr<Value>(reinterpret_cast<Value*>(buffer), [](Value*){});
    auto devVal = std::shared_ptr<Value>(reinterpret_cast<Value*>(device), [](Value*){});
    auto op = builder->builder->createPlaceOnOp(bufVal, devVal);
    builder->operations.push_back(std::move(op));
    return nullptr;
}

hscir_value_t* hscir_builder_create_move_to(hscir_builder_t* builder, hscir_value_t* buffer, hscir_value_t* device)
{
    // 类似 place_on
    auto bufVal = std::shared_ptr<Value>(reinterpret_cast<Value*>(buffer), [](Value*){});
    auto devVal = std::shared_ptr<Value>(reinterpret_cast<Value*>(device), [](Value*){});
    auto op = std::make_unique<Operation>("hsc.move_to");
    op->addOperation(bufVal);
    op->addOperation(devVal);
    builder->operations.push_back(std::move(op));
    return nullptr;
}

// ========== 控制流 ==========
void hscir_builder_create_return(hscir_builder_t* builder, hscir_value_t* value)
{
    auto op = std::make_unique<Operation>("hsc.return");
    if (value)
    {
        auto val = std::shared_ptr<Value>(reinterpret_cast<Value*>(value), [](Value*){});
        op->addOperation(val);
        op->addResultType(val->getType());
    }
    builder->operations.push_back(std::move(op));
}

hscir_value_t* hscir_builder_create_call(hscir_builder_t* builder, const char* func_name, hscir_value_t** args, size_t n_args)
{
    auto op = std::make_unique<Operation>("hsc.call");
    op->setAttribute("callee", std::make_unique<StringAttr>(func_name));

    for (size_t i = 0; i < n_args; ++i)
    {
        auto val = std::shared_ptr<Value>(reinterpret_cast<Value*>(args[i]), [](Value*){});
        op->addOperation(val);
    }

    builder->operations.push_back(std::move(op));
    return nullptr;
}

// ========== 内存操作 ==========
hscir_value_t* hscir_builder_create_load(hscir_builder_t* builder, hscir_value_t* buffer, hscir_value_t* index)
{
    auto op = std::make_unique<Operation>("hsc.load");
    auto bufVal = std::shared_ptr<Value>(reinterpret_cast<Value*>(buffer), [](Value*){});
    auto idxVal = std::shared_ptr<Value>(reinterpret_cast<Value*>(index), [](Value*){});
    op->addOperation(bufVal);
    op->addOperation(idxVal);

    // TODO: 设置结果类型为元素类型
    builder->operations.push_back(std::move(op));
    return nullptr;
}

void hscir_builder_create_store(hscir_builder_t* builder, hscir_value_t* value, hscir_value_t* buffer, hscir_value_t* index)
{
    auto op = std::make_unique<Operation>("hsc.store");
    auto val = std::shared_ptr<Value>(reinterpret_cast<Value*>(value), [](Value*){});
    auto bufVal = std::shared_ptr<Value>(reinterpret_cast<Value*>(buffer), [](Value*){});
    auto idxVal = std::shared_ptr<Value>(reinterpret_cast<Value*>(index), [](Value*){});
    op->addOperation(val);
    op->addOperation(bufVal);
    op->addOperation(idxVal);
    builder->operations.push_back(std::move(op));
}

hscir_value_t* hscir_builder_create_alloc(hscir_builder_t* builder, hscir_type_t* elem_type, const int64_t* shape, size_t rank)
{
    auto op = std::make_unique<Operation>("hsc.alloc");
    auto elemTy = elem_type ? *reinterpret_cast<std::shared_ptr<Type>*>(elem_type) : nullptr;

    if (elemTy)
    {
        std::vector<int64_t> shapeVec(shape, shape + rank);
        auto bufType = TypeManager::getInstance().getBufferType(elemTy, shapeVec);
        op->addResultType(bufType);
    }

    builder->operations.push_back(std::move(op));
    return nullptr;
}

// ========== 值操作 ==========
hscir_type_t* hscir_value_get_type(hscir_value_t* value)
{
    auto val = reinterpret_cast<Value*>(value);
    if (val)
    {
        return reinterpret_cast<hscir_type_t*>(new std::shared_ptr<Type>(val->getType()));
    }
    return nullptr;
}

// ========== 工具函数 ==========
void hscir_string_free(char* s)
{
    if (s)
    {
        free(s);
    }
}

void hscir_builder_insert_operation(hscir_builder_t* builder, hscir_value_t* op)
{
    // TODO: 实现
}
