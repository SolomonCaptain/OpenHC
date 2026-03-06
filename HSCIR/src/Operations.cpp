#include "hscir/Operations.h"
#include <iostream>
#include <sstream>
#include <iomanip>

namespace hscir
{

    // ============================================================
    // Attribute 实现
    // ============================================================

    std::string FloatAttr::toString() const
    {
        std::ostringstream oss;
        oss << std::setprecision(17) << value_;
        return oss.str();
    }

    std::string ArrayAttr::toString() const
    {
        std::string result = "[";
        for (size_t i = 0; i < elements_.size(); ++i)
        {
            if (i > 0) result += ", ";
            result += elements_[i]->toString();
        }
        result += "]";
        return result;
    }

    // ============================================================
    // Operation 实现
    // ============================================================

    void Operation::addOperand(std::shared_ptr<Value> operand)
    {
        operands_.push_back(std::move(operand));
    }

    void Operation::addOperands(const std::vector<std::shared_ptr<Value>>& operands)
    {
        for (const auto& op : operands)
        {
            operands_.push_back(op);
        }
    }

    std::shared_ptr<Value> Operation::getOperand(size_t index) const
    {
        if (index < operands_.size())
        {
            return operands_[index];
        }
        return nullptr;
    }

    void Operation::addResultType(std::shared_ptr<Type> type)
    {
        resultTypes_.push_back(std::move(type));
    }

    std::shared_ptr<Type> Operation::getResultType(size_t index) const
    {
        if (index < resultTypes_.size())
        {
            return resultTypes_[index];
        }
        return nullptr;
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

    bool Operation::hasAttribute(const std::string& key) const
    {
        return attributes_.find(key) != attributes_.end();
    }

    void Operation::addRegion(std::unique_ptr<Region> region)
    {
        regions_.push_back(std::move(region));
    }

    Region* Operation::getRegion(size_t index) const
    {
        if (index < regions_.size())
        {
            return regions_[index].get();
        }
        return nullptr;
    }

    std::shared_ptr<Value> Operation::getResult(size_t index) const
    {
        if (index >= results_.size())
        {
            results_.resize(index + 1);
        }
        if (!results_[index])
        {
            results_[index] = std::make_shared<OpResult>(getResultType(index), 
                                                          const_cast<Operation*>(this), index);
        }
        return results_[index];
    }

    void Operation::print(std::ostream& os, unsigned indent) const
    {
        std::string indentStr(indent, ' ');
        
        // 打印结果
        if (!resultTypes_.empty())
        {
            os << indentStr << "%result_" << this << " = ";
        }
        else
        {
            os << indentStr;
        }
        
        // 打印操作名
        os << "\"" << name_ << "\"(";
        
        // 打印操作数
        for (size_t i = 0; i < operands_.size(); ++i)
        {
            if (i > 0) os << ", ";
            os << operands_[i]->toString();
        }
        os << ")";
        
        // 打印结果类型
        if (!resultTypes_.empty())
        {
            os << " : (";
            for (size_t i = 0; i < resultTypes_.size(); ++i)
            {
                if (i > 0) os << ", ";
                os << resultTypes_[i]->toString();
            }
            os << ")";
        }
        
        // 打印属性
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
        
        // 打印区域
        for (const auto& region : regions_)
        {
            region->print(os, indent + 2);
        }
    }

    // ============================================================
    // BinaryArithOp 实现
    // ============================================================

    void BinaryArithOp::print(std::ostream& os, unsigned indent) const
    {
        std::string indentStr(indent, ' ');
        os << indentStr << "%" << this << " = \"" << name_ << "\"(" 
           << getLhs()->toString() << ", " << getRhs()->toString() 
           << ") : " << resultTypes_[0]->toString() << "\n";
    }

    // ============================================================
    // CmpOp 实现
    // ============================================================

    std::string CmpOp::getPredicateString() const
    {
        switch (predicate_)
        {
            case Predicate::EQ: return "eq";
            case Predicate::NE: return "ne";
            case Predicate::LT: return "lt";
            case Predicate::LE: return "le";
            case Predicate::GT: return "gt";
            case Predicate::GE: return "ge";
        }
        return "unknown";
    }

    void CmpOp::print(std::ostream& os, unsigned indent) const
    {
        std::string indentStr(indent, ' ');
        os << indentStr << "%" << this << " = \"cmp\"(" 
           << getLhs()->toString() << ", " << getRhs()->toString() 
           << ") {pred = \"" << getPredicateString() << "\"} : i1\n";
    }

    // ============================================================
    // AllocOp 实现
    // ============================================================

    std::shared_ptr<BufferType> AllocOp::getBufferType() const
    {
        return std::dynamic_pointer_cast<BufferType>(resultTypes_[0]);
    }

    void AllocOp::print(std::ostream& os, unsigned indent) const
    {
        std::string indentStr(indent, ' ');
        os << indentStr << "%" << this << " = \"alloc\"(";
        for (size_t i = 0; i < getNumDims(); ++i)
        {
            if (i > 0) os << ", ";
            os << getOperand(i)->toString();
        }
        os << ") : " << resultTypes_[0]->toString() << "\n";
    }

    // ============================================================
    // LoadOp 实现
    // ============================================================

    std::vector<std::shared_ptr<Value>> LoadOp::getIndices() const
    {
        std::vector<std::shared_ptr<Value>> indices;
        for (size_t i = 1; i < operands_.size(); ++i)
        {
            indices.push_back(operands_[i]);
        }
        return indices;
    }

    void LoadOp::print(std::ostream& os, unsigned indent) const
    {
        std::string indentStr(indent, ' ');
        os << indentStr << "%" << this << " = \"load\"(" << getBuffer()->toString();
        for (const auto& idx : getIndices())
        {
            os << ", " << idx->toString();
        }
        os << ") : " << resultTypes_[0]->toString() << "\n";
    }

    // ============================================================
    // StoreOp 实现
    // ============================================================

    std::vector<std::shared_ptr<Value>> StoreOp::getIndices() const
    {
        std::vector<std::shared_ptr<Value>> indices;
        for (size_t i = 2; i < operands_.size(); ++i)
        {
            indices.push_back(operands_[i]);
        }
        return indices;
    }

    void StoreOp::print(std::ostream& os, unsigned indent) const
    {
        std::string indentStr(indent, ' ');
        os << indentStr << "\"store\"(" << getValue()->toString() 
           << ", " << getBuffer()->toString();
        for (const auto& idx : getIndices())
        {
            os << ", " << idx->toString();
        }
        os << ")\n";
    }

    // ============================================================
    // ConstantOp 实现
    // ============================================================

    int64_t ConstantOp::getIntValue() const
    {
        auto* attr = dynamic_cast<IntegerAttr*>(getAttribute("value"));
        return attr ? attr->getValue() : 0;
    }

    double ConstantOp::getFloatValue() const
    {
        auto* attr = dynamic_cast<FloatAttr*>(getAttribute("value"));
        return attr ? attr->getValue() : 0.0;
    }

    bool ConstantOp::isInteger() const
    {
        return dynamic_cast<IntegerAttr*>(getAttribute("value")) != nullptr;
    }

    void ConstantOp::print(std::ostream& os, unsigned indent) const
    {
        std::string indentStr(indent, ' ');
        os << indentStr << "%" << this << " = \"constant\"(" 
           << getAttribute("value")->toString() 
           << ") : " << resultTypes_[0]->toString() << "\n";
    }

    // ============================================================
    // BranchOp 实现
    // ============================================================

    Block* BranchOp::getTarget() const
    {
        auto* attr = dynamic_cast<IntegerAttr*>(getAttribute("target"));
        return attr ? reinterpret_cast<Block*>(attr->getValue()) : nullptr;
    }

    std::vector<std::shared_ptr<Value>> BranchOp::getArguments() const
    {
        return operands_;
    }

    void BranchOp::print(std::ostream& os, unsigned indent) const
    {
        std::string indentStr(indent, ' ');
        os << indentStr << "\"br\"(";
        for (size_t i = 0; i < operands_.size(); ++i)
        {
            if (i > 0) os << ", ";
            os << operands_[i]->toString();
        }
        os << ") {target = \"block_" << getTarget() << "\"}\n";
    }

    // ============================================================
    // CondBranchOp 实现
    // ============================================================

    Block* CondBranchOp::getTrueBlock() const
    {
        auto* attr = dynamic_cast<IntegerAttr*>(getAttribute("true_target"));
        return attr ? reinterpret_cast<Block*>(attr->getValue()) : nullptr;
    }

    Block* CondBranchOp::getFalseBlock() const
    {
        auto* attr = dynamic_cast<IntegerAttr*>(getAttribute("false_target"));
        return attr ? reinterpret_cast<Block*>(attr->getValue()) : nullptr;
    }

    void CondBranchOp::print(std::ostream& os, unsigned indent) const
    {
        std::string indentStr(indent, ' ');
        os << indentStr << "\"cond_br\"(" << getCondition()->toString() 
           << ") {true = \"block_" << getTrueBlock() 
           << "\", false = \"block_" << getFalseBlock() << "\"}\n";
    }

    // ============================================================
    // ReturnOp 实现
    // ============================================================

    void ReturnOp::print(std::ostream& os, unsigned indent) const
    {
        std::string indentStr(indent, ' ');
        os << indentStr << "\"return\"";
        if (hasValue())
        {
            os << "(" << getValue()->toString() << ")";
        }
        os << "\n";
    }

    // ============================================================
    // FuncOp 实现
    // ============================================================

    std::string FuncOp::getSymName() const
    {
        auto* attr = dynamic_cast<StringAttr*>(getAttribute("sym_name"));
        return attr ? attr->getValue() : "";
    }

    std::shared_ptr<FunctionType> FuncOp::getFunctionType() const
    {
        return std::dynamic_pointer_cast<FunctionType>(resultTypes_[0]);
    }

    Region* FuncOp::getBody() const
    {
        return getRegion(0);
    }

    Block* FuncOp::getEntryBlock() const
    {
        auto* body = getBody();
        return body ? body->getEntryBlock() : nullptr;
    }

    void FuncOp::print(std::ostream& os, unsigned indent) const
    {
        std::string indentStr(indent, ' ');
        os << indentStr << "\"func\"(@" << getSymName() << ") : " 
           << getFunctionType()->toString();
        if (auto* body = getBody())
        {
            os << " {\n";
            body->print(os, indent + 2);
            os << indentStr << "}\n";
        }
        else
        {
            os << "\n";
        }
    }

    // ============================================================
    // ParallelForOp 实现
    // ============================================================

    void ParallelForOp::print(std::ostream& os, unsigned indent) const
    {
        std::string indentStr(indent, ' ');
        os << indentStr << "\"hsc.parallel_for\"(" 
           << getLowerBound()->toString() << ", " 
           << getUpperBound()->toString() << ", "
           << getStep()->toString() << ") {\n";
        if (getBody())
        {
            getBody()->print(os, indent + 2);
        }
        os << indentStr << "}\n";
    }

    // ============================================================
    // ReduceOp 实现
    // ============================================================

    std::string ReduceOp::getReductionKindString() const
    {
        switch (kind_)
        {
            case ReductionKind::SUM: return "sum";
            case ReductionKind::PROD: return "prod";
            case ReductionKind::MIN: return "min";
            case ReductionKind::MAX: return "max";
            case ReductionKind::AND: return "and";
            case ReductionKind::OR: return "or";
            case ReductionKind::XOR: return "xor";
        }
        return "unknown";
    }

    void ReduceOp::print(std::ostream& os, unsigned indent) const
    {
        std::string indentStr(indent, ' ');
        os << indentStr << "%" << this << " = \"hsc.reduce\"(" 
           << getInput()->toString() << ", " << getInitValue()->toString()
           << ") {kind = \"" << getReductionKindString() << "\"} : "
           << (resultTypes_.empty() ? "none" : resultTypes_[0]->toString()) << "\n";
    }

    // ============================================================
    // SpawnOp 实现
    // ============================================================

    std::string SpawnOp::getTaskName() const
    {
        auto* attr = dynamic_cast<StringAttr*>(getAttribute("task"));
        return attr ? attr->getValue() : "";
    }

    std::vector<std::shared_ptr<Value>> SpawnOp::getArguments() const
    {
        return std::vector<std::shared_ptr<Value>>(operands_.begin() + 1, operands_.end());
    }

    bool SpawnOp::isAwait() const
    {
        auto* attr = dynamic_cast<BoolAttr*>(getAttribute("await"));
        return attr ? attr->getValue() : false;
    }

    void SpawnOp::print(std::ostream& os, unsigned indent) const
    {
        std::string indentStr(indent, ' ');
        os << indentStr << "%" << this << " = \"hsc.spawn\"(" << getDevice()->toString();
        for (const auto& arg : getArguments())
        {
            os << ", " << arg->toString();
        }
        os << ") {task = @" << getTaskName() 
           << ", await = " << (isAwait() ? "true" : "false") << "}\n";
    }

    // ============================================================
    // SyncOp 实现
    // ============================================================

    void SyncOp::print(std::ostream& os, unsigned indent) const
    {
        std::string indentStr(indent, ' ');
        os << indentStr << "\"hsc.sync\"(";
        auto dev = getDevice();
        if (dev)
        {
            os << dev->toString();
        }
        os << ")\n";
    }

    // ============================================================
    // MoveToOp 实现
    // ============================================================

    void MoveToOp::print(std::ostream& os, unsigned indent) const
    {
        std::string indentStr(indent, ' ');
        os << indentStr << "%" << this << " = \"hsc.move_to\"(" 
           << getBuffer()->toString() << ", " << getDevice()->toString()
           << ") : " << resultTypes_[0]->toString() << "\n";
    }

    // ============================================================
    // PlaceOnOp 实现
    // ============================================================

    void PlaceOnOp::print(std::ostream& os, unsigned indent) const
    {
        std::string indentStr(indent, ' ');
        os << indentStr << "%" << this << " = \"hsc.place_on\"(" 
           << getBuffer()->toString() << ", " << getDevice()->toString()
           << ") : " << resultTypes_[0]->toString() << "\n";
    }

    // ============================================================
    // TaskOp 实现
    // ============================================================

    std::string TaskOp::getSymName() const
    {
        auto* attr = dynamic_cast<StringAttr*>(getAttribute("sym_name"));
        return attr ? attr->getValue() : "";
    }

    std::shared_ptr<FunctionType> TaskOp::getFunctionType() const
    {
        return std::dynamic_pointer_cast<FunctionType>(resultTypes_[0]);
    }

    Region* TaskOp::getBody() const
    {
        return getRegion(0);
    }

    Block* TaskOp::getEntryBlock() const
    {
        auto* body = getBody();
        return body ? body->getEntryBlock() : nullptr;
    }

    void TaskOp::print(std::ostream& os, unsigned indent) const
    {
        std::string indentStr(indent, ' ');
        os << indentStr << "\"hsc.task\"(@" << getSymName() << ") : " 
           << getFunctionType()->toString();
        if (auto* body = getBody())
        {
            os << " {\n";
            body->print(os, indent + 2);
            os << indentStr << "}\n";
        }
        else
        {
            os << "\n";
        }
    }

    // ============================================================
    // Value 实现
    // ============================================================

    std::string OpResult::toString() const
    {
        return "%result_" + std::to_string(reinterpret_cast<uintptr_t>(op_)) + "_" + std::to_string(index_);
    }

    std::string BlockArgument::toString() const
    {
        return "%arg" + std::to_string(index_);
    }

    // ============================================================
    // Block 实现
    // ============================================================

    void Block::addOperation(std::unique_ptr<Operation> op)
    {
        operations_.push_back(std::move(op));
    }

    void Block::insertOperation(size_t pos, std::unique_ptr<Operation> op)
    {
        if (pos >= operations_.size())
        {
            operations_.push_back(std::move(op));
        }
        else
        {
            operations_.insert(operations_.begin() + pos, std::move(op));
        }
    }

    Operation* Block::getOperation(size_t index) const
    {
        return index < operations_.size() ? operations_[index].get() : nullptr;
    }

    Operation* Block::getTerminator() const
    {
        if (operations_.empty()) return nullptr;
        auto* lastOp = operations_.back().get();
        // 检查是否是终结符操作
        if (dynamic_cast<ReturnOp*>(lastOp) || 
            dynamic_cast<BranchOp*>(lastOp) || 
            dynamic_cast<CondBranchOp*>(lastOp))
        {
            return lastOp;
        }
        return nullptr;
    }

    void Block::addArgument(std::shared_ptr<Type> type)
    {
        auto arg = std::make_shared<BlockArgument>(std::move(type), this, arguments_.size());
        arguments_.push_back(std::move(arg));
    }

    std::shared_ptr<BlockArgument> Block::getArgument(size_t index) const
    {
        return index < arguments_.size() ? arguments_[index] : nullptr;
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
        }
        else
        {
            os << indentStr << "block:\n";
        }
        for (const auto& op : operations_)
        {
            op->print(os, indent + 2);
        }
    }

    // ============================================================
    // Region 实现
    // ============================================================

    void Region::addBlock(std::unique_ptr<Block> block)
    {
        blocks_.push_back(std::move(block));
    }

    Block* Region::insertBlock(size_t pos, std::unique_ptr<Block> block)
    {
        if (pos >= blocks_.size())
        {
            blocks_.push_back(std::move(block));
            return blocks_.back().get();
        }
        else
        {
            auto it = blocks_.insert(blocks_.begin() + pos, std::move(block));
            return it->get();
        }
    }

    Block* Region::getBlock(size_t index) const
    {
        return index < blocks_.size() ? blocks_[index].get() : nullptr;
    }

    Block* Region::getEntryBlock() const
    {
        return blocks_.empty() ? nullptr : blocks_.front().get();
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

    // ============================================================
    // Module 实现
    // ============================================================

    void Module::addOperation(std::unique_ptr<Operation> op)
    {
        // 更新符号表
        if (auto* funcOp = dynamic_cast<FuncOp*>(op.get()))
        {
            symbolTable_[funcOp->getSymName()] = op.get();
        }
        else if (auto* taskOp = dynamic_cast<TaskOp*>(op.get()))
        {
            symbolTable_[taskOp->getSymName()] = op.get();
        }
        operations_.push_back(std::move(op));
    }

    void Module::insertOperation(size_t pos, std::unique_ptr<Operation> op)
    {
        // 更新符号表
        if (auto* funcOp = dynamic_cast<FuncOp*>(op.get()))
        {
            symbolTable_[funcOp->getSymName()] = op.get();
        }
        else if (auto* taskOp = dynamic_cast<TaskOp*>(op.get()))
        {
            symbolTable_[taskOp->getSymName()] = op.get();
        }

        if (pos >= operations_.size())
        {
            operations_.push_back(std::move(op));
        }
        else
        {
            operations_.insert(operations_.begin() + pos, std::move(op));
        }
    }

    Operation* Module::getOperation(size_t index) const
    {
        return index < operations_.size() ? operations_[index].get() : nullptr;
    }

    FuncOp* Module::lookupFunction(const std::string& name) const
    {
        auto it = symbolTable_.find(name);
        if (it != symbolTable_.end())
        {
            return dynamic_cast<FuncOp*>(it->second);
        }
        return nullptr;
    }

    TaskOp* Module::lookupTask(const std::string& name) const
    {
        auto it = symbolTable_.find(name);
        if (it != symbolTable_.end())
        {
            return dynamic_cast<TaskOp*>(it->second);
        }
        return nullptr;
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

} // namespace hscir
