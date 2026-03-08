#include <catch2/catch_test_macros.hpp>
#include "hscir/Builder.h"
#include "hscir/Verifier.h"

using namespace hscir;

// ============================================================
// 辅助验证函数测试
// ============================================================

TEST_CASE("isValidType - 类型有效性检查", "[verifier][helper]")
{
    auto typeManager = TypeManager::getInstance();

    SECTION("有效整数类型")
    {
        auto i32 = typeManager->getIntegerType(32, true);
        REQUIRE(isValidType(i32) == true);
    }

    SECTION("有效浮点类型")
    {
        auto f32 = typeManager->getFloatType(32);
        REQUIRE(isValidType(f32) == true);
    }

    SECTION("无效类型 - 空指针")
    {
        REQUIRE(isValidType(nullptr) == false);
    }
}

TEST_CASE("areTypesCompatible - 类型兼容性检查", "[verifier][helper]")
{
    auto typeManager = TypeManager::getInstance();

    SECTION("相同类型兼容")
    {
        auto i32_a = typeManager->getIntegerType(32, true);
        auto i32_b = typeManager->getIntegerType(32, true);
        REQUIRE(areTypesCompatible(i32_a, i32_b) == true);
    }

    SECTION("不同宽度整数不兼容")
    {
        auto i32 = typeManager->getIntegerType(32, true);
        auto i64 = typeManager->getIntegerType(64, true);
        REQUIRE(areTypesCompatible(i32, i64) == false);
    }

    SECTION("有符号和无符号不兼容")
    {
        auto i32 = typeManager->getIntegerType(32, true);
        auto u32 = typeManager->getIntegerType(32, false);
        REQUIRE(areTypesCompatible(i32, u32) == false);
    }

    SECTION("整数和浮点不兼容")
    {
        auto i32 = typeManager->getIntegerType(32, true);
        auto f32 = typeManager->getFloatType(32);
        REQUIRE(areTypesCompatible(i32, f32) == false);
    }
}

TEST_CASE("isValidBufferShape - Buffer 形状有效性检查", "[verifier][helper]")
{
    SECTION("有效静态形状")
    {
        REQUIRE(isValidBufferShape({100}) == true);
        REQUIRE(isValidBufferShape({64, 64}) == true);
        REQUIRE(isValidBufferShape({1, 2, 3, 4}) == true);
    }

    SECTION("有效动态形状")
    {
        REQUIRE(isValidBufferShape({-1}) == true);  // -1 表示动态维度
        REQUIRE(isValidBufferShape({-1, 64}) == true);
    }

    SECTION("无效形状 - 负数（除 -1 外）")
    {
        REQUIRE(isValidBufferShape({-2}) == false);
        REQUIRE(isValidBufferShape({-10, 64}) == false);
    }
}

// ============================================================
// VerificationResult 测试
// ============================================================

TEST_CASE("VerificationResult - 添加诊断信息", "[verifier][result]")
{
    VerificationResult result;

    SECTION("添加错误")
    {
        result.addError("Test error", "test_op", 10);
        REQUIRE_FALSE(result.success());
        REQUIRE(result.getErrors().size() == 1);
    }

    SECTION("添加警告")
    {
        result.addWarning("Test warning", "test_op");
        REQUIRE(result.success());  // 警告不影响成功
        REQUIRE(result.hasWarnings());
    }

    SECTION("添加提示")
    {
        result.addNote("Test note");
        REQUIRE(result.success());
    }

    SECTION("合并结果")
    {
        VerificationResult other;
        other.addError("Other error");
        result.addWarning("Warning");
        
        result.merge(other);
        
        REQUIRE_FALSE(result.success());
        REQUIRE(result.getErrors().size() == 1);
        REQUIRE(result.getWarnings().size() == 1);
    }
}

// ============================================================
// TypeVerifier 测试
// ============================================================

TEST_CASE("TypeVerifier - 验证操作类型", "[verifier][type]")
{
    TypeVerifier verifier;
    Builder builder;

    SECTION("验证常量操作")
    {
        auto val = builder.createI32Constant(42);
        // 注意：这里需要从 Value 获取定义它的 Operation
        // 简化测试，直接创建操作
        auto op = std::make_unique<ConstantOp>(builder.getI32Type(), 42);
        auto result = verifier.verify(op.get());
        REQUIRE(result.success());
    }

    SECTION("验证空操作")
    {
        auto result = verifier.verify(nullptr);
        REQUIRE_FALSE(result.success());
    }
}

// ============================================================
// OperationVerifier 测试
// ============================================================

TEST_CASE("OperationVerifier - 验证操作", "[verifier][operation]")
{
    OperationVerifier verifier;

    SECTION("验证空操作")
    {
        auto result = verifier.verify(nullptr);
        REQUIRE_FALSE(result.success());
    }

    SECTION("验证有效操作")
    {
        auto op = std::make_unique<Operation>("add");
        auto result = verifier.verify(op.get());
        // 由于没有设置操作数，可能会有警告
    }
}

// ============================================================
// ComprehensiveVerifier 测试
// ============================================================

TEST_CASE("ComprehensiveVerifier - 综合验证", "[verifier][comprehensive]")
{
    ComprehensiveVerifier verifier;
    Builder builder;

    SECTION("验证简单函数")
    {
        auto i32 = builder.getI32Type();
        auto funcType = builder.getFunctionType({i32, i32}, {i32});
        
        auto body = builder.createRegion();
        auto entryBlock = builder.createBlock(body, {i32, i32});
        
        builder.setInsertionPoint(entryBlock);
        auto& args = entryBlock->getArguments();
        auto sum = builder.createAddOp(args[0], args[1]);
        builder.createReturnOp(sum);
        
        auto func = builder.createFuncOp("add", funcType, std::move(body));
        
        auto result = verifier.verify(func.get());
        REQUIRE(result.success());
    }
}

// ============================================================
// ModuleVerifier 测试
// ============================================================

TEST_CASE("ModuleVerifier - 验证模块", "[verifier][module]")
{
    Builder builder;

    SECTION("验证空模块")
    {
        auto module = std::make_unique<Module>();
        auto result = ModuleVerifier::verify(module.get());
        REQUIRE(result.success());
    }

    SECTION("验证空指针")
    {
        auto result = ModuleVerifier::verify(nullptr);
        REQUIRE_FALSE(result.success());
    }

    SECTION("验证带函数的模块")
    {
        auto module = std::make_unique<Module>();
        
        auto i32 = builder.getI32Type();
        auto funcType = builder.getFunctionType({i32}, {i32});
        
        auto body = builder.createRegion();
        auto entryBlock = builder.createBlock(body, {i32});
        
        builder.setInsertionPoint(entryBlock);
        builder.createReturnOp(entryBlock->getArguments()[0]);
        
        auto func = builder.createFuncOp("identity", funcType, std::move(body));
        module->addOperation(std::move(func));
        
        auto result = ModuleVerifier::verify(module.get());
        REQUIRE(result.success());
    }

    SECTION("验证符号重复")
    {
        auto module = std::make_unique<Module>();
        
        auto i32 = builder.getI32Type();
        auto funcType = builder.getFunctionType({}, {});
        
        auto body1 = builder.createRegion();
        builder.createBlock(body1, {});
        auto func1 = builder.createFuncOp("duplicate", funcType, std::move(body1));
        module->addOperation(std::move(func1));
        
        auto body2 = builder.createRegion();
        builder.createBlock(body2, {});
        auto func2 = builder.createFuncOp("duplicate", funcType, std::move(body2));
        module->addOperation(std::move(func2));
        
        auto result = ModuleVerifier::verify(module.get());
        REQUIRE_FALSE(result.success());  // 应该检测到重复符号
    }
}

// ============================================================
// 错误诊断测试
// ============================================================

TEST_CASE("VerificationDiagnostic - 诊断信息格式", "[verifier][diagnostic]")
{
    SECTION("错误信息")
    {
        VerificationDiagnostic diag(VerificationLevel::Error, "Test error", "test_op", 10);
        std::string str = diag.toString();
        REQUIRE(str.find("error:") != std::string::npos);
        REQUIRE(str.find("Test error") != std::string::npos);
        REQUIRE(str.find("test_op") != std::string::npos);
        REQUIRE(str.find("line 10") != std::string::npos);
    }

    SECTION("警告信息")
    {
        VerificationDiagnostic diag(VerificationLevel::Warning, "Test warning", "test_op");
        std::string str = diag.toString();
        REQUIRE(str.find("warning:") != std::string::npos);
    }

    SECTION("提示信息")
    {
        VerificationDiagnostic diag(VerificationLevel::Note, "Test note");
        std::string str = diag.toString();
        REQUIRE(str.find("note:") != std::string::npos);
    }
}

// ============================================================
// 控制流验证测试
// ============================================================

TEST_CASE("ControlFlowVerifier - 验证控制流", "[verifier][controlflow]")
{
    ControlFlowVerifier verifier;
    Builder builder;

    SECTION("验证缺少终结符的块")
    {
        auto region = builder.createRegion();
        auto block = builder.createBlock(region, {});
        
        builder.setInsertionPoint(block);
        auto val = builder.createI32Constant(42);
        // 不添加 return，块没有终结符
        
        // 验证应该失败或产生警告
        auto result = verifier.verify(nullptr);  // 需要适当的操作来测试
    }
}

// ============================================================
// 并行操作验证测试
// ============================================================

TEST_CASE("ParallelVerifier - 验证并行操作", "[verifier][parallel]")
{
    ParallelVerifier verifier;

    SECTION("验证空操作")
    {
        auto result = verifier.verify(nullptr);
        REQUIRE_FALSE(result.success());
    }
}

// ============================================================
// 设备操作验证测试
// ============================================================

TEST_CASE("DeviceVerifier - 验证设备操作", "[verifier][device]")
{
    DeviceVerifier verifier;

    SECTION("验证空操作")
    {
        auto result = verifier.verify(nullptr);
        REQUIRE_FALSE(result.success());
    }
}
