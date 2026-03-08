#include <catch2/catch_test_macros.hpp>
#include "hscir/Types.h"
#include "hscir/Builder.h"

using namespace hscir;

// ============================================================
// IntegerType 测试
// ============================================================

TEST_CASE("IntegerType - 创建和属性", "[types][integer]")
{
    auto typeManager = TypeManager::getInstance();

    SECTION("创建 I32 类型")
    {
        auto i32 = typeManager->getIntegerType(32, true);
        REQUIRE(i32 != nullptr);
        REQUIRE(i32->getWidth() == 32);
        REQUIRE(i32->isSigned() == true);
        REQUIRE(i32->getKind() == Type::Kind::Integer);
    }

    SECTION("创建 I64 类型")
    {
        auto i64 = typeManager->getIntegerType(64, true);
        REQUIRE(i64 != nullptr);
        REQUIRE(i64->getWidth() == 64);
    }

    SECTION("创建无符号整数类型")
    {
        auto u32 = typeManager->getIntegerType(32, false);
        REQUIRE(u32 != nullptr);
        REQUIRE(u32->isSigned() == false);
    }

    SECTION("创建 I1 (布尔) 类型")
    {
        auto i1 = typeManager->getIntegerType(1, false);
        REQUIRE(i1 != nullptr);
        REQUIRE(i1->getWidth() == 1);
    }
}

TEST_CASE("IntegerType - 类型唯一性", "[types][integer]")
{
    auto typeManager = TypeManager::getInstance();

    SECTION("相同类型应该返回相同指针")
    {
        auto i32_a = typeManager->getIntegerType(32, true);
        auto i32_b = typeManager->getIntegerType(32, true);
        REQUIRE(i32_a == i32_b);
    }

    SECTION("不同类型应该返回不同指针")
    {
        auto i32 = typeManager->getIntegerType(32, true);
        auto i64 = typeManager->getIntegerType(64, true);
        REQUIRE(i32 != i64);
    }
}

TEST_CASE("IntegerType - toString", "[types][integer]")
{
    auto typeManager = TypeManager::getInstance();

    SECTION("有符号整数")
    {
        auto i32 = typeManager->getIntegerType(32, true);
        REQUIRE(i32->toString() == "i32");
    }

    SECTION("无符号整数")
    {
        auto u32 = typeManager->getIntegerType(32, false);
        REQUIRE(u32->toString() == "u32");
    }
}

// ============================================================
// FloatType 测试
// ============================================================

TEST_CASE("FloatType - 创建和属性", "[types][float]")
{
    auto typeManager = TypeManager::getInstance();

    SECTION("创建 F32 类型")
    {
        auto f32 = typeManager->getFloatType(32);
        REQUIRE(f32 != nullptr);
        REQUIRE(f32->getWidth() == 32);
        REQUIRE(f32->getKind() == Type::Kind::Float);
    }

    SECTION("创建 F64 类型")
    {
        auto f64 = typeManager->getFloatType(64);
        REQUIRE(f64 != nullptr);
        REQUIRE(f64->getWidth() == 64);
    }

    SECTION("创建 F16 类型")
    {
        auto f16 = typeManager->getFloatType(16);
        REQUIRE(f16 != nullptr);
        REQUIRE(f16->getWidth() == 16);
    }
}

TEST_CASE("FloatType - 类型唯一性", "[types][float]")
{
    auto typeManager = TypeManager::getInstance();

    SECTION("相同类型应该返回相同指针")
    {
        auto f32_a = typeManager->getFloatType(32);
        auto f32_b = typeManager->getFloatType(32);
        REQUIRE(f32_a == f32_b);
    }

    SECTION("不同类型应该返回不同指针")
    {
        auto f32 = typeManager->getFloatType(32);
        auto f64 = typeManager->getFloatType(64);
        REQUIRE(f32 != f64);
    }
}

TEST_CASE("FloatType - toString", "[types][float]")
{
    auto typeManager = TypeManager::getInstance();

    SECTION("F32")
    {
        auto f32 = typeManager->getFloatType(32);
        REQUIRE(f32->toString() == "f32");
    }

    SECTION("F64")
    {
        auto f64 = typeManager->getFloatType(64);
        REQUIRE(f64->toString() == "f64");
    }
}

// ============================================================
// BufferType 测试
// ============================================================

TEST_CASE("BufferType - 创建和属性", "[types][buffer]")
{
    auto typeManager = TypeManager::getInstance();

    SECTION("创建一维 Buffer")
    {
        auto elemType = typeManager->getFloatType(32);
        auto bufType = typeManager->getBufferType(elemType, {100});
        REQUIRE(bufType != nullptr);
        REQUIRE(bufType->getKind() == Type::Kind::Buffer);
        REQUIRE(bufType->getElementType() == elemType);
        REQUIRE(bufType->getShape() == std::vector<int64_t>{100});
        REQUIRE(bufType->getRank() == 1);
    }

    SECTION("创建二维 Buffer")
    {
        auto elemType = typeManager->getFloatType(32);
        auto bufType = typeManager->getBufferType(elemType, {64, 64});
        REQUIRE(bufType != nullptr);
        REQUIRE(bufType->getRank() == 2);
        REQUIRE(bufType->getShape() == std::vector<int64_t>{64, 64});
    }

    SECTION("创建动态形状 Buffer")
    {
        auto elemType = typeManager->getIntegerType(32, true);
        auto bufType = typeManager->getBufferType(elemType, {-1, 64});
        REQUIRE(bufType != nullptr);
        REQUIRE(bufType->getShape()[0] == -1);  // 动态维度
        REQUIRE(bufType->getShape()[1] == 64);
    }
}

TEST_CASE("BufferType - toString", "[types][buffer]")
{
    auto typeManager = TypeManager::getInstance();

    SECTION("静态形状 Buffer")
    {
        auto elemType = typeManager->getFloatType(32);
        auto bufType = typeManager->getBufferType(elemType, {64, 64});
        REQUIRE(bufType->toString() == "buffer<f32x64x64>");
    }

    SECTION("动态形状 Buffer")
    {
        auto elemType = typeManager->getFloatType(32);
        auto bufType = typeManager->getBufferType(elemType, {-1, 64});
        REQUIRE(bufType->toString() == "buffer<f32x?x64>");
    }
}

// ============================================================
// FunctionType 测试
// ============================================================

TEST_CASE("FunctionType - 创建和属性", "[types][function]")
{
    auto typeManager = TypeManager::getInstance();

    SECTION("创建简单函数类型")
    {
        auto i32 = typeManager->getIntegerType(32, true);
        auto funcType = typeManager->getFunctionType({i32}, {i32});
        REQUIRE(funcType != nullptr);
        REQUIRE(funcType->getKind() == Type::Kind::Function);
        REQUIRE(funcType->getInputs().size() == 1);
        REQUIRE(funcType->getOutputs().size() == 1);
    }

    SECTION("创建多参数函数类型")
    {
        auto i32 = typeManager->getIntegerType(32, true);
        auto f32 = typeManager->getFloatType(32);
        auto funcType = typeManager->getFunctionType({i32, f32}, {f32});
        REQUIRE(funcType != nullptr);
        REQUIRE(funcType->getInputs().size() == 2);
        REQUIRE(funcType->getOutputs().size() == 1);
    }

    SECTION("创建无返回值函数类型")
    {
        auto i32 = typeManager->getIntegerType(32, true);
        auto funcType = typeManager->getFunctionType({i32}, {});
        REQUIRE(funcType != nullptr);
        REQUIRE(funcType->getOutputs().empty());
    }
}

TEST_CASE("FunctionType - toString", "[types][function]")
{
    auto typeManager = TypeManager::getInstance();

    SECTION("简单函数")
    {
        auto i32 = typeManager->getIntegerType(32, true);
        auto funcType = typeManager->getFunctionType({i32}, {i32});
        REQUIRE(funcType->toString() == "(i32) -> i32");
    }

    SECTION("多参数函数")
    {
        auto i32 = typeManager->getIntegerType(32, true);
        auto f32 = typeManager->getFloatType(32);
        auto funcType = typeManager->getFunctionType({i32, f32}, {f32});
        REQUIRE(funcType->toString() == "(i32, f32) -> f32");
    }
}

// ============================================================
// 类型比较测试
// ============================================================

TEST_CASE("Type - 比较操作", "[types][compare]")
{
    auto typeManager = TypeManager::getInstance();

    SECTION("相同类型相等")
    {
        auto i32_a = typeManager->getIntegerType(32, true);
        auto i32_b = typeManager->getIntegerType(32, true);
        REQUIRE(*i32_a == *i32_b);
    }

    SECTION("不同类型不相等")
    {
        auto i32 = typeManager->getIntegerType(32, true);
        auto f32 = typeManager->getFloatType(32);
        REQUIRE(!(*i32 == *f32));
    }

    SECTION("有符号和无符号整数不相等")
    {
        auto i32 = typeManager->getIntegerType(32, true);
        auto u32 = typeManager->getIntegerType(32, false);
        REQUIRE(!(*i32 == *u32));
    }
}
