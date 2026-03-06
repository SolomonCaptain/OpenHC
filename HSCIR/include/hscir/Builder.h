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

            // ============================================================
            // 插入点管理
            // ============================================================

            void setInsertionPoint(Block* block);
            void setInsertionPointToStart(Block* block);
            void setInsertionPointToEnd(Block* block);
            Block* getInsertionBlock() const { return currentBlock_; }
            void clearInsertionPoint() { currentBlock_ = nullptr; }

            // ============================================================
            // 类型创建（委托给 TypeManager）
            // ============================================================

            std::shared_ptr<IntegerType> getI1Type();
            std::shared_ptr<IntegerType> getI8Type();
            std::shared_ptr<IntegerType> getI16Type();
            std::shared_ptr<IntegerType> getI32Type();
            std::shared_ptr<IntegerType> getI64Type();
            std::shared_ptr<IntegerType> getI128Type();
            std::shared_ptr<IntegerType> getIntegerType(unsigned width, bool isSigned = true);
            
            std::shared_ptr<FloatType> getF16Type();
            std::shared_ptr<FloatType> getF32Type();
            std::shared_ptr<FloatType> getF64Type();
            std::shared_ptr<FloatType> getFloatType(unsigned width);
            
            std::shared_ptr<BufferType> getBufferType(std::shared_ptr<Type> elemType, const std::vector<int64_t>& shape = {});
            std::shared_ptr<FunctionType> getFunctionType(const std::vector<std::shared_ptr<Type>>& inputs, const std::vector<std::shared_ptr<Type>>& outputs);

            // ============================================================
            // 常量创建
            // ============================================================

            std::shared_ptr<Value> createConstant(std::shared_ptr<Type> type, int64_t value);
            std::shared_ptr<Value> createConstant(std::shared_ptr<Type> type, double value);
            std::shared_ptr<Value> createI32Constant(int32_t value);
            std::shared_ptr<Value> createI64Constant(int64_t value);
            std::shared_ptr<Value> createF32Constant(float value);
            std::shared_ptr<Value> createF64Constant(double value);
            std::shared_ptr<Value> createBoolConstant(bool value);

            // ============================================================
            // 算术操作创建
            // ============================================================

            std::shared_ptr<Value> createAddOp(std::shared_ptr<Value> lhs, std::shared_ptr<Value> rhs);
            std::shared_ptr<Value> createSubOp(std::shared_ptr<Value> lhs, std::shared_ptr<Value> rhs);
            std::shared_ptr<Value> createMulOp(std::shared_ptr<Value> lhs, std::shared_ptr<Value> rhs);
            std::shared_ptr<Value> createDivOp(std::shared_ptr<Value> lhs, std::shared_ptr<Value> rhs);
            std::shared_ptr<Value> createModOp(std::shared_ptr<Value> lhs, std::shared_ptr<Value> rhs);
            std::shared_ptr<Value> createCmpOp(CmpOp::Predicate pred, std::shared_ptr<Value> lhs, std::shared_ptr<Value> rhs);

            // ============================================================
            // 内存操作创建
            // ============================================================

            std::shared_ptr<Value> createAllocOp(std::shared_ptr<Type> elementType, const std::vector<std::shared_ptr<Value>>& dims);
            std::shared_ptr<Value> createLoadOp(std::shared_ptr<Value> buffer, const std::vector<std::shared_ptr<Value>>& indices);
            void createStoreOp(std::shared_ptr<Value> value, std::shared_ptr<Value> buffer, const std::vector<std::shared_ptr<Value>>& indices);

            // ============================================================
            // 控制流操作创建
            // ============================================================

            void createBranchOp(Block* target, const std::vector<std::shared_ptr<Value>>& args = {});
            void createCondBranchOp(std::shared_ptr<Value> condition, Block* trueBlock, Block* falseBlock,
                                   const std::vector<std::shared_ptr<Value>>& trueArgs = {},
                                   const std::vector<std::shared_ptr<Value>>& falseArgs = {});
            void createReturnOp(std::shared_ptr<Value> value = nullptr);

            // ============================================================
            // 函数和任务操作创建
            // ============================================================

            std::unique_ptr<FuncOp> createFuncOp(const std::string& name, std::shared_ptr<FunctionType> type, std::unique_ptr<Region> body = nullptr);
            std::unique_ptr<FuncOp> createFuncOp(const std::string& name, const std::vector<std::shared_ptr<Type>>& inputs, const std::vector<std::shared_ptr<Type>>& outputs, std::unique_ptr<Region> body = nullptr);
            std::unique_ptr<TaskOp> createTaskOp(const std::string& name, std::shared_ptr<FunctionType> type, std::unique_ptr<Region> body = nullptr);
            std::unique_ptr<TaskOp> createTaskOp(const std::string& name, const std::vector<std::shared_ptr<Type>>& inputs, const std::vector<std::shared_ptr<Type>>& outputs, std::unique_ptr<Region> body = nullptr);

            // ============================================================
            // 并行操作创建
            // ============================================================

            std::unique_ptr<ParallelForOp> createParallelForOp(std::shared_ptr<Value> lb, std::shared_ptr<Value> ub, std::shared_ptr<Value> step, std::unique_ptr<Region> body);
            std::shared_ptr<Value> createReduceOp(ReduceOp::ReductionKind kind, std::shared_ptr<Value> input, std::shared_ptr<Value> initValue, const std::vector<int64_t>& axes);

            // ============================================================
            // 设备操作创建
            // ============================================================

            std::shared_ptr<Value> createSpawnOp(std::shared_ptr<Value> device, const std::string& taskName, const std::vector<std::shared_ptr<Value>>& args, bool await);
            void createSyncOp(std::shared_ptr<Value> device = nullptr);
            std::shared_ptr<Value> createMoveToOp(std::shared_ptr<Value> buffer, std::shared_ptr<Value> device);
            std::shared_ptr<Value> createPlaceOnOp(std::shared_ptr<Value> buffer, std::shared_ptr<Value> device);

            // ============================================================
            // 区域和块创建
            // ============================================================

            std::unique_ptr<Region> createRegion();
            Block* createBlock(std::unique_ptr<Region>& region, const std::vector<std::shared_ptr<Type>>& argTypes = {});
            Block* createBlockBefore(std::unique_ptr<Region>& region, Block* before, const std::vector<std::shared_ptr<Type>>& argTypes = {});
            Block* createBlockAfter(std::unique_ptr<Region>& region, Block* after, const std::vector<std::shared_ptr<Type>>& argTypes = {});

            // ============================================================
            // 操作插入
            // ============================================================

            void insert(std::unique_ptr<Operation> op);
            std::shared_ptr<Value> insertAndgetResult(std::unique_ptr<Operation> op, size_t resultIndex = 0);

        private:
            Block* currentBlock_ = nullptr;
    };

} // namespace hscir

#endif //HSCIR_BUILDER_H