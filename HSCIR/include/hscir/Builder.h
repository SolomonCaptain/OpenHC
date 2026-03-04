#ifndef HSCIR_BUILDER_H
#define HSCIR_BUILDER_H

#include "Operations.h"
#include "Types.h"
#include <memory>

namespace hscir
{
    class Builder
    {
        public:
            Builder() = default;
            ~Builder() = default;

            // 插入点管理
            void setInsertionPoint(Block* block);
            void setInsertionPointToStart(Block* block);
            void setInsertionPointToEnd(Block* block);

            // 创建类型（委托给 TypeManager）
            std::shared_ptr<IntegerType> getI32Type();
            std::shared_ptr<FloatType> getF32Type();
            std::shared_ptr<BufferType> getBufferType(std::shared_ptr<Type> elemType, const std::vector<int64_t>& shape = {});

            // 创建值和操作
            std::shared_ptr<Value> createConstant(std::shared_ptr<Type> type, int64_t value);
            std::unique_ptr<Operation> createFuncOp(const std::string& name, std::vector<std::shared_ptr<Type>> inputs, std::vector<std::shared_ptr<Type>> outputs, std::unique_ptr<Region> body);
            std::unique_ptr<Operation> createTaskOp(const std::string& name, std::vector<std::shared_ptr<Type>> inputs, std::vector<std::shared_ptr<Type>> outputs, std::unique_ptr<Region> body);
            std::unique_ptr<Operation> createParallelForOp(std::shared_ptr<Value> lb, std::shared_ptr<Value> ub, std::shared_ptr<Value> step, std::unique_ptr<Region> body);
            std::unique_ptr<Operation> createSpawnOp(std::shared_ptr<Value> task, std::vector<std::shared_ptr<Value>> args, bool await);
            std::unique_ptr<Operation> createPlaceOnOp(std::shared_ptr<Value> buffer, std::shared_ptr<Value> device);

            // 区域和块创建
            std::unique_ptr<Region> createRegion();
            Block* createBlock(std::unique_ptr<Region>& region, const std::vector<std::shared_ptr<Type>>& argTypes = {});

            // 插入操作到当前块
            void insert(std::unique_ptr<Operation> op);

        private:
            Block* currentBlock_ = nullptr;
    };

}

#endif //HSCIR_BUILDER_H