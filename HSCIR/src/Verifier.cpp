#include "hscir/Verifier.h"
#include "hscir/Builder.h"
#include <iostream>
#include <sstream>
#include <algorithm>
#include <unordered_set>

namespace hscir
{

    // ============================================================
    // VerificationDiagnostic 实现
    // ============================================================

    std::string VerificationDiagnostic::toString() const
    {
        std::ostringstream oss;
        switch (level)
        {
            case VerificationLevel::Error:
                oss << "error: ";
                break;
            case VerificationLevel::Warning:
                oss << "warning: ";
                break;
            case VerificationLevel::Note:
                oss << "note: ";
                break;
        }

        if (!operationName.empty())
        {
            oss << "[" << operationName << "] ";
        }

        oss << message;

        if (line >= 0)
        {
            oss << " (line " << line << ")";
        }

        return oss.str();
    }

    // ============================================================
    // VerificationResult 实现
    // ============================================================

    void VerificationResult::addError(const std::string& msg, const std::string& opName, int line)
    {
        errors_.emplace_back(VerificationLevel::Error, msg, opName, line);
    }

    void VerificationResult::addWarning(const std::string& msg, const std::string& opName, int line)
    {
        warnings_.emplace_back(VerificationLevel::Warning, msg, opName, line);
    }

    void VerificationResult::addNote(const std::string& msg, const std::string& opName, int line)
    {
        notes_.emplace_back(VerificationLevel::Note, msg, opName, line);
    }

    void VerificationResult::merge(const VerificationResult& other)
    {
        errors_.insert(errors_.end(), other.errors_.begin(), other.errors_.end());
        warnings_.insert(warnings_.end(), other.warnings_.begin(), other.warnings_.end());
        notes_.insert(notes_.end(), other.notes_.begin(), other.notes_.end());
    }

    void VerificationResult::print() const
    {
        std::cout << diagnosticsToString() << std::endl;
    }

    std::string VerificationResult::diagnosticsToString() const
    {
        std::ostringstream oss;

        for (const auto& diag : errors_)
        {
            oss << diag.toString() << "\n";
        }

        for (const auto& diag : warnings_)
        {
            oss << diag.toString() << "\n";
        }

        for (const auto& diag : notes_)
        {
            oss << diag.toString() << "\n";
        }

        return oss.str();
    }

    // ============================================================
    // Verifier 基类实现
    // ============================================================

    VerificationResult Verifier::verifyModule(Module* module)
    {
        VerificationResult result;

        if (!module)
        {
            result.addError("Module is null");
            return result;
        }

        // 验证模块中的所有操作
        for (auto& op : module->getOperations())
        {
            result.merge(verify(op.get()));
        }

        return result;
    }

    // ============================================================
    // TypeVerifier 实现
    // ============================================================

    VerificationResult TypeVerifier::verify(Operation* op)
    {
        VerificationResult result;

        if (!op)
        {
            result.addError("Operation is null");
            return result;
        }

        verifyOperandTypes(op, result);
        verifyResultTypes(op, result);
        verifyTypeConsistency(op, result);

        return result;
    }

    bool TypeVerifier::verifyOperandTypes(Operation* op, VerificationResult& result)
    {
        const auto& operands = op->getOperands();

        for (size_t i = 0; i < operands.size(); ++i)
        {
            auto value = operands[i];
            if (!value)
            {
                result.addError("Operand " + std::to_string(i) + " is null", op->getName());
                continue;
            }

            auto type = value->getType();
            if (!type)
            {
                result.addError("Operand " + std::to_string(i) + " has null type", op->getName());
                continue;
            }

            if (!isValidType(type))
            {
                result.addError("Operand " + std::to_string(i) + " has invalid type: " + type->toString(),
                               op->getName());
            }
        }

        return result.success();
    }

    bool TypeVerifier::verifyResultTypes(Operation* op, VerificationResult& result)
    {
        const auto& resultTypes = op->getResultTypes();

        for (size_t i = 0; i < resultTypes.size(); ++i)
        {
            auto type = resultTypes[i];
            if (!type)
            {
                result.addError("Result " + std::to_string(i) + " has null type", op->getName());
                continue;
            }

            if (!isValidType(type))
            {
                result.addError("Result " + std::to_string(i) + " has invalid type: " + type->toString(),
                               op->getName());
            }
        }

        return result.success();
    }

    bool TypeVerifier::verifyTypeConsistency(Operation* op, VerificationResult& result)
    {
        const std::string& opName = op->getName();

        // 验证特定操作的类型一致性
        if (opName == "add" || opName == "sub" || opName == "mul" || opName == "div" || opName == "mod")
        {
            // 算术操作：操作数类型必须一致
            const auto& operands = op->getOperands();
            if (operands.size() >= 2)
            {
                auto lhsType = operands[0] ? operands[0]->getType() : nullptr;
                auto rhsType = operands[1] ? operands[1]->getType() : nullptr;

                if (lhsType && rhsType && !areTypesCompatible(lhsType, rhsType))
                {
                    result.addError("Type mismatch in arithmetic operation: " +
                                   lhsType->toString() + " vs " + rhsType->toString(), opName);
                }
            }
        }
        else if (opName == "cmp")
        {
            // 比较操作：操作数类型必须一致，结果类型必须是 i1
            const auto& operands = op->getOperands();
            if (operands.size() >= 2)
            {
                auto lhsType = operands[0] ? operands[0]->getType() : nullptr;
                auto rhsType = operands[1] ? operands[1]->getType() : nullptr;

                if (lhsType && rhsType && !areTypesCompatible(lhsType, rhsType))
                {
                    result.addError("Type mismatch in comparison operation: " +
                                   lhsType->toString() + " vs " + rhsType->toString(), opName);
                }
            }

            // 结果类型应该是 i1
            const auto& resultTypes = op->getResultTypes();
            if (!resultTypes.empty() && resultTypes[0])
            {
                if (resultTypes[0]->getKind() != Type::Kind::Integer ||
                    std::dynamic_pointer_cast<IntegerType>(resultTypes[0])->getWidth() != 1)
                {
                    result.addWarning("Comparison result type should be i1, got: " +
                                     resultTypes[0]->toString(), opName);
                }
            }
        }
        else if (opName == "load")
        {
            // 加载操作：第一个操作数应该是 Buffer 类型
            const auto& operands = op->getOperands();
            if (!operands.empty() && operands[0])
            {
                auto bufType = operands[0]->getType();
                if (bufType && bufType->getKind() != Type::Kind::Buffer)
                {
                    result.addError("Load operation requires Buffer operand, got: " +
                                   bufType->toString(), opName);
                }
            }
        }
        else if (opName == "store")
        {
            // 存储操作：第一个操作数是值，第二个是 Buffer
            const auto& operands = op->getOperands();
            if (operands.size() >= 2 && operands[1])
            {
                auto bufType = operands[1]->getType();
                if (bufType && bufType->getKind() != Type::Kind::Buffer)
                {
                    result.addError("Store operation requires Buffer operand, got: " +
                                   bufType->toString(), opName);
                }
            }
        }

        return result.success();
    }

    // ============================================================
    // OperationVerifier 实现
    // ============================================================

    VerificationResult OperationVerifier::verify(Operation* op)
    {
        VerificationResult result;

        if (!op)
        {
            result.addError("Operation is null");
            return result;
        }

        verifyOperationName(op, result);
        verifyOperands(op, result);
        verifyAttributes(op, result);
        verifyRegions(op, result);
        verifySpecificOperation(op, result);

        return result;
    }

    bool OperationVerifier::verifyOperationName(Operation* op, VerificationResult& result)
    {
        const std::string& name = op->getName();

        if (name.empty())
        {
            result.addError("Operation has empty name");
            return false;
        }

        // 检查操作名称是否在允许列表中
        static const std::unordered_set<std::string> validOps = {
            "add", "sub", "mul", "div", "mod", "cmp",
            "alloc", "load", "store",
            "br", "cond_br", "return",
            "parallel_for", "reduce",
            "spawn", "sync", "move_to", "place_on",
            "constant", "func", "task"
        };

        if (validOps.find(name) == validOps.end())
        {
            result.addWarning("Unknown operation: " + name);
        }

        return true;
    }

    bool OperationVerifier::verifyOperands(Operation* op, VerificationResult& result)
    {
        const auto& operands = op->getOperands();

        // 检查操作数数量是否符合操作要求
        const std::string& name = op->getName();

        if (name == "add" || name == "sub" || name == "mul" || name == "div" || name == "mod" || name == "cmp")
        {
            if (operands.size() != 2)
            {
                result.addError("Arithmetic operation requires exactly 2 operands, got " +
                               std::to_string(operands.size()), name);
            }
        }
        else if (name == "br")
        {
            // 无条件跳转可以有块参数
        }
        else if (name == "cond_br")
        {
            if (operands.size() < 1)
            {
                result.addError("Conditional branch requires at least 1 operand (condition)", name);
            }
        }
        else if (name == "return")
        {
            // return 可以有 0 或 1 个操作数
            if (operands.size() > 1)
            {
                result.addWarning("Return operation should have at most 1 operand", name);
            }
        }

        return result.success();
    }

    bool OperationVerifier::verifyAttributes(Operation* op, VerificationResult& result)
    {
        const auto& attrs = op->getAttributes();

        // 检查特定操作需要的属性
        const std::string& name = op->getName();

        if (name == "parallel_for")
        {
            if (attrs.find("lower_bound") == attrs.end())
            {
                result.addWarning("parallel_for missing 'lower_bound' attribute", name);
            }
            if (attrs.find("upper_bound") == attrs.end())
            {
                result.addWarning("parallel_for missing 'upper_bound' attribute", name);
            }
        }
        else if (name == "reduce")
        {
            if (attrs.find("kind") == attrs.end())
            {
                result.addError("reduce operation requires 'kind' attribute", name);
            }
        }
        else if (name == "cmp")
        {
            if (attrs.find("predicate") == attrs.end())
            {
                result.addError("cmp operation requires 'predicate' attribute", name);
            }
        }

        return result.success();
    }

    bool OperationVerifier::verifyRegions(Operation* op, VerificationResult& result)
    {
        const auto& regions = op->getRegions();
        const std::string& name = op->getName();

        // 检查区域数量
        if (name == "func" || name == "task")
        {
            if (regions.size() != 1)
            {
                result.addError("Function/task operation must have exactly 1 region, got " +
                               std::to_string(regions.size()), name);
            }
        }
        else if (name == "parallel_for")
        {
            if (regions.size() != 1)
            {
                result.addError("parallel_for must have exactly 1 region for loop body, got " +
                               std::to_string(regions.size()), name);
            }
        }

        // 验证每个区域
        for (size_t i = 0; i < regions.size(); ++i)
        {
            if (!regions[i])
            {
                result.addError("Region " + std::to_string(i) + " is null", name);
                continue;
            }

            if (regions[i]->empty())
            {
                result.addWarning("Region " + std::to_string(i) + " is empty", name);
            }
        }

        return result.success();
    }

    bool OperationVerifier::verifySpecificOperation(Operation* op, VerificationResult& result)
    {
        const std::string& name = op->getName();

        if (name == "constant")
        {
            // 常量操作必须有值属性
            const auto& attrs = op->getAttributes();
            if (attrs.find("value") == attrs.end())
            {
                result.addError("Constant operation requires 'value' attribute", name);
            }
        }
        else if (name == "alloc")
        {
            // 分配操作必须有元素类型
            const auto& resultTypes = op->getResultTypes();
            if (resultTypes.empty() || !resultTypes[0])
            {
                result.addError("Alloc operation must have a result type", name);
            }
        }

        return result.success();
    }

    // ============================================================
    // ControlFlowVerifier 实现
    // ============================================================

    VerificationResult ControlFlowVerifier::verify(Operation* op)
    {
        VerificationResult result;

        if (!op)
        {
            result.addError("Operation is null");
            return result;
        }

        // 验证区域中的控制流
        for (auto& region : op->getRegions())
        {
            if (region)
            {
                for (auto& block : region->getBlocks())
                {
                    verifyBlock(block.get(), result);
                }
                verifyControlFlowIntegrity(region.get(), result);
            }
        }

        return result;
    }

    bool ControlFlowVerifier::verifyBlock(Block* block, VerificationResult& result)
    {
        if (!block)
        {
            result.addError("Block is null");
            return false;
        }

        const auto& operations = block->getOperations();

        if (operations.empty())
        {
            result.addWarning("Empty block");
            return true;
        }

        // 检查块是否以终结符结束
        auto& lastOp = operations.back();
        if (lastOp)
        {
            verifyTerminator(lastOp.get(), result);
        }

        return result.success();
    }

    bool ControlFlowVerifier::verifyTerminator(Operation* op, VerificationResult& result)
    {
        if (!op)
        {
            return true;
        }

        const std::string& name = op->getName();

        // 检查是否是有效的终结符
        static const std::unordered_set<std::string> terminators = {
            "br", "cond_br", "return"
        };

        if (terminators.find(name) == terminators.end())
        {
            result.addError("Block must end with a terminator, got: " + name);
            return false;
        }

        return true;
    }

    bool ControlFlowVerifier::verifyDominance(Block* block, VerificationResult& result)
    {
        // 简化的支配关系验证
        // 实际实现需要构建支配树
        return true;
    }

    bool ControlFlowVerifier::verifyPostDominance(Block* block, VerificationResult& result)
    {
        // 简化的后支配关系验证
        return true;
    }

    bool ControlFlowVerifier::verifyControlFlowIntegrity(Region* region, VerificationResult& result)
    {
        if (!region)
        {
            return true;
        }

        const auto& blocks = region->getBlocks();

        if (blocks.empty())
        {
            return true;
        }

        // 检查入口块
        auto& entryBlock = blocks.front();
        if (!entryBlock)
        {
            result.addError("Entry block is null");
            return false;
        }

        // 检查所有块是否可达（简化版本）
        // 实际实现需要构建控制流图并执行可达性分析

        return true;
    }

    // ============================================================
    // ParallelVerifier 实现
    // ============================================================

    VerificationResult ParallelVerifier::verify(Operation* op)
    {
        VerificationResult result;

        if (!op)
        {
            result.addError("Operation is null");
            return result;
        }

        const std::string& name = op->getName();

        if (name == "parallel_for")
        {
            // 验证并行循环
            // 注意：dynamic_cast 在这里可能失败，我们使用保守的验证方式
            verifyParallelFor(nullptr, result);
        }
        else if (name == "reduce")
        {
            verifyReduce(nullptr, result);
        }

        return result;
    }

    bool ParallelVerifier::verifyParallelFor(ParallelForOp* op, VerificationResult& result)
    {
        // 验证循环边界
        // 验证步长
        // 验证循环体
        return true;
    }

    bool ParallelVerifier::verifyReduce(ReduceOp* op, VerificationResult& result)
    {
        // 验证归约类型
        // 验证初始值
        return true;
    }

    bool ParallelVerifier::verifyLoopIndependence(ParallelForOp* op, VerificationResult& result)
    {
        // 验证循环独立性（用于并行化正确性）
        return true;
    }

    // ============================================================
    // DeviceVerifier 实现
    // ============================================================

    VerificationResult DeviceVerifier::verify(Operation* op)
    {
        VerificationResult result;

        if (!op)
        {
            result.addError("Operation is null");
            return result;
        }

        const std::string& name = op->getName();

        if (name == "spawn" || name == "move_to" || name == "place_on")
        {
            verifyDevicePlacement(op, result);
        }
        else if (name == "sync")
        {
            verifyDeviceSync(op, result);
        }

        return result;
    }

    bool DeviceVerifier::verifyDevicePlacement(Operation* op, VerificationResult& result)
    {
        // 验证设备类型是否有效
        const auto& attrs = op->getAttributes();

        if (attrs.find("device") != attrs.end())
        {
            const auto& deviceAttrPtr = attrs.at("device");
            if (auto strAttr = dynamic_cast<StringAttr*>(deviceAttrPtr.get()))
            {
                const std::string& device = strAttr->getValue();
                static const std::unordered_set<std::string> validDevices = {
                    "CPU", "GPU", "NPU", "FPGA", "Host"
                };

                if (validDevices.find(device) == validDevices.end())
                {
                    result.addWarning("Unknown device type: " + device, op->getName());
                }
            }
        }

        return true;
    }

    bool DeviceVerifier::verifyDataMovement(Operation* op, VerificationResult& result)
    {
        // 验证数据迁移操作
        return true;
    }

    bool DeviceVerifier::verifyDeviceSync(Operation* op, VerificationResult& result)
    {
        // 验证设备同步操作
        return true;
    }

    // ============================================================
    // ComprehensiveVerifier 实现
    // ============================================================

    ComprehensiveVerifier::ComprehensiveVerifier()
    {
        verifiers_.push_back(std::make_unique<TypeVerifier>());
        verifiers_.push_back(std::make_unique<OperationVerifier>());
        verifiers_.push_back(std::make_unique<ControlFlowVerifier>());
        verifiers_.push_back(std::make_unique<ParallelVerifier>());
        verifiers_.push_back(std::make_unique<DeviceVerifier>());
    }

    VerificationResult ComprehensiveVerifier::verify(Operation* op)
    {
        VerificationResult result;

        for (auto& verifier : verifiers_)
        {
            result.merge(verifier->verify(op));
        }

        return result;
    }

    void ComprehensiveVerifier::addVerifier(std::unique_ptr<Verifier> verifier)
    {
        verifiers_.push_back(std::move(verifier));
    }

    // ============================================================
    // ModuleVerifier 实现
    // ============================================================

    VerificationResult ModuleVerifier::verify(Module* module)
    {
        VerificationResult result;

        if (!module)
        {
            result.addError("Module is null");
            return result;
        }

        // 使用综合验证器验证所有操作
        ComprehensiveVerifier verifier;

        for (auto& op : module->getOperations())
        {
            result.merge(verifier.verify(op.get()));
        }

        // 验证符号表
        result.merge(verifySymbolTable(module));

        return result;
    }

    VerificationResult ModuleVerifier::verifyFunctions(Module* module)
    {
        VerificationResult result;

        if (!module)
        {
            result.addError("Module is null");
            return result;
        }

        for (auto& op : module->getOperations())
        {
            if (op && op->getName() == "func")
            {
                ComprehensiveVerifier verifier;
                result.merge(verifier.verify(op.get()));
            }
        }

        return result;
    }

    VerificationResult ModuleVerifier::verifyTasks(Module* module)
    {
        VerificationResult result;

        if (!module)
        {
            result.addError("Module is null");
            return result;
        }

        for (auto& op : module->getOperations())
        {
            if (op && op->getName() == "task")
            {
                ComprehensiveVerifier verifier;
                result.merge(verifier.verify(op.get()));
            }
        }

        return result;
    }

    VerificationResult ModuleVerifier::verifySymbolTable(Module* module)
    {
        VerificationResult result;

        if (!module)
        {
            result.addError("Module is null");
            return result;
        }

        // 检查符号重复
        std::unordered_set<std::string> symbols;

        for (auto& op : module->getOperations())
        {
            if (!op)
                continue;

            const auto& attrs = op->getAttributes();
            if (attrs.find("name") != attrs.end())
            {
                const auto& nameAttrPtr = attrs.at("name");
                if (auto strAttr = dynamic_cast<StringAttr*>(nameAttrPtr.get()))
                {
                    const std::string& name = strAttr->getValue();
                    if (symbols.find(name) != symbols.end())
                    {
                        result.addError("Duplicate symbol: " + name, op->getName());
                    }
                    else
                    {
                        symbols.insert(name);
                    }
                }
            }
        }

        return result;
    }

    // ============================================================
    // 辅助验证函数实现
    // ============================================================

    bool isValidType(std::shared_ptr<Type> type)
    {
        if (!type)
            return false;

        switch (type->getKind())
        {
            case Type::Kind::Integer:
            {
                auto intType = std::dynamic_pointer_cast<IntegerType>(type);
                unsigned width = intType->getWidth();
                // 支持的整数宽度: 1, 8, 16, 32, 64, 128
                return width == 1 || width == 8 || width == 16 || width == 32 || width == 64 || width == 128;
            }
            case Type::Kind::Float:
            {
                auto floatType = std::dynamic_pointer_cast<FloatType>(type);
                unsigned width = floatType->getWidth();
                // 支持的浮点宽度: 16, 32, 64
                return width == 16 || width == 32 || width == 64;
            }
            case Type::Kind::Buffer:
            {
                auto bufType = std::dynamic_pointer_cast<BufferType>(type);
                return isValidType(bufType->getElementType()) && isValidBufferShape(bufType->getShape());
            }
            case Type::Kind::Function:
            {
                auto funcType = std::dynamic_pointer_cast<FunctionType>(type);
                for (auto& input : funcType->getInputs())
                {
                    if (!isValidType(input))
                        return false;
                }
                for (auto& output : funcType->getOutputs())
                {
                    if (!isValidType(output))
                        return false;
                }
                return true;
            }
            case Type::Kind::None:
                return true;
        }

        return false;
    }

    bool areTypesCompatible(std::shared_ptr<Type> lhs, std::shared_ptr<Type> rhs)
    {
        if (!lhs || !rhs)
            return false;

        if (lhs->getKind() != rhs->getKind())
            return false;

        switch (lhs->getKind())
        {
            case Type::Kind::Integer:
            {
                auto lhsInt = std::dynamic_pointer_cast<IntegerType>(lhs);
                auto rhsInt = std::dynamic_pointer_cast<IntegerType>(rhs);
                return lhsInt->getWidth() == rhsInt->getWidth() &&
                       lhsInt->isSigned() == rhsInt->isSigned();
            }
            case Type::Kind::Float:
            {
                auto lhsFloat = std::dynamic_pointer_cast<FloatType>(lhs);
                auto rhsFloat = std::dynamic_pointer_cast<FloatType>(rhs);
                return lhsFloat->getWidth() == rhsFloat->getWidth();
            }
            case Type::Kind::Buffer:
            {
                auto lhsBuf = std::dynamic_pointer_cast<BufferType>(lhs);
                auto rhsBuf = std::dynamic_pointer_cast<BufferType>(rhs);
                return areTypesCompatible(lhsBuf->getElementType(), rhsBuf->getElementType());
            }
            case Type::Kind::Function:
            case Type::Kind::None:
                return *lhs == *rhs;
        }

        return false;
    }

    bool isValidBufferShape(const std::vector<int64_t>& shape)
    {
        for (auto dim : shape)
        {
            if (dim < 0 && dim != -1)  // -1 表示动态维度
                return false;
        }
        return true;
    }

    bool isValidFunctionSignature(std::shared_ptr<FunctionType> funcType)
    {
        if (!funcType)
            return false;

        // 验证输入类型
        for (auto& input : funcType->getInputs())
        {
            if (!isValidType(input))
                return false;
        }

        // 验证输出类型
        for (auto& output : funcType->getOutputs())
        {
            if (!isValidType(output))
                return false;
        }

        return true;
    }

} // namespace hscir
