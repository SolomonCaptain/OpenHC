#include "hscir/Builder.h"

namespace hscir
{
    void Builder::setInsertionPoint(Block* block)
    {
        currentBlock_ = block;
    }

    void Builder::setInsertionPointToStart(Block* block)
    {
        currentBlock_ = block;
        // 目前仅实现支持插入到块末尾。 TODO: 支持插入到特定位置
    }

    void Builder::setInsertionPointToEnd(Block* block)
    {
        currentBlock_ = block;
    }

    std::shared_ptr<IntegerType> Builder::getI32Type()
    {
        return TypeManager::getInstance().getIntegerType(32, true);
    }

    std::shared_ptr<FloatType> Builder::getF32Type()
    {
        return TypeManager::getInstance().getFloatType(32);
    }

    std::shared_ptr<BufferType> Builder::getBufferType(std::shared_ptr<Type> elemType, const std::vector<int64_t>& shape)
    {
        return TypeManager::getInstance().getBufferType(std::move(elemType), shape);
    }

    std::shared_ptr<Value> Builder::createConstant(std::shared_ptr<Type> type, int64_t value)
    {
        auto op = std::make_unique<Operation>("constant");
        op->addResultType(type);
        op->setAttribute("value", std::make_unique<IntegerAttr>(value));
        // 插入当前块
        if (currentBlock_)
        {
            currentBlock_->addOperation(std::move(op));
        }
        return nullptr;
    }

    std::unique_ptr<Operation> Builder::createFuncOp(const std::string& name, std::vector<std::shared_ptr<Type>> inputs, std::vector<std::shared_ptr<Type>> outputs, std::unique_ptr<Region> body)
    {
        auto op = std::make_unique<Operation>("func");
        op->setAttribute("sym_name", std::make_unique<StringAttr>(name));
        auto funcType = TypeManager::getInstance().getFunctionType(std::move(inputs), std::move(outputs));
        op->addResultType(funcType);
        op->addRegion(std::move(body));
        return op;
    }

    std::unique_ptr<Operation> Builder::createTaskOp(const std::string& name, std::vector<std::shared_ptr<Type>> inputs, std::vector<std::shared_ptr<Type>> outputs, std::unique_ptr<Region> body)
    {
        auto op = std::make_unique<Operation>("hsc.task");
        op->setAttribute("sym_name", std::make_unique<StringAttr>(name));
        auto funcType = TypeManager::getInstance().getFunctionType(std::move(inputs), std::move(outputs));
        op->addResultType(funcType);
        op->addRegion(std::move(body));
        return op;
    }

    std::unique_ptr<Operation> Builder::createParallelForOp(std::shared_ptr<Value> lb, std::shared_ptr<Value> ub, std::shared_ptr<Value> step, std::unique_ptr<Region> body)
    {
        auto op = std::make_unique<Operation>("hsc.parallel_for");
        op->addOperation(lb);
        op->addOperation(ub);
        op->addOperation(step);
        op->addRegion(std::move(body));
        return op;
    }

    std::unique_ptr<Operation> Builder::createSpawnOp(std::shared_ptr<Value> task, std::vector<std::shared_ptr<Value>> args, bool await)
    {
        auto op = std::make_unique<Operation>("hsc.spawn");
        op->addOperation(task);
        for (auto& arg : args)
        {
            op->addOperation(arg);
        }
        op->setAttribute("await", std::make_unique<IntegerAttr>(await ? 1 : 0));
        return op;
    }

    std::unique_ptr<Operation> Builder::createPlaceOnOp(std::shared_ptr<Value> buffer, std::shared_ptr<Value> device)
    {
        auto op = std::make_unique<Operation>("hsc.place_on");
        op->addOperation(buffer);
        op->addOperation(device);
        return op;
    }

    std::unique_ptr<Region> Builder::createRegion()
    {
        return std::make_unique<Region>();
    }

    std::unique_ptr<Block> Builder::createBlock(std::unique_ptr<Region>& region, const std::vector<std::shared_ptr<Type>>& argTypes)
    {
        auto block = std::make_unique<Block>();
        for (auto& type : argTypes)
        {
            block->addArgument(type);
        }
        region->addBlock(std::move(block));
        return nullptr;
    }

    void Builder::insert(std::unique_ptr<Operation> op)
    {
        if (currentBlock_)
        {
            currentBlock_->addOperation(std::move(op));
        }
    }

}