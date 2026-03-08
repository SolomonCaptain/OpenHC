#include <catch2/catch_test_macros.hpp>
#include "hscir/Builder.h"
#include "hscir/Verifier.h"

using namespace hscir;

// ============================================================
// Builder 类型创建测试
// ============================================================

TEST_CASE("Builder - 创建整数类型", "[builder][types]")
{
    Builder builder;

    SECTION("I32 类型")
    {
        auto i32 = builder.getI32Type();
        REQUIRE(i32 != nullptr);
        REQUIRE(i32->getWidth() == 32);
        REQUIRE(i32->isSigned() == true);
    }

    SECTION("I64 类型")
    {
        auto i64 = builder.getI64Type();
        REQUIRE(i64 != nullptr);
        REQUIRE(i64->getWidth() == 64);
    }

    SECTION("自定义宽度整数")
    {
        auto i128 = builder.getIntegerType(128, true);
        REQUIRE(i128 != nullptr);
        REQUIRE(i128->getWidth() == 128);
    }
}

TEST_CASE("Builder - 创建浮点类型", "[builder][types]")
{
    Builder builder;

    SECTION("F32 类型")
    {
        auto f32 = builder.getF32Type();
        REQUIRE(f32 != nullptr);
        REQUIRE(f32->getWidth() == 32);
    }

    SECTION("F64 类型")
    {
        auto f64 = builder.getF64Type();
        REQUIRE(f64 != nullptr);
        REQUIRE(f64->getWidth() == 64);
    }
}

TEST_CASE("Builder - 创建 Buffer 类型", "[builder][types]")
{
    Builder builder;

    SECTION("一维 Buffer")
    {
        auto f32 = builder.getF32Type();
        auto bufType = builder.getBufferType(f32, {100});
        REQUIRE(bufType != nullptr);
        REQUIRE(bufType->getRank() == 1);
    }

    SECTION("多维 Buffer")
    {
        auto f32 = builder.getF32Type();
        auto bufType = builder.getBufferType(f32, {64, 64, 3});
        REQUIRE(bufType != nullptr);
        REQUIRE(bufType->getRank() == 3);
    }
}

TEST_CASE("Builder - 创建函数类型", "[builder][types]")
{
    Builder builder;

    SECTION("简单函数类型")
    {
        auto i32 = builder.getI32Type();
        auto funcType = builder.getFunctionType({i32}, {i32});
        REQUIRE(funcType != nullptr);
        REQUIRE(funcType->getInputs().size() == 1);
        REQUIRE(funcType->getOutputs().size() == 1);
    }
}

// ============================================================
// Builder 常量创建测试
// ============================================================

TEST_CASE("Builder - 创建常量", "[builder][constant]")
{
    Builder builder;

    SECTION("I32 常量")
    {
        auto val = builder.createI32Constant(42);
        REQUIRE(val != nullptr);
    }

    SECTION("I64 常量")
    {
        auto val = builder.createI64Constant(12345678901234LL);
        REQUIRE(val != nullptr);
    }

    SECTION("F32 常量")
    {
        auto val = builder.createF32Constant(3.14f);
        REQUIRE(val != nullptr);
    }

    SECTION("F64 常量")
    {
        auto val = builder.createF64Constant(3.14159265358979);
        REQUIRE(val != nullptr);
    }

    SECTION("Bool 常量")
    {
        auto val = builder.createBoolConstant(true);
        REQUIRE(val != nullptr);
    }
}

// ============================================================
// Builder 算术操作测试
// ============================================================

TEST_CASE("Builder - 创建算术操作", "[builder][arithmetic]")
{
    Builder builder;

    auto lhs = builder.createI32Constant(10);
    auto rhs = builder.createI32Constant(20);

    SECTION("AddOp")
    {
        auto result = builder.createAddOp(lhs, rhs);
        REQUIRE(result != nullptr);
    }

    SECTION("SubOp")
    {
        auto result = builder.createSubOp(lhs, rhs);
        REQUIRE(result != nullptr);
    }

    SECTION("MulOp")
    {
        auto result = builder.createMulOp(lhs, rhs);
        REQUIRE(result != nullptr);
    }

    SECTION("DivOp")
    {
        auto result = builder.createDivOp(lhs, rhs);
        REQUIRE(result != nullptr);
    }

    SECTION("ModOp")
    {
        auto result = builder.createModOp(lhs, rhs);
        REQUIRE(result != nullptr);
    }
}

TEST_CASE("Builder - 创建比较操作", "[builder][compare]")
{
    Builder builder;

    auto lhs = builder.createI32Constant(10);
    auto rhs = builder.createI32Constant(20);

    SECTION("EQ")
    {
        auto result = builder.createCmpOp(CmpOp::Predicate::EQ, lhs, rhs);
        REQUIRE(result != nullptr);
    }

    SECTION("LT")
    {
        auto result = builder.createCmpOp(CmpOp::Predicate::LT, lhs, rhs);
        REQUIRE(result != nullptr);
    }

    SECTION("GT")
    {
        auto result = builder.createCmpOp(CmpOp::Predicate::GT, lhs, rhs);
        REQUIRE(result != nullptr);
    }
}

// ============================================================
// Builder 控制流测试
// ============================================================

TEST_CASE("Builder - 创建控制流操作", "[builder][controlflow]")
{
    Builder builder;

    SECTION("创建 Region 和 Block")
    {
        auto region = builder.createRegion();
        REQUIRE(region != nullptr);
        REQUIRE(region->empty());

        auto block = builder.createBlock(region, {});
        REQUIRE(block != nullptr);
        REQUIRE(!region->empty());
    }

    SECTION("设置插入点")
    {
        auto region = builder.createRegion();
        auto block = builder.createBlock(region, {});
        
        builder.setInsertionPoint(block);
        REQUIRE(builder.getInsertionBlock() == block);
    }
}

// ============================================================
// Builder 函数创建测试
// ============================================================

TEST_CASE("Builder - 创建函数", "[builder][func]")
{
    Builder builder;

    SECTION("创建空函数体")
    {
        auto i32 = builder.getI32Type();
        auto funcType = builder.getFunctionType({i32}, {i32});
        
        auto func = builder.createFuncOp("add_one", funcType);
        REQUIRE(func != nullptr);
        REQUIRE(func->getFunctionName() == "add_one");
    }

    SECTION("创建带参数的函数")
    {
        auto i32 = builder.getI32Type();
        auto f32 = builder.getF32Type();
        auto funcType = builder.getFunctionType({i32, f32}, {f32});
        
        auto func = builder.createFuncOp("mixed_func", funcType);
        REQUIRE(func != nullptr);
    }
}

TEST_CASE("Builder - 创建任务", "[builder][task]")
{
    Builder builder;

    SECTION("创建简单任务")
    {
        auto f32 = builder.getF32Type();
        auto bufType = builder.getBufferType(f32, {100});
        auto funcType = builder.getFunctionType({bufType}, {bufType});
        
        auto task = builder.createTaskOp("vector_add", funcType);
        REQUIRE(task != nullptr);
        REQUIRE(task->getTaskName() == "vector_add");
    }
}

// ============================================================
// Builder 完整示例测试
// ============================================================

TEST_CASE("Builder - 构建完整函数", "[builder][integration]")
{
    Builder builder;

    // 创建函数类型: (i32, i32) -> i32
    auto i32 = builder.getI32Type();
    auto funcType = builder.getFunctionType({i32, i32}, {i32});

    // 创建函数
    auto func = builder.createFuncOp("add", funcType);
    REQUIRE(func != nullptr);

    // 创建函数体
    auto body = builder.createRegion();
    auto entryBlock = builder.createBlock(body, {i32, i32});

    // 设置插入点
    builder.setInsertionPoint(entryBlock);

    // 获取块参数
    auto& args = entryBlock->getArguments();
    REQUIRE(args.size() == 2);

    // 创建加法操作
    auto lhs = args[0];
    auto rhs = args[1];
    auto sum = builder.createAddOp(lhs, rhs);

    // 创建返回操作
    builder.createReturnOp(sum);

    // 验证构建的函数
    REQUIRE(entryBlock->getOperations().size() == 2);  // add + return
}

TEST_CASE("Builder - 构建简单循环", "[builder][integration]")
{
    Builder builder;

    // 创建循环边界
    auto lb = builder.createI32Constant(0);
    auto ub = builder.createI32Constant(100);
    auto step = builder.createI32Constant(1);

    // 创建循环体区域
    auto body = builder.createRegion();
    auto bodyBlock = builder.createBlock(body, {builder.getI32Type()});  // 循环变量

    // 创建并行循环
    auto parallelFor = builder.createParallelForOp(lb, ub, step, std::move(body));
    REQUIRE(parallelFor != nullptr);
}
