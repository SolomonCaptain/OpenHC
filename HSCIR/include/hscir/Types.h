#ifndef HSCIR_TYPES_H
#define HSCIR_TYPES_H

#include <memory>
#include <vector>
#include <string>
#include <unordered_map>
#include <cstdint>

namespace hscir
{
    class Context;

    // 类型基类
    class Type
    {
        public:
            enum class Kind
            {
                Integer,
                Float,
                Buffer,
                Function,
                None,
            };

            explicit Type(Kind kind) : kind_(kind) {}
            virtual ~Type() = default;

            Kind getKind() const { return kind_; }
            virtual std::string toString() const = 0;

            // 类型比较（用于规范化存储）
            virtual bool operator==(const Type& other) const = 0;
            bool operator!=(const Type& other) const { return !(*this == other); }

        protected:
            Kind kind_;
    };

    // 整数类型
    class IntegerType : public Type
    {
        public:
            explicit IntegerType(unsigned width, bool isSigned = true) : Type(Kind::Integer), width_(width), isSigned_(isSigned) {}

            unsigned getWidth() const { return width_; }
            bool isSigned() const { return isSigned_; }

            std::string toString() const override;
            bool operator==(const IntegerType&) const;
            bool operator==(const Type&) const override;

        private:
            unsigned width_;
            bool isSigned_;
    };

    // 浮点类型
    class FloatType : public Type
    {
        public:
            explicit FloatType(unsigned width) : Type(Kind::Float), width_(width) {}

            unsigned getWidth() const { return width_; }

            std::string toString() const override;
            bool operator==(const FloatType&) const;
            bool operator==(const Type&) const override;

        private:
            unsigned width_;
    };

    // 缓冲区类型（对应 hsc.buffer）
    class BufferType : public Type
    {
        public:
            BufferType(std::shared_ptr<Type> elementType, std::vector<int64_t> shape = {}) : Type(Kind::Buffer), elementType_(std::move(elementType)), shape_(std::move(shape)) {}

            const std::shared_ptr<Type>& getElementType() const { return elementType_; }
            const std::vector<int64_t>& getShape() const { return shape_; }
            size_t getRank() const { return shape_.size(); }

            std::string toString() const override;
            bool operator==(const BufferType&) const;
            bool operator==(const Type&) const override;

        private:
            std::shared_ptr<Type> elementType_;
            std::vector<int64_t> shape_;
    };

    // 函数类型（用于函数/任务签名）
    class FunctionType : public Type
    {
        public:
            FunctionType(std::vector<std::shared_ptr<Type>> inputs, std::vector<std::shared_ptr<Type>> outputs) : Type(Kind::Function), inputs_(std::move(inputs)), outputs_(std::move(outputs)) {}

            const std::vector<std::shared_ptr<Type>>& getInputs() const { return inputs_; }
            const std::vector<std::shared_ptr<Type>>& getOutputs() const { return outputs_; }

            std::string toString() const override;
            bool operator==(const FunctionType&) const;
            bool operator==(const Type&) const override;

        private:
            std::vector<std::shared_ptr<Type>> inputs_;
            std::vector<std::shared_ptr<Type>> outputs_;
    };

    // 类型管理器（确保类型唯一）
    class TypeManager
    {
        public:
            // 获取单例
            static TypeManager& getInstance();

            // 获取已规范化的类型
            std::shared_ptr<IntegerType> getIntegerType(unsigned width, bool isSigned = true);
            std::shared_ptr<FloatType> getFloatType(unsigned width);
            std::shared_ptr<BufferType> getBufferType(std::shared_ptr<Type> elemType, std::vector<int64_t> shape);
            std::shared_ptr<FunctionType> getFunctionType(std::vector<std::shared_ptr<Type>> inputs, std::vector<std::shared_ptr<Type>> outputs);

        private:
            TypeManager() = default;

            struct IntegerTypeKey
            {
                unsigned width;
                bool isSigned;
                bool operator==(const IntegerTypeKey& other) const = default;
            };
            struct IntegerTypeHash
            {
                std::size_t operator()(const IntegerTypeKey& key) const
                {
                    return std::hash<unsigned>()(key.width) ^ (std::hash<bool>()(key.isSigned) << 1);
                }
            };
            std::unordered_map<IntegerTypeKey, std::shared_ptr<IntegerType>, IntegerTypeHash> integerTypes_;

            struct FloatTypeKey
            {
                unsigned width;
                bool operator==(const FloatTypeKey& other) const = default;
            };
            struct FloatTypeHash
            {
                std::size_t operator()(const FloatTypeKey& key) const
                {
                    return std::hash<unsigned>()(key.width);
                }
            };
            std::unordered_map<FloatTypeKey, std::shared_ptr<FloatType>, FloatTypeHash> floatTypes_;

            struct BufferTypeKey
            {
                std::shared_ptr<Type> elemType;
                std::vector<int64_t> shape;
                bool operator==(const BufferTypeKey& other) const;
            };
            struct BufferTypeHash
            {
                std::size_t operator()(const BufferTypeKey& key) const;
            };
            std::unordered_map<BufferTypeKey, std::shared_ptr<BufferType>, BufferTypeHash> bufferTypes_;

            struct FunctionTypeKey
            {
                std::vector<std::shared_ptr<Type>> inputs;
                std::vector<std::shared_ptr<Type>> outputs;
                bool operator==(const FunctionTypeKey& other) const;
            };
            struct FunctionTypeHash
            {
                std::size_t operator()(const FunctionTypeKey& key) const;
            };
            std::unordered_map<FunctionTypeKey, std::shared_ptr<FunctionType>, FunctionTypeHash> functionTypes_;
    };

} // namespace hscir

#endif //HSCIR_TYPES_H