#include "hscir/Operations.h"
#include <iostream>

namespace hscir
{
    void Operation::addOperation(std::shared_ptr<Value> operand)
    {
        operands_.push_back(std::move(operand));
    }

    void Operation::addResultType(std::shared_ptr<Type> type)
    {
        resultTypes_.push_back(std::move(type));
    }

    void Operation::setAttribute(const std::string& key, std::unique_ptr<Attribute> attr)
    {
        attributes_[key] = std::move(attr);
    }

    Attribute* Operation::getAttribute(const std::string& key) const
    {
        auto it = attributes_.find(key);
        if (it != attributes_.end())
            return it->second.get();
        return nullptr;
    }

    void Operation::addRegion(std::unique_ptr<Region> region)
    {
        regions_.push_back(std::move(region));
    }

    void Operation::print(std::ostream& os, unsigned indent) const
    {
        std::string indentStr(indent, ' ');
        os << indentStr << "%" << this << " = \"" << name_ << "\"(";
        for (size_t i = 0; i < regions_.size(); ++i)
        {
            if (i > 0) os << ", ";
            os << operands_[i]->toString();
        }
        os << ") : (";
        for (size_t i = 0; i < resultTypes_.size(); ++i)
        {
            if (i > 0) os << ", ";
            os << resultTypes_[i]->toString();
        }
        os << ")";
        if (!attributes_.empty())
        {
            os << " {";
            bool first = true;
            for (const auto& [k, v] : attributes_)
            {
                if (!first) os << ", ";
                os << k << " = " << v->toString();
                first = false;
            }
            os << "}";
        }
        os << "\n";
        for (const auto& region : regions_)
        {
            region->print(os, indent + 2);
        }
    }

    std::string OpResult::toString() const
    {
        return "%" + std::to_string(reinterpret_cast<uintptr_t>(op_)) + "_" + std::to_string(index_);
    }

    std::string BlockArgument::toString() const
    {
        return "%arg" + std::to_string(index_);
    }

    void Block::addOperation(std::unique_ptr<Operation> op)
    {
        operations_.push_back(std::move(op));
    }

    void Block::addArgument(std::shared_ptr<Type> type)
    {
        auto arg = std::make_shared<BlockArgument>(std::move(type), arguments_.size());
        arguments_.push_back(std::move(arg));
    }

    void Block::print(std::ostream& os, unsigned indent) const
    {
        std::string indentStr(indent, ' ');
        if (!arguments_.empty())
        {
            os << indentStr << "block(";
            for (size_t i = 0; i < arguments_.size(); ++i)
            {
                if (i > 0) os << ", ";
                os << arguments_[i]->toString() << ": " << arguments_[i]->getType()->toString();
            }
            os << "):\n";
        } else
        {
            os << indentStr << "block:\n";
        }
        for (const auto& op : operations_)
        {
            op->print(os, indent + 2);
        }
    }

    void Region::addBlock(std::unique_ptr<Block> block)
    {
        blocks_.push_back(std::move(block));
    }

    void Region::print(std::ostream& os, unsigned indent) const
    {
        std::string indentStr(indent, ' ');
        os << indentStr << "{\n";
        for (const auto& block : blocks_)
        {
            block->print(os, indent + 2);
        }
        os << indentStr << "}\n";
    }

    void Module::addOperation(std::unique_ptr<Operation> op)
    {
        operations_.push_back(std::move(op));
    }

    void Module::print(std::ostream& os) const
    {
        os << "module \"" << name_ << "\" {\n";
        for (const auto& op : operations_)
        {
            op->print(os, 2);
        }
        os << "}\n";
    }

}