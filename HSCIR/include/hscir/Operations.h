#ifndef HSCIR_OPERATIONS_H
#define HSCIR_OPERATIONS_H

#include "Types.h"
#include <string>
#include <vector>
#include <memory>
#include <unordered_map>

namespace hscir
{
    class Value;
    class Block;
    class Region;

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

    // 操作基类
    class Operation
    {
        public:
            explicit Operation(const std::string& name) : name_(name) {}
            virtual ~Operation() = default;

            const std::string& getName() const { return name_; }

            // 操作数
            void addOperation(std::shared_ptr<Value> operand);
            const std::vector<std::shared_ptr<Value>>& getOperands() const { return operands_; }

            // 结果类型
            void addResultType(std::shared_ptr<Type> type);
            const std::vector<std::shared_ptr<Type>>& getResultTypes() const { return resultTypes_; }

            // 属性
            void setAttribute(const std::string& key, std::unique_ptr<Attribute> attr);
            Attribute* getAttribute(const std::string& key) const;

            // 区域
            void addRegion(std::unique_ptr<Region> region);
            const std::vector<std::unique_ptr<Region>>& getRegions() const { return regions_; }

            // 打印
            virtual void print(std::ostream& os, unsigned indent = 0) const;

        protected:
            std::string name_;
            std::vector<std::shared_ptr<Value>> operands_;
            std::vector<std::shared_ptr<Type>> resultTypes_;
            std::unordered_map<std::string, std::unique_ptr<Attribute>> attributes_;
            std::vector<std::unique_ptr<Region>> regions_;
    };

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

    // 操作结果值
    class OpResult : public Value
    {
        public:
            OpResult(std::shared_ptr<Type> type, Operation* op, unsigned index) : Value(std::move(type)), op_(op), index_(index) {}

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
            BlockArgument(std::shared_ptr<Type> type, unsigned index) : Value(std::move(type)), index_(index) {}

            unsigned getIndex() const { return index_; }

            std::string toString() const override;

        private:
            unsigned index_;
    };

    // 基本块
    class Block
    {
        public:
            Block() = default;
            ~Block() = default;

            void addOperation(std::unique_ptr<Operation> op);
            const std::vector<std::unique_ptr<Operation>>& getOperations() const { return operations_; }

            void addArgument(std::shared_ptr<Type> type);
            const std::vector<std::shared_ptr<BlockArgument>>& getArguments() const { return arguments_; }

            void print(std::ostream& os, unsigned indent = 0) const;

        private:
            std::vector<std::unique_ptr<Operation>> operations_;
            std::vector<std::shared_ptr<BlockArgument>> arguments_;
    };

    // 区域（包含一个或多个块）
    class Region
    {
        public:
            Region() = default;
            ~Region() = default;

            void addBlock(std::unique_ptr<Block> block);
            const std::vector<std::unique_ptr<Block>>& getBlocks() const { return blocks_; }

            void print(std::ostream& os, unsigned indent = 0) const;

        private:
            std::vector<std::unique_ptr<Block>> blocks_;
    };

    // 模板（顶层容器）
    class Module
    {
        public:
            explicit Module(const std::string& name) : name_(name) {}
            ~Module() = default;

            void addOperation(std::unique_ptr<Operation> op);
            const std::vector<std::unique_ptr<Operation>>& getOperations() const { return operations_; }

            void print(std::ostream& os) const;

        private:
            std::string name_;
            std::vector<std::unique_ptr<Operation>> operations_;
    };

}

#endif //HSCIR_OPERATIONS_H