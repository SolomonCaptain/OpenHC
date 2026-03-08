#ifndef HSCIR_OPERATIONS_H
#define HSCIR_OPERATIONS_H

#include "Types.h"
#include <string>
#include <vector>
#include <memory>
#include <unordered_map>
#include <variant>

namespace hscir
{
    class Block;
    class Region;

    // ============================================================
    // 值定义（前向声明操作类）
    // ============================================================

    // 操作结果值（前置声明）
    class OpResult;
    class BlockArgument;

    // 值（操作结果或块参数）
    class Value
    {
        public:
            explicit Value(std::shared_ptr<Type> type) : type_(std::move(type)) {}
            virtual ~Value() = default;

            std::shared_ptr<Type> getType() const { return type_; }
            virtual std::string toString() const = 0;

        protected:
            std::shared_ptr<Type> type_;
    };

    // ============================================================
    // 属性定义
    // ============================================================

    // 属性值基类
    class Attribute
    {
        public:
            virtual ~Attribute() = default;
            virtual std::string toString() const = 0;
    };

    // 字符串属性
    class StringAttr : public Attribute
    {
        public:
            explicit StringAttr(std::string value) : value_(std::move(value)) {}
            const std::string& getValue() const { return value_; }
            std::string toString() const override { return "\"" + value_ + "\""; }

        private:
            std::string value_;
    };

    // 整型属性
    class IntegerAttr : public Attribute
    {
        public:
            explicit IntegerAttr(int64_t value) : value_(value) {}
            int64_t getValue() const { return value_; }
            std::string toString() const override { return std::to_string(value_); }

        private:
            int64_t value_;
    };

    // 浮点属性
    class FloatAttr : public Attribute
    {
        public:
            explicit FloatAttr(double value) : value_(value) {}
            double getValue() const { return value_; }
            std::string toString() const override;

        private:
            double value_;
    };

    // 布尔属性
    class BoolAttr : public Attribute
    {
        public:
            explicit BoolAttr(bool value) : value_(value) {}
            bool getValue() const { return value_; }
            std::string toString() const override { return value_ ? "true" : "false"; }

        private:
            bool value_;
    };

    // 数组属性
    class ArrayAttr : public Attribute
    {
        public:
            explicit ArrayAttr(std::vector<std::unique_ptr<Attribute>> elements) : elements_(std::move(elements)) {}
            const std::vector<std::unique_ptr<Attribute>>& getElements() const { return elements_; }
            std::string toString() const override;

        private:
            std::vector<std::unique_ptr<Attribute>> elements_;
    };

    // ============================================================
    // 区域和前向声明（解决循环依赖）
    // ============================================================

    // 区域（提前定义以解决 sizeof 问题）
    class Region
    {
        public:
            Region() = default;
            ~Region() = default;

            void addBlock(std::unique_ptr<Block> block);
            Block* insertBlock(size_t pos, std::unique_ptr<Block> block);
            const std::vector<std::unique_ptr<Block>>& getBlocks() const { return blocks_; }
            Block* getBlock(size_t index) const;
            size_t getNumBlocks() const { return blocks_.size(); }
            bool empty() const { return blocks_.empty(); }
            Block* getEntryBlock() const;

            void print(std::ostream& os, unsigned indent = 0) const;

        private:
            std::vector<std::unique_ptr<Block>> blocks_;
    };

    // ============================================================
    // 操作基类
    // ============================================================

    // 操作基类
    class Operation
    {
        public:
            explicit Operation(std::string name) : name_(std::move(name)) {}
            virtual ~Operation() = default;

            const std::string& getName() const { return name_; }

            // 操作数
            void addOperand(std::shared_ptr<Value> operand);
            void addOperands(const std::vector<std::shared_ptr<Value>>& operands);
            const std::vector<std::shared_ptr<Value>>& getOperands() const { return operands_; }
            std::shared_ptr<Value> getOperand(size_t index) const;

            // 结果类型
            void addResultType(std::shared_ptr<Type> type);
            const std::vector<std::shared_ptr<Type>>& getResultTypes() const { return resultTypes_; }
            std::shared_ptr<Type> getResultType(size_t index) const;

            // 属性
            void setAttribute(const std::string& key, std::unique_ptr<Attribute> attr);
            Attribute* getAttribute(const std::string& key) const;
            bool hasAttribute(const std::string& key) const;
            const std::unordered_map<std::string, std::unique_ptr<Attribute>>& getAttributes() const { return attributes_; }

            // 区域
            void addRegion(std::unique_ptr<Region> region);
            const std::vector<std::unique_ptr<Region>>& getRegions() const { return regions_; }
            Region* getRegion(size_t index) const;

            // 结果值（用于 SSA 形式）
            std::shared_ptr<Value> getResult(size_t index) const;

            // 打印
            virtual void print(std::ostream& os, unsigned indent = 0) const;

            // 类型检查
            virtual bool classof(const Operation* op) const { return true; }

        protected:
            std::string name_;
            std::vector<std::shared_ptr<Value>> operands_;
            std::vector<std::shared_ptr<Type>> resultTypes_;
            std::unordered_map<std::string, std::unique_ptr<Attribute>> attributes_;
            std::vector<std::unique_ptr<Region>> regions_;
            mutable std::vector<std::shared_ptr<Value>> results_; // 缓存的结果值
    };

    // ============================================================
    // 值的具体实现（必须在 Operation 定义之后）
    // ============================================================

    // 操作结果值
    class OpResult : public Value
    {
        public:
            OpResult(std::shared_ptr<Type> type, Operation* op, unsigned index)
                : Value(std::move(type)), op_(op), index_(index) {}

            Operation* getOwner() const { return op_; }
            unsigned getIndex() const { return index_; }

            std::string toString() const override;

        private:
            Operation* op_;
            unsigned index_;
    };

    // 块参数值
    class BlockArgument : public Value
    {
        public:
            BlockArgument(std::shared_ptr<Type> type, Block* block, unsigned index)
                : Value(std::move(type)), block_(block), index_(index) {}

            Block* getOwner() const { return block_; }
            unsigned getIndex() const { return index_; }

            std::string toString() const override;

        private:
            Block* block_;
            unsigned index_;
    };

    // ============================================================
    // 算术操作
    // ============================================================

    // 二元算术操作基类
    class BinaryArithOp : public Operation
    {
        public:
            BinaryArithOp(std::string name, std::shared_ptr<Value> lhs, std::shared_ptr<Value> rhs)
                : Operation(std::move(name))
            {
                addOperand(lhs);
                addOperand(rhs);
                // 结果类型与操作数类型相同
                if (lhs) addResultType(lhs->getType());
            }

            std::shared_ptr<Value> getLhs() const { return getOperand(0); }
            std::shared_ptr<Value> getRhs() const { return getOperand(1); }

            void print(std::ostream& os, unsigned indent = 0) const override;
    };

    // 加法操作
    class AddOp : public BinaryArithOp
    {
        public:
            AddOp(std::shared_ptr<Value> lhs, std::shared_ptr<Value> rhs)
                : BinaryArithOp("add", lhs, rhs) {}
    };

    // 减法操作
    class SubOp : public BinaryArithOp
    {
        public:
            SubOp(std::shared_ptr<Value> lhs, std::shared_ptr<Value> rhs)
                : BinaryArithOp("sub", lhs, rhs) {}
    };

    // 乘法操作
    class MulOp : public BinaryArithOp
    {
        public:
            MulOp(std::shared_ptr<Value> lhs, std::shared_ptr<Value> rhs)
                : BinaryArithOp("mul", lhs, rhs) {}
    };

    // 除法操作
    class DivOp : public BinaryArithOp
    {
        public:
            DivOp(std::shared_ptr<Value> lhs, std::shared_ptr<Value> rhs)
                : BinaryArithOp("div", lhs, rhs) {}
    };

    // 取模操作
    class ModOp : public BinaryArithOp
    {
        public:
            ModOp(std::shared_ptr<Value> lhs, std::shared_ptr<Value> rhs)
                : BinaryArithOp("mod", lhs, rhs) {}
    };

    // 比较操作
    class CmpOp : public Operation
    {
        public:
            enum class Predicate {
                EQ,   // ==
                NE,   // !=
                LT,   // <
                LE,   // <=
                GT,   // >
                GE,   // >=
            };

            CmpOp(Predicate pred, std::shared_ptr<Value> lhs, std::shared_ptr<Value> rhs)
                : Operation("cmp"), predicate_(pred)
            {
                addOperand(lhs);
                addOperand(rhs);
                // 结果类型为 bool (i1)
                addResultType(TypeManager::getInstance().getIntegerType(1, false));
            }

            Predicate getPredicate() const { return predicate_; }
            std::shared_ptr<Value> getLhs() const { return getOperand(0); }
            std::shared_ptr<Value> getRhs() const { return getOperand(1); }

            std::string getPredicateString() const;
            void print(std::ostream& os, unsigned indent = 0) const override;

        private:
            Predicate predicate_;
    };

    // ============================================================
    // 内存操作
    // ============================================================

    // 内存分配操作
    class AllocOp : public Operation
    {
        public:
            AllocOp(std::shared_ptr<Type> elementType, const std::vector<std::shared_ptr<Value>>& dims)
                : Operation("alloc")
            {
                // 结果类型为 Buffer
                std::vector<int64_t> shape;
                for (const auto& dim : dims)
                {
                    addOperand(dim);
                    // 尝试从常量获取维度值，否则使用动态维度
                    shape.push_back(-1); // 动态维度
                }
                auto bufferType = TypeManager::getInstance().getBufferType(elementType, shape);
                addResultType(bufferType);
            }

            std::shared_ptr<BufferType> getBufferType() const;
            size_t getNumDims() const { return getOperands().size(); }

            void print(std::ostream& os, unsigned indent = 0) const override;
    };

    // 内存加载操作
    class LoadOp : public Operation
    {
        public:
            LoadOp(std::shared_ptr<Value> buffer, const std::vector<std::shared_ptr<Value>>& indices)
                : Operation("load")
            {
                addOperand(buffer);
                for (const auto& idx : indices)
                {
                    addOperand(idx);
                }
                // 结果类型为 Buffer 的元素类型
                if (buffer)
                {
                    auto bufType = std::dynamic_pointer_cast<BufferType>(buffer->getType());
                    if (bufType)
                    {
                        addResultType(bufType->getElementType());
                    }
                }
            }

            std::shared_ptr<Value> getBuffer() const { return getOperand(0); }
            std::vector<std::shared_ptr<Value>> getIndices() const;

            void print(std::ostream& os, unsigned indent = 0) const override;
    };

    // 内存存储操作
    class StoreOp : public Operation
    {
        public:
            StoreOp(std::shared_ptr<Value> value, std::shared_ptr<Value> buffer, const std::vector<std::shared_ptr<Value>>& indices)
                : Operation("store")
            {
                addOperand(value);
                addOperand(buffer);
                for (const auto& idx : indices)
                {
                    addOperand(idx);
                }
                // 无结果类型
            }

            std::shared_ptr<Value> getValue() const { return getOperand(0); }
            std::shared_ptr<Value> getBuffer() const { return getOperand(1); }
            std::vector<std::shared_ptr<Value>> getIndices() const;

            void print(std::ostream& os, unsigned indent = 0) const override;
    };

    // 常量操作
    class ConstantOp : public Operation
    {
        public:
            ConstantOp(std::shared_ptr<Type> type, int64_t value)
                : Operation("constant")
            {
                addResultType(type);
                setAttribute("value", std::make_unique<IntegerAttr>(value));
            }

            ConstantOp(std::shared_ptr<Type> type, double value)
                : Operation("constant")
            {
                addResultType(type);
                setAttribute("value", std::make_unique<FloatAttr>(value));
            }

            int64_t getIntValue() const;
            double getFloatValue() const;
            bool isInteger() const;

            void print(std::ostream& os, unsigned indent = 0) const override;
    };

    // ============================================================
    // 控制流操作
    // ============================================================

    // 无条件跳转
    class BranchOp : public Operation
    {
        public:
            explicit BranchOp(Block* target, const std::vector<std::shared_ptr<Value>>& args = {})
                : Operation("br")
            {
                setAttribute("target", std::make_unique<IntegerAttr>(reinterpret_cast<int64_t>(target)));
                for (const auto& arg : args)
                {
                    addOperand(arg);
                }
            }

            Block* getTarget() const;
            std::vector<std::shared_ptr<Value>> getArguments() const;

            void print(std::ostream& os, unsigned indent = 0) const override;
    };

    // 条件跳转
    class CondBranchOp : public Operation
    {
        public:
            CondBranchOp(std::shared_ptr<Value> condition, Block* trueBlock, Block* falseBlock,
                        const std::vector<std::shared_ptr<Value>>& trueArgs = {},
                        const std::vector<std::shared_ptr<Value>>& falseArgs = {})
                : Operation("cond_br")
            {
                addOperand(condition);
                setAttribute("true_target", std::make_unique<IntegerAttr>(reinterpret_cast<int64_t>(trueBlock)));
                setAttribute("false_target", std::make_unique<IntegerAttr>(reinterpret_cast<int64_t>(falseBlock)));
                for (const auto& arg : trueArgs)
                {
                    addOperand(arg);
                }
                for (const auto& arg : falseArgs)
                {
                    addOperand(arg);
                }
            }

            std::shared_ptr<Value> getCondition() const { return getOperand(0); }
            Block* getTrueBlock() const;
            Block* getFalseBlock() const;

            void print(std::ostream& os, unsigned indent = 0) const override;
    };

    // 返回操作
    class ReturnOp : public Operation
    {
        public:
            explicit ReturnOp(std::shared_ptr<Value> value = nullptr)
                : Operation("return")
            {
                if (value)
                {
                    addOperand(value);
                }
            }

            bool hasValue() const { return !getOperands().empty(); }
            std::shared_ptr<Value> getValue() const { return hasValue() ? getOperand(0) : nullptr; }

            void print(std::ostream& os, unsigned indent = 0) const override;
    };

    // 函数操作
    class FuncOp : public Operation
    {
        public:
            FuncOp(const std::string& name, std::shared_ptr<FunctionType> type, std::unique_ptr<Region> body = nullptr)
                : Operation("func")
            {
                setAttribute("sym_name", std::make_unique<StringAttr>(name));
                addResultType(type);
                if (body)
                {
                    addRegion(std::move(body));
                }
            }

            std::string getSymName() const;
            std::shared_ptr<FunctionType> getFunctionType() const;
            Region* getBody() const;
            Block* getEntryBlock() const;

            void print(std::ostream& os, unsigned indent = 0) const override;
    };

    // ============================================================
    // 并行操作
    // ============================================================

    // 并行 for 循环
    class ParallelForOp : public Operation
    {
        public:
            ParallelForOp(std::shared_ptr<Value> lowerBound, std::shared_ptr<Value> upperBound,
                         std::shared_ptr<Value> step, std::unique_ptr<Region> body)
                : Operation("hsc.parallel_for")
            {
                addOperand(lowerBound);
                addOperand(upperBound);
                addOperand(step);
                addRegion(std::move(body));
            }

            std::shared_ptr<Value> getLowerBound() const { return getOperand(0); }
            std::shared_ptr<Value> getUpperBound() const { return getOperand(1); }
            std::shared_ptr<Value> getStep() const { return getOperand(2); }
            Region* getBody() const { return getRegion(0); }

            void print(std::ostream& os, unsigned indent = 0) const override;
    };

    // Reduce 操作
    class ReduceOp : public Operation
    {
        public:
            enum class ReductionKind {
                SUM,    // 求和
                PROD,   // 求积
                MIN,    // 最小值
                MAX,    // 最大值
                AND,    // 按位与
                OR,     // 按位或
                XOR,    // 按位异或
            };

            ReduceOp(ReductionKind kind, std::shared_ptr<Value> input,
                    std::shared_ptr<Value> initValue, const std::vector<int64_t>& axes)
                : Operation("hsc.reduce"), kind_(kind)
            {
                addOperand(input);
                addOperand(initValue);

                // 设置 axes 属性
                auto axesAttr = std::make_unique<ArrayAttr>(std::vector<std::unique_ptr<Attribute>>());
                for (int64_t axis : axes)
                {
                    // 这里需要创建 ArrayAttr，暂时简化处理
                }
                setAttribute("axes", std::move(axesAttr));
                setAttribute("kind", std::make_unique<StringAttr>(getReductionKindString()));
            }

            ReductionKind getReductionKind() const { return kind_; }
            std::shared_ptr<Value> getInput() const { return getOperand(0); }
            std::shared_ptr<Value> getInitValue() const { return getOperand(1); }
            std::string getReductionKindString() const;

            void print(std::ostream& os, unsigned indent = 0) const override;

        private:
            ReductionKind kind_;
    };

    // ============================================================
    // 设备操作
    // ============================================================

    // Spawn 操作（在设备上启动任务）
    class SpawnOp : public Operation
    {
        public:
            SpawnOp(std::shared_ptr<Value> device, const std::string& taskName,
                   const std::vector<std::shared_ptr<Value>>& args, bool await)
                : Operation("hsc.spawn")
            {
                addOperand(device);
                setAttribute("task", std::make_unique<StringAttr>(taskName));
                setAttribute("await", std::make_unique<BoolAttr>(await));
                for (const auto& arg : args)
                {
                    addOperand(arg);
                }
            }

            std::shared_ptr<Value> getDevice() const { return getOperand(0); }
            std::string getTaskName() const;
            std::vector<std::shared_ptr<Value>> getArguments() const;
            bool isAwait() const;

            void print(std::ostream& os, unsigned indent = 0) const override;
    };

    // Sync 操作（同步设备）
    class SyncOp : public Operation
    {
        public:
            explicit SyncOp(std::shared_ptr<Value> device = nullptr)
                : Operation("hsc.sync")
            {
                if (device)
                {
                    addOperand(device);
                }
            }

            std::shared_ptr<Value> getDevice() const { return getOperands().empty() ? nullptr : getOperand(0); }

            void print(std::ostream& os, unsigned indent = 0) const override;
    };

    // MoveTo 操作（在设备间移动数据）
    class MoveToOp : public Operation
    {
        public:
            MoveToOp(std::shared_ptr<Value> buffer, std::shared_ptr<Value> device)
                : Operation("hsc.move_to")
            {
                addOperand(buffer);
                addOperand(device);
                // 结果类型与输入 buffer 相同
                if (buffer)
                {
                    addResultType(buffer->getType());
                }
            }

            std::shared_ptr<Value> getBuffer() const { return getOperand(0); }
            std::shared_ptr<Value> getDevice() const { return getOperand(1); }

            void print(std::ostream& os, unsigned indent = 0) const override;
    };

    // PlaceOn 操作（标记数据放置位置）
    class PlaceOnOp : public Operation
    {
        public:
            PlaceOnOp(std::shared_ptr<Value> buffer, std::shared_ptr<Value> device)
                : Operation("hsc.place_on")
            {
                addOperand(buffer);
                addOperand(device);
                // 结果类型与输入 buffer 相同
                if (buffer)
                {
                    addResultType(buffer->getType());
                }
            }

            std::shared_ptr<Value> getBuffer() const { return getOperand(0); }
            std::shared_ptr<Value> getDevice() const { return getOperand(1); }

            void print(std::ostream& os, unsigned indent = 0) const override;
    };

    // Task 操作（任务定义）
    class TaskOp : public Operation
    {
        public:
            TaskOp(const std::string& name, std::shared_ptr<FunctionType> type,
                  std::unique_ptr<Region> body = nullptr)
                : Operation("hsc.task")
            {
                setAttribute("sym_name", std::make_unique<StringAttr>(name));
                addResultType(type);
                if (body)
                {
                    addRegion(std::move(body));
                }
            }

            std::string getSymName() const;
            std::shared_ptr<FunctionType> getFunctionType() const;
            Region* getBody() const;
            Block* getEntryBlock() const;

            void print(std::ostream& os, unsigned indent = 0) const override;
    };

    // ============================================================
    // 基本块和区域
    // ============================================================

    // 基本块
    class Block
    {
    public:
        Block() = default;
        ~Block() = default;

        void addOperation(std::unique_ptr<Operation> op);
        void insertOperation(size_t pos, std::unique_ptr<Operation> op);
        const std::vector<std::unique_ptr<Operation>>& getOperations() const { return operations_; }
        Operation* getOperation(size_t index) const;
        size_t getNumOperations() const { return operations_.size(); }
        bool empty() const { return operations_.empty(); }
        Operation* getTerminator() const;

        void addArgument(std::shared_ptr<Type> type);
        const std::vector<std::shared_ptr<BlockArgument>>& getArguments() const { return arguments_; }
        std::shared_ptr<BlockArgument> getArgument(size_t index) const;
        size_t getNumArguments() const { return arguments_.size(); }

        void print(std::ostream& os, unsigned indent = 0) const;

    private:
        std::vector<std::unique_ptr<Operation>> operations_;
        std::vector<std::shared_ptr<BlockArgument>> arguments_;
    };

    // 模块（顶层容器）
    class Module
    {
        public:
            explicit Module(const std::string& name) : name_(name) {}
            ~Module() = default;

            void addOperation(std::unique_ptr<Operation> op);
            void insertOperation(size_t pos, std::unique_ptr<Operation> op);
            const std::vector<std::unique_ptr<Operation>>& getOperations() const { return operations_; }
            Operation* getOperation(size_t index) const;
            size_t getNumOperations() const { return operations_.size(); }

            // 查找操作
            FuncOp* lookupFunction(const std::string& name) const;
            TaskOp* lookupTask(const std::string& name) const;

            void print(std::ostream& os) const;

        private:
            std::string name_;
            std::vector<std::unique_ptr<Operation>> operations_;
            std::unordered_map<std::string, Operation*> symbolTable_;
    };

} // namespace hscir

#endif //HSCIR_OPERATIONS_H