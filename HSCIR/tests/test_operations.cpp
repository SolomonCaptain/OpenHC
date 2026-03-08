#include <catch2/catch_test_macros.hpp>
#include "hscir/Operations.h"
#include "hscir/Builder.h"

using namespace hscir;

// ============================================================
// Operation 基础测试
// ============================================================

TEST_CASE("Operation - 创建和属性", "[operations][basic]")
{
    auto typeManager = TypeManager::getInstance();

    SECTION("创建基本操作")
    {
        auto op = std::make_unique<Operation>("add");
        REQUIRE(op != nullptr);
        REQUIRE(op->getName() == "add");
    }

    SECTION("操作添加属性")
    {
        auto op = std::make_unique<Operation>("constant");
        op->addAttribute("value", std::make_shared<IntegerAttr>(42));
        
        auto& attrs = op->getAttributes();
        REQUIRE(attrs.find("value") != attrs.end());
    }
}

TEST_CASE("Operation - 操作数管理", "[operations][operands]")
{
    auto typeManager = TypeManager::getInstance();

    SECTION("添加操作数")
    {
        auto op = std::make_unique<Operation>("add");
        auto i32 = typeManager->getIntegerType(32, true);
        
        auto val1 = std::make_shared<OpResult>(i32, 0);
        auto val2 = std::make_shared<OpResult>(i32, 1);
        
        op->addOperand(val1);
        op->addOperand(val2);
        
        REQUIRE(op->getOperands().size() == 2);
    }
}

TEST_CASE("Operation - 结果类型管理", "[operations][results]")
{
    auto typeManager = TypeManager::getInstance();

    SECTION("添加结果类型")
    {
        auto op = std::make_unique<Operation>("add");
        auto i32 = typeManager->getIntegerType(32, true);
        
        op->addResultType(i32);
        
        REQUIRE(op->getResultTypes().size() == 1);
    }
}

// ============================================================
// 常量操作测试
// ============================================================

TEST_CASE("ConstantOp - 创建常量", "[operations][constant]")
{
    auto typeManager = TypeManager::getInstance();

    SECTION("整数常量")
    {
        auto i32 = typeManager->getIntegerType(32, true);
        auto op = std::make_unique<ConstantOp>(i32, 42);
        
        REQUIRE(op->getName() == "constant");
        REQUIRE(op->getResultTypes().size() == 1);
        REQUIRE(op->getResultTypes()[0] == i32);
    }

    SECTION("浮点常量")
    {
        auto f32 = typeManager->getFloatType(32);
        auto op = std::make_unique<ConstantOp>(f32, 3.14f);
        
        REQUIRE(op->getName() == "constant");
        REQUIRE(op->getResultTypes().size() == 1);
        REQUIRE(op->getResultTypes()[0] == f32);
    }
}

// ============================================================
// 算术操作测试
// ============================================================

TEST_CASE("ArithmeticOps - 创建算术操作", "[operations][arithmetic]")
{
    auto typeManager = TypeManager::getInstance();
    auto i32 = typeManager->getIntegerType(32, true);
    
    auto lhs = std::make_shared<OpResult>(i32, 0);
    auto rhs = std::make_shared<OpResult>(i32, 1);

    SECTION("AddOp")
    {
        auto op = std::make_unique<AddOp>(lhs, rhs);
        REQUIRE(op->getName() == "add");
        REQUIRE(op->getOperands().size() == 2);
    }

    SECTION("SubOp")
    {
        auto op = std::make_unique<SubOp>(lhs, rhs);
        REQUIRE(op->getName() == "sub");
    }

    SECTION("MulOp")
    {
        auto op = std::make_unique<MulOp>(lhs, rhs);
        REQUIRE(op->getName() == "mul");
    }

    SECTION("DivOp")
    {
        auto op = std::make_unique<DivOp>(lhs, rhs);
        REQUIRE(op->getName() == "div");
    }

    SECTION("ModOp")
    {
        auto op = std::make_unique<ModOp>(lhs, rhs);
        REQUIRE(op->getName() == "mod");
    }
}

TEST_CASE("CmpOp - 创建比较操作", "[operations][compare]")
{
    auto typeManager = TypeManager::getInstance();
    auto i32 = typeManager->getIntegerType(32, true);
    
    auto lhs = std::make_shared<OpResult>(i32, 0);
    auto rhs = std::make_shared<OpResult>(i32, 1);

    SECTION("相等比较")
    {
        auto op = std::make_unique<CmpOp>(CmpOp::Predicate::EQ, lhs, rhs);
        REQUIRE(op->getName() == "cmp");
        REQUIRE(op->getPredicate() == CmpOp::Predicate::EQ);
    }

    SECTION("小于比较")
    {
        auto op = std::make_unique<CmpOp>(CmpOp::Predicate::LT, lhs, rhs);
        REQUIRE(op->getPredicate() == CmpOp::Predicate::LT);
    }

    SECTION("所有比较谓词")
    {
        std::vector<CmpOp::Predicate> predicates = {
            CmpOp::Predicate::EQ, CmpOp::Predicate::NE,
            CmpOp::Predicate::LT, CmpOp::Predicate::LE,
            CmpOp::Predicate::GT, CmpOp::Predicate::GE
        };
        
        for (auto pred : predicates)
        {
            auto op = std::make_unique<CmpOp>(pred, lhs, rhs);
            REQUIRE(op->getPredicate() == pred);
        }
    }
}

// ============================================================
// 内存操作测试
// ============================================================

TEST_CASE("MemoryOps - 创建内存操作", "[operations][memory]")
{
    auto typeManager = TypeManager::getInstance();
    auto f32 = typeManager->getFloatType(32);
    auto bufType = typeManager->getBufferType(f32, {100});

    SECTION("AllocOp")
    {
        auto op = std::make_unique<AllocOp>(f32, std::vector<std::shared_ptr<Value>>{});
        REQUIRE(op->getName() == "alloc");
    }

    SECTION("LoadOp")
    {
        auto buffer = std::make_shared<OpResult>(bufType, 0);
        auto idx = std::make_shared<OpResult>(typeManager->getIntegerType(32, true), 1);
        
        auto op = std::make_unique<LoadOp>(buffer, std::vector<std::shared_ptr<Value>>{idx});
        REQUIRE(op->getName() == "load");
        REQUIRE(op->getOperands().size() == 2);  // buffer + index
    }

    SECTION("StoreOp")
    {
        auto buffer = std::make_shared<OpResult>(bufType, 0);
        auto value = std::make_shared<OpResult>(f32, 1);
        auto idx = std::make_shared<OpResult>(typeManager->getIntegerType(32, true), 2);
        
        auto op = std::make_unique<StoreOp>(value, buffer, std::vector<std::shared_ptr<Value>>{idx});
        REQUIRE(op->getName() == "store");
    }
}

// ============================================================
// 控制流操作测试
// ============================================================

TEST_CASE("ControlFlowOps - 创建控制流操作", "[operations][controlflow]")
{
    auto typeManager = TypeManager::getInstance();

    SECTION("BranchOp")
    {
        auto block = std::make_unique<Block>();
        auto op = std::make_unique<BranchOp>(block.get());
        REQUIRE(op->getName() == "br");
    }

    SECTION("CondBranchOp")
    {
        auto i1 = typeManager->getIntegerType(1, false);
        auto cond = std::make_shared<OpResult>(i1, 0);
        
        auto trueBlock = std::make_unique<Block>();
        auto falseBlock = std::make_unique<Block>();
        
        auto op = std::make_unique<CondBranchOp>(cond, trueBlock.get(), falseBlock.get());
        REQUIRE(op->getName() == "cond_br");
    }

    SECTION("ReturnOp with value")
    {
        auto i32 = typeManager->getIntegerType(32, true);
        auto value = std::make_shared<OpResult>(i32, 0);
        
        auto op = std::make_unique<ReturnOp>(value);
        REQUIRE(op->getName() == "return");
        REQUIRE(op->getOperands().size() == 1);
    }

    SECTION("ReturnOp void")
    {
        auto op = std::make_unique<ReturnOp>(nullptr);
        REQUIRE(op->getName() == "return");
        REQUIRE(op->getOperands().empty());
    }
}

// ============================================================
// 并行操作测试
// ============================================================

TEST_CASE("ParallelOps - 创建并行操作", "[operations][parallel]")
{
    auto typeManager = TypeManager::getInstance();
    auto i32 = typeManager->getIntegerType(32, true);

    SECTION("ParallelForOp")
    {
        auto lb = std::make_shared<OpResult>(i32, 0);
        auto ub = std::make_shared<OpResult>(i32, 1);
        auto step = std::make_shared<OpResult>(i32, 2);
        
        auto body = std::make_unique<Region>();
        
        auto op = std::make_unique<ParallelForOp>(lb, ub, step, std::move(body));
        REQUIRE(op->getName() == "parallel_for");
    }

    SECTION("ReduceOp")
    {
        auto f32 = typeManager->getFloatType(32);
        auto input = std::make_shared<OpResult>(f32, 0);
        auto init = std::make_shared<OpResult>(f32, 1);
        
        auto op = std::make_unique<ReduceOp>(ReduceOp::ReductionKind::SUM, input, init, std::vector<int64_t>{0});
        REQUIRE(op->getName() == "reduce");
        REQUIRE(op->getReductionKind() == ReduceOp::ReductionKind::SUM);
    }
}

// ============================================================
// 设备操作测试
// ============================================================

TEST_CASE("DeviceOps - 创建设备操作", "[operations][device]")
{
    auto typeManager = TypeManager::getInstance();
    auto i32 = typeManager->getIntegerType(32, true);

    SECTION("SpawnOp")
    {
        auto device = std::make_shared<OpResult>(i32, 0);
        auto op = std::make_unique<SpawnOp>(device, "my_task", std::vector<std::shared_ptr<Value>>{}, false);
        REQUIRE(op->getName() == "spawn");
    }

    SECTION("SyncOp")
    {
        auto device = std::make_shared<OpResult>(i32, 0);
        auto op = std::make_unique<SyncOp>(device);
        REQUIRE(op->getName() == "sync");
    }

    SECTION("MoveToOp")
    {
        auto f32 = typeManager->getFloatType(32);
        auto bufType = typeManager->getBufferType(f32, {100});
        auto buffer = std::make_shared<OpResult>(bufType, 0);
        auto device = std::make_shared<OpResult>(i32, 1);
        
        auto op = std::make_unique<MoveToOp>(buffer, device);
        REQUIRE(op->getName() == "move_to");
    }

    SECTION("PlaceOnOp")
    {
        auto f32 = typeManager->getFloatType(32);
        auto bufType = typeManager->getBufferType(f32, {100});
        auto buffer = std::make_shared<OpResult>(bufType, 0);
        auto device = std::make_shared<OpResult>(i32, 1);
        
        auto op = std::make_unique<PlaceOnOp>(buffer, device);
        REQUIRE(op->getName() == "place_on");
    }
}

// ============================================================
// 函数和任务操作测试
// ============================================================

TEST_CASE("FuncOp - 创建函数", "[operations][func]")
{
    auto typeManager = TypeManager::getInstance();
    auto i32 = typeManager->getIntegerType(32, true);
    auto funcType = typeManager->getFunctionType({i32}, {i32});

    SECTION("创建空函数体")
    {
        auto body = std::make_unique<Region>();
        auto op = std::make_unique<FuncOp>("my_func", funcType, std::move(body));
        
        REQUIRE(op->getName() == "func");
        REQUIRE(op->getFunctionName() == "my_func");
    }
}

TEST_CASE("TaskOp - 创建任务", "[operations][task]")
{
    auto typeManager = TypeManager::getInstance();
    auto i32 = typeManager->getIntegerType(32, true);
    auto funcType = typeManager->getFunctionType({i32}, {i32});

    SECTION("创建任务")
    {
        auto body = std::make_unique<Region>();
        auto op = std::make_unique<TaskOp>("my_task", funcType, std::move(body));
        
        REQUIRE(op->getName() == "task");
        REQUIRE(op->getTaskName() == "my_task");
    }
}

// ============================================================
// Block 和 Region 测试
// ============================================================

TEST_CASE("Block - 基本块操作", "[operations][block]")
{
    SECTION("创建空块")
    {
        auto block = std::make_unique<Block>();
        REQUIRE(block->getOperations().empty());
        REQUIRE(block->getArguments().empty());
    }

    SECTION("添加操作到块")
    {
        auto block = std::make_unique<Block>();
        auto op = std::make_unique<Operation>("add");
        
        block->addOperation(std::move(op));
        REQUIRE(block->getOperations().size() == 1);
    }

    SECTION("添加参数到块")
    {
        auto typeManager = TypeManager::getInstance();
        auto i32 = typeManager->getIntegerType(32, true);
        
        auto block = std::make_unique<Block>();
        block->addArgument(i32);
        
        REQUIRE(block->getArguments().size() == 1);
    }
}

TEST_CASE("Region - 区域操作", "[operations][region]")
{
    SECTION("创建空区域")
    {
        auto region = std::make_unique<Region>();
        REQUIRE(region->empty());
        REQUIRE(region->size() == 0);
    }

    SECTION("添加块到区域")
    {
        auto region = std::make_unique<Region>();
        auto block = std::make_unique<Block>();
        
        region->addBlock(std::move(block));
        REQUIRE(region->size() == 1);
        REQUIRE(!region->empty());
    }
}

// ============================================================
// Module 测试
// ============================================================

TEST_CASE("Module - 模块操作", "[operations][module]")
{
    SECTION("创建空模块")
    {
        auto module = std::make_unique<Module>();
        REQUIRE(module->getOperations().empty());
    }

    SECTION("添加操作到模块")
    {
        auto module = std::make_unique<Module>();
        auto op = std::make_unique<Operation>("func");
        
        module->addOperation(std::move(op));
        REQUIRE(module->getOperations().size() == 1);
    }
}

// ============================================================
// 属性测试
// ============================================================

TEST_CASE("Attributes - 属性类型", "[operations][attributes]")
{
    SECTION("StringAttr")
    {
        auto attr = std::make_shared<StringAttr>("hello");
        REQUIRE(attr->getValue() == "hello");
    }

    SECTION("IntegerAttr")
    {
        auto attr = std::make_shared<IntegerAttr>(42);
        REQUIRE(attr->getValue() == 42);
    }

    SECTION("FloatAttr")
    {
        auto attr = std::make_shared<FloatAttr>(3.14);
        REQUIRE(attr->getValue() == 3.14);
    }

    SECTION("BoolAttr")
    {
        auto attr = std::make_shared<BoolAttr>(true);
        REQUIRE(attr->getValue() == true);
    }

    SECTION("ArrayAttr")
    {
        std::vector<int64_t> values = {1, 2, 3, 4, 5};
        auto attr = std::make_shared<ArrayAttr>(values);
        REQUIRE(attr->getValues() == values);
    }
}
