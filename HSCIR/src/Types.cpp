#include "hscir/Types.h"
#include <sstream>
#include <functional>

namespace hscir
{
    // TypeManager 单例
    TypeManager& TypeManager::getInstance()
    {
        static TypeManager instance;
        return instance;
    }

    std::shared_ptr<IntegerType> TypeManager::getIntegerType(unsigned width, bool isSigned)
    {
        IntegerTypeKey key(width, isSigned);
        auto it = integerTypes_.find(key);
        if (it != integerTypes_.end())
        {
            return it->second;
        }
        auto type = std::make_shared<IntegerType>(width, isSigned);
        integerTypes_[key] = type;
        return type;
    }

    std::shared_ptr<FloatType> TypeManager::getFloatType(unsigned width)
    {
        FloatTypeKey key(width);
        auto it = floatTypes_.find(key);
        if (it != floatTypes_.end())
        {
            return it->second;
        }
        auto type = std::make_shared<FloatType>(width);
        floatTypes_[key] = type;
        return type;
    }

    std::shared_ptr<BufferType> TypeManager::getBufferType(std::shared_ptr<Type> elemType, std::vector<int64_t> shape)
    {
        BufferTypeKey key{std::move(elemType), std::move(shape)};
        auto it = bufferTypes_.find(key);
        if (it != bufferTypes_.end())
        {
            return it->second;
        }
        auto type = std::make_shared<BufferType>(key.elemType, key.shape);
        bufferTypes_[key] = type;
        return type;
    }

    std::shared_ptr<FunctionType> TypeManager::getFunctionType(std::vector<std::shared_ptr<Type>> inputs, std::vector<std::shared_ptr<Type>> outputs)
    {
        FunctionTypeKey key{std::move(inputs), std::move(outputs)};
        auto it = functionTypes_.find(key);
        if (it != functionTypes_.end())
        {
            return it->second;
        }
        auto type = std::make_shared<FunctionType>(key.inputs, key.outputs);
        functionTypes_[key] = type;
        return type;
    }

    bool TypeManager::BufferTypeKey::operator==(const BufferTypeKey& other) const
    {
        return *elemType == *other.elemType && shape == other.shape;
    }

    std::size_t TypeManager::BufferTypeHash::operator()(const BufferTypeKey& key) const
    {
        std::size_t h = std::hash<std::shared_ptr<Type>>()(key.elemType);
        for (auto d : key.shape)
        {
            h ^= std::hash<int64_t>()(d) + 0x9e3779b9 + (h << 6) + (h >> 2);
        }
        return h;
    }

    bool TypeManager::FunctionTypeKey::operator==(const FunctionTypeKey& other) const
    {
        if (inputs.size() != other.inputs.size() || outputs.size() != other.outputs.size()) { return false; }
        for (size_t i = 0; i < inputs.size(); ++i)
        {
            if (*inputs[i] != *other.inputs[i]) { return false; }
        }
        for (size_t i = 0; i < outputs.size(); ++i)
        {
            if (*outputs[i] != *other.outputs[i]) { return false; }
        }
        return true;
    }

    std::size_t TypeManager::FunctionTypeHash::operator()(const FunctionTypeKey& key) const
    {
        std::size_t h = 0;
        for (const auto& t : key.inputs)
        {
            h ^= std::hash<std::shared_ptr<Type>>()(t) + 0x9e3779b9 + (h << 6) + (h >> 2);
        }
        for (const auto& t : key.outputs)
        {
            h ^= std::hash<std::shared_ptr<Type>>()(t) + 0x9e3779b9 + (h << 6) + (h >> 2);
        }
        return h;
    }

    // IntegerType
    std::string IntegerType::toString() const
    {
        return (isSigned_ ? "i" : "u") + std::to_string(width_);
    }

    bool IntegerType::operator==(const IntegerType& other) const
    {
        return this->width_ == other.width_ && this->isSigned_ == other.isSigned_;
    }

    bool IntegerType::operator==(const Type& other) const
    {
        if (auto intOther = dynamic_cast<const IntegerType*>(&other))
        {
            return *this == *intOther;
        }
        return false;
    }

    // FloatType
    std::string FloatType::toString() const
    {
        return "f" + std::to_string(width_);
    }
    bool FloatType::operator==(const FloatType& other) const
    {
        return this->width_ == other.width_;
    }

    bool FloatType::operator==(const Type& other) const
    {
        if (auto floatOther = dynamic_cast<const FloatType*>(&other))
        {
            return *this == *floatOther;
        }
        return false;
    }

    // BufferType
    std::string BufferType::toString() const
    {
        std::ostringstream oss;
        oss << "buffer<" << elementType_->toString();
        if (!shape_.empty())
        {
            oss << ", [";
            for (size_t i = 0; i < shape_.size(); ++i)
            {
                if (i > 0) oss << ", ";
                oss << shape_[i];
            }
            oss << "]";
        }
        oss << ">";
        return oss.str();
    }

    bool BufferType::operator==(const BufferType& other) const
    {
        return this->elementType_ == other.elementType_ && this->shape_ == other.shape_;
    }

    bool BufferType::operator==(const Type& other) const
    {
        if (auto bufferOther = dynamic_cast<const BufferType*>(&other))
        {
            return *this == *bufferOther;
        }
        return false;
    }

    // FunctionType
    std::string FunctionType::toString() const
    {
        std::ostringstream oss;
        oss << "(";
        for (size_t i = 0; i < inputs_.size(); ++i)
        {
            if (i > 0) oss << ", ";
            oss << inputs_[i]->toString();
        }
        oss << ") -> ";
        if (outputs_.size() == 1)
        {
            oss << outputs_[0]->toString();
        } else
        {
            oss << "(";
            for (size_t i = 0; i < outputs_.size(); ++i)
            {
                if (i > 0) oss << ", ";
                oss << outputs_[i]->toString();
            }
            oss << ")";
        }
        return oss.str();
    }
    bool FunctionType::operator==(const FunctionType& other) const
    {
        return this->inputs_ == other.inputs_ && this->outputs_ == other.outputs_;
    }

    bool FunctionType::operator==(const Type& other) const
    {
        if (auto funcOther = dynamic_cast<const FunctionType*>(&other))
        {
            return *this == *funcOther;
        }
        return false;
    }
}
