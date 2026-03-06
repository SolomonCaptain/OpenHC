#include "hscir/Builder.h"

namespace hscir
{
    // ============================================================
    // 插入点管理
    // ============================================================

    void Builder::setInsertionPoint(Block* block)
    {
        currentBlock_ = block;
    }

    void Builder::setInsertionPointToStart(Block* block)
    {
        currentBlock_ = block;
        // TODO: 支持插入到块开头
    }

    void Builder::setInsertionPointToEnd(Block* block)
    {
        currentBlock_ = block;
    }

    // ============================================================
    // 类型创建
    // ============================================================

    std::shared_ptr<IntegerType> Builder::getI1Type()
    {
        return TypeManager::getInstance().getIntegerType(1, false);
    }

    std::shared_ptr<IntegerType> Builder::getI8Type()
    {
        return TypeManager::getInstance().getIntegerType(8, true);
    }

    std::shared_ptr<IntegerType> Builder::getI16Type()
    {
        return TypeManager::getInstance().getIntegerType(16, true);
    }

    std::shared_ptr<IntegerType> Builder::getI32Type()
    {
        return TypeManager::getInstance().getIntegerType(32, true);
    }

    std::shared_ptr<IntegerType> Builder::getI64Type()
    {
        return TypeManager::getInstance().getIntegerType(64, true);
    }

    std::shared_ptr<IntegerType> Builder::getI128Type()
    {
        return TypeManager::getInstance().getIntegerType(128, true);
    }

    std::shared_ptr<IntegerType> Builder::getIntegerType(unsigned width, bool isSigned)
    {
        return TypeManager::getInstance().getIntegerType(width, isSigned);
    }

    std::shared_ptr<FloatType> Builder::getF16Type()
    {
        return TypeManager::getInstance().getFloatType(16);
    }

    std::shared_ptr<FloatType> Builder::getF32Type()
    {
        return TypeManager::getInstance().getFloatType(32);
    }

    std::shared_ptr<FloatType> Builder::getF64Type()
    {
        return TypeManager::getInstance().getFloatType(64);
    }

    std::shared_ptr<FloatType> Builder::getFloatType(unsigned width)
    {
        return TypeManager::getInstance().getFloatType(width);
    }

    std::shared_ptr<BufferType> Builder::getBufferType(std::shared_ptr<Type> elemType, const std::vector<int64_t>& shape)
    {
        return TypeManager::getInstance().getBufferType(std::move(elemType), shape);
    }

    std::shared_ptr<FunctionType> Builder::getFunctionType(const std::vector<std::shared_ptr<Type>>& inputs, const std::vector<std::shared_ptr<Type>>& outputs)
    {
        return TypeManager::getInstance().getFunctionType(inputs, outputs);
    }

    // ============================================================
    // 常量创建
    // ============================================================

    std::shared_ptr<Value> Builder::createConstant(std::shared_ptr<Type> type, int64_t value)
    {
        auto op = std::make_unique<ConstantOp>(type, value);
        auto result = op->getResult(0);
        if (currentBlock_)
        {
            currentBlock_->addOperation(std::move(op));
        }
        return result;
    }

    std::shared_ptr<Value> Builder::createConstant(std::shared_ptr<Type> type, double value)
    {
        auto op = std::make_unique<ConstantOp>(type, value);
        auto result = op->getResult(0);
        if (currentBlock_)
        {
            currentBlock_->addOperation(std::move(op));
        }
        return result;
    }

    std::shared_ptr<Value> Builder::createI32Constant(int32_t value)
    {
        return createConstant(getI32Type(), static_cast<int64_t>(value));
    }

    std::shared_ptr<Value> Builder::createI64Constant(int64_t value)
    {
        return createConstant(getI64Type(), value);
    }

    std::shared_ptr<Value> Builder::createF32Constant(float value)
    {
        return createConstant(getF32Type(), static_cast<double>(value));
    }

    std::shared_ptr<Value> Builder::createF64Constant(double value)
    {
        return createConstant(getF64Type(), value);
    }

    std::shared_ptr<Value> Builder::createBoolConstant(bool value)
    {
        return createConstant(getI1Type(), static_cast<int64_t>(value ? 1 : 0));
    }

    // ============================================================
    // 算术操作创建
    // ============================================================

    std::shared_ptr<Value> Builder::createAddOp(std::shared_ptr<Value> lhs, std::shared_ptr<Value> rhs)
    {
        auto op = std::make_unique<AddOp>(lhs, rhs);
        auto result = op->getResult(0);
        if (currentBlock_)
        {
            currentBlock_->addOperation(std::move(op));
        }
        return result;
    }

    std::shared_ptr<Value> Builder::createSubOp(std::shared_ptr<Value> lhs, std::shared_ptr<Value> rhs)
    {
        auto op = std::make_unique<SubOp>(lhs, rhs);
        auto result = op->getResult(0);
        if (currentBlock_)
        {
            currentBlock_->addOperation(std::move(op));
        }
        return result;
    }

    std::shared_ptr<Value> Builder::createMulOp(std::shared_ptr<Value> lhs, std::shared_ptr<Value> rhs)
    {
        auto op = std::make_unique<MulOp>(lhs, rhs);
        auto result = op->getResult(0);
        if (currentBlock_)
        {
            currentBlock_->addOperation(std::move(op));
        }
        return result;
    }

    std::shared_ptr<Value> Builder::createDivOp(std::shared_ptr<Value> lhs, std::shared_ptr<Value> rhs)
    {
        auto op = std::make_unique<DivOp>(lhs, rhs);
        auto result = op->getResult(0);
        if (currentBlock_)
        {
            currentBlock_->addOperation(std::move(op));
        }
        return result;
    }

    std::shared_ptr<Value> Builder::createModOp(std::shared_ptr<Value> lhs, std::shared_ptr<Value> rhs)
    {
        auto op = std::make_unique<ModOp>(lhs, rhs);
        auto result = op->getResult(0);
        if (currentBlock_)
        {
            currentBlock_->addOperation(std::move(op));
        }
        return result;
    }

    std::shared_ptr<Value> Builder::createCmpOp(CmpOp::Predicate pred, std::shared_ptr<Value> lhs, std::shared_ptr<Value> rhs)
    {
        auto op = std::make_unique<CmpOp>(pred, lhs, rhs);
        auto result = op->getResult(0);
        if (currentBlock_)
        {
            currentBlock_->addOperation(std::move(op));
        }
        return result;
    }

    // ============================================================
    // 内存操作创建
    // ============================================================

    std::shared_ptr<Value> Builder::createAllocOp(std::shared_ptr<Type> elementType, const std::vector<std::shared_ptr<Value>>& dims)
    {
        auto op = std::make_unique<AllocOp>(elementType, dims);
        auto result = op->getResult(0);
        if (currentBlock_)
        {
            currentBlock_->addOperation(std::move(op));
        }
        return result;
    }

    std::shared_ptr<Value> Builder::createLoadOp(std::shared_ptr<Value> buffer, const std::vector<std::shared_ptr<Value>>& indices)
    {
        auto op = std::make_unique<LoadOp>(buffer, indices);
        auto result = op->getResult(0);
        if (currentBlock_)
        {
            currentBlock_->addOperation(std::move(op));
        }
        return result;
    }

    void Builder::createStoreOp(std::shared_ptr<Value> value, std::shared_ptr<Value> buffer, const std::vector<std::shared_ptr<Value>>& indices)
    {
        auto op = std::make_unique<StoreOp>(value, buffer, indices);
        if (currentBlock_)
        {
            currentBlock_->addOperation(std::move(op));
        }
    }

    // ============================================================
    // 控制流操作创建
    // ============================================================

    void Builder::createBranchOp(Block* target, const std::vector<std::shared_ptr<Value>>& args)
    {
        auto op = std::make_unique<BranchOp>(target, args);
        if (currentBlock_)
        {
            currentBlock_->addOperation(std::move(op));
        }
    }

    void Builder::createCondBranchOp(std::shared_ptr<Value> condition, Block* trueBlock, Block* falseBlock,
                                      const std::vector<std::shared_ptr<Value>>& trueArgs,
                                      const std::vector<std::shared_ptr<Value>>& falseArgs)
    {
        auto op = std::make_unique<CondBranchOp>(condition, trueBlock, falseBlock, trueArgs, falseArgs);
        if (currentBlock_)
        {
            currentBlock_->addOperation(std::move(op));
        }
    }

    void Builder::createReturnOp(std::shared_ptr<Value> value)
    {
        auto op = std::make_unique<ReturnOp>(value);
        if (currentBlock_)
        {
            currentBlock_->addOperation(std::move(op));
        }
    }

    // ============================================================
    // 函数和任务操作创建
    // ============================================================

    std::unique_ptr<FuncOp> Builder::createFuncOp(const std::string& name, std::shared_ptr<FunctionType> type, std::unique_ptr<Region> body)
    {
        return std::make_unique<FuncOp>(name, type, std::move(body));
    }

    std::unique_ptr<FuncOp> Builder::createFuncOp(const std::string& name, const std::vector<std::shared_ptr<Type>>& inputs, const std::vector<std::shared_ptr<Type>>& outputs, std::unique_ptr<Region> body)
    {
        auto funcType = getFunctionType(inputs, outputs);
        return createFuncOp(name, funcType, std::move(body));
    }

    std::unique_ptr<TaskOp> Builder::createTaskOp(const std::string& name, std::shared_ptr<FunctionType> type, std::unique_ptr<Region> body)
    {
        return std::make_unique<TaskOp>(name, type, std::move(body));
    }

    std::unique_ptr<TaskOp> Builder::createTaskOp(const std::string& name, const std::vector<std::shared_ptr<Type>>& inputs, const std::vector<std::shared_ptr<Type>>& outputs, std::unique_ptr<Region> body)
    {
        auto funcType = getFunctionType(inputs, outputs);
        return createTaskOp(name, funcType, std::move(body));
    }

    // ============================================================
    // 并行操作创建
    // ============================================================

    std::unique_ptr<ParallelForOp> Builder::createParallelForOp(std::shared_ptr<Value> lb, std::shared_ptr<Value> ub, std::shared_ptr<Value> step, std::unique_ptr<Region> body)
    {
        return std::make_unique<ParallelForOp>(lb, ub, step, std::move(body));
    }

    std::shared_ptr<Value> Builder::createReduceOp(ReduceOp::ReductionKind kind, std::shared_ptr<Value> input, std::shared_ptr<Value> initValue, const std::vector<int64_t>& axes)
    {
        auto op = std::make_unique<ReduceOp>(kind, input, initValue, axes);
        auto result = op->getResult(0);
        if (currentBlock_)
        {
            currentBlock_->addOperation(std::move(op));
        }
        return result;
    }

    // ============================================================
    // 设备操作创建
    // ============================================================

    std::shared_ptr<Value> Builder::createSpawnOp(std::shared_ptr<Value> device, const std::string& taskName, const std::vector<std::shared_ptr<Value>>& args, bool await)
    {
        auto op = std::make_unique<SpawnOp>(device, taskName, args, await);
        auto result = op->getResult(0);
        if (currentBlock_)
        {
            currentBlock_->addOperation(std::move(op));
        }
        return result;
    }

    void Builder::createSyncOp(std::shared_ptr<Value> device)
    {
        auto op = std::make_unique<SyncOp>(device);
        if (currentBlock_)
        {
            currentBlock_->addOperation(std::move(op));
        }
    }

    std::shared_ptr<Value> Builder::createMoveToOp(std::shared_ptr<Value> buffer, std::shared_ptr<Value> device)
    {
        auto op = std::make_unique<MoveToOp>(buffer, device);
        auto result = op->getResult(0);
        if (currentBlock_)
        {
            currentBlock_->addOperation(std::move(op));
        }
        return result;
    }

    std::shared_ptr<Value> Builder::createPlaceOnOp(std::shared_ptr<Value> buffer, std::shared_ptr<Value> device)
    {
        auto op = std::make_unique<PlaceOnOp>(buffer, device);
        auto result = op->getResult(0);
        if (currentBlock_)
        {
            currentBlock_->addOperation(std::move(op));
        }
        return result;
    }

    // ============================================================
    // 区域和块创建
    // ============================================================

    std::unique_ptr<Region> Builder::createRegion()
    {
        return std::make_unique<Region>();
    }

    Block* Builder::createBlock(std::unique_ptr<Region>& region, const std::vector<std::shared_ptr<Type>>& argTypes)
    {
        auto block = std::make_unique<Block>();
        for (const auto& type : argTypes)
        {
            block->addArgument(type);
        }
        Block* blockPtr = block.get();
        region->addBlock(std::move(block));
        return blockPtr;
    }

    Block* Builder::createBlockBefore(std::unique_ptr<Region>& region, Block* before, const std::vector<std::shared_ptr<Type>>& argTypes)
    {
        // 找到 before 块的位置
        const auto& blocks = region->getBlocks();
        size_t pos = 0;
        for (size_t i = 0; i < blocks.size(); ++i)
        {
            if (blocks[i].get() == before)
            {
                pos = i;
                break;
            }
        }
        
        auto block = std::make_unique<Block>();
        for (const auto& type : argTypes)
        {
            block->addArgument(type);
        }
        
        return region->insertBlock(pos, std::move(block));
    }

    Block* Builder::createBlockAfter(std::unique_ptr<Region>& region, Block* after, const std::vector<std::shared_ptr<Type>>& argTypes)
    {
        // 找到 after 块的位置
        const auto& blocks = region->getBlocks();
        size_t pos = blocks.size();
        for (size_t i = 0; i < blocks.size(); ++i)
        {
            if (blocks[i].get() == after)
            {
                pos = i + 1;
                break;
            }
        }
        
        auto block = std::make_unique<Block>();
        for (const auto& type : argTypes)
        {
            block->addArgument(type);
        }
        
        return region->insertBlock(pos, std::move(block));
    }

    // ============================================================
    // 操作插入
    // ============================================================

    void Builder::insert(std::unique_ptr<Operation> op)
    {
        if (currentBlock_)
        {
            currentBlock_->addOperation(std::move(op));
        }
    }

    std::shared_ptr<Value> Builder::insertAndgetResult(std::unique_ptr<Operation> op, size_t resultIndex)
    {
        auto result = op->getResult(resultIndex);
        if (currentBlock_)
        {
            currentBlock_->addOperation(std::move(op));
        }
        return result;
    }

} // namespace hscir
