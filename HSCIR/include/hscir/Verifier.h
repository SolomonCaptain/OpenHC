#ifndef HSCIR_VERIFIER_H
#define HSCIR_VERIFIER_H

#include "Operations.h"
#include "Types.h"
#include <string>
#include <vector>
#include <memory>
#include <functional>

namespace hscir
{

    // ============================================================
    // 验证结果
    // ============================================================

    /// 验证错误级别
    enum class VerificationLevel
    {
        Error,      // 严重错误，IR 无效
        Warning,    // 警告，可能有问题但不影响正确性
        Note        // 提示信息
    };

    /// 验证诊断信息
    struct VerificationDiagnostic
    {
        VerificationLevel level;
        std::string message;
        std::string operationName;  // 相关操作名称
        int line = -1;              // 行号（如果可用）

        VerificationDiagnostic(VerificationLevel lvl, const std::string& msg,
                               const std::string& opName = "", int ln = -1)
            : level(lvl), message(msg), operationName(opName), line(ln) {}

        std::string toString() const;
    };

    /// 验证结果
    class VerificationResult
    {
        public:
            VerificationResult() = default;

            void addError(const std::string& msg, const std::string& opName = "", int line = -1);
            void addWarning(const std::string& msg, const std::string& opName = "", int line = -1);
            void addNote(const std::string& msg, const std::string& opName = "", int line = -1);

            bool success() const { return errors_.empty(); }
            bool hasWarnings() const { return !warnings_.empty(); }

            const std::vector<VerificationDiagnostic>& getErrors() const { return errors_; }
            const std::vector<VerificationDiagnostic>& getWarnings() const { return warnings_; }
            const std::vector<VerificationDiagnostic>& getNotes() const { return notes_; }

            /// 合并另一个验证结果
            void merge(const VerificationResult& other);

            /// 输出所有诊断信息
            void print() const;

            /// 获取诊断信息字符串
            std::string diagnosticsToString() const;

        private:
            std::vector<VerificationDiagnostic> errors_;
            std::vector<VerificationDiagnostic> warnings_;
            std::vector<VerificationDiagnostic> notes_;
    };

    // ============================================================
    // 验证器基类
    // ============================================================

    /// 验证器接口
    class Verifier
    {
        public:
            virtual ~Verifier() = default;

            /// 验证操作
            virtual VerificationResult verify(Operation* op) = 0;

            /// 验证模块
            virtual VerificationResult verifyModule(Module* module);
    };

    // ============================================================
    // 类型验证器
    // ============================================================

    /// 类型验证器
    class TypeVerifier : public Verifier
    {
        public:
            VerificationResult verify(Operation* op) override;

        private:
            /// 验证操作数类型
            bool verifyOperandTypes(Operation* op, VerificationResult& result);

            /// 验证结果类型
            bool verifyResultTypes(Operation* op, VerificationResult& result);

            /// 验证类型一致性
            bool verifyTypeConsistency(Operation* op, VerificationResult& result);
    };

    // ============================================================
    // 操作验证器
    // ============================================================

    /// 操作验证器
    class OperationVerifier : public Verifier
    {
        public:
            VerificationResult verify(Operation* op) override;

        private:
            /// 验证操作名称
            bool verifyOperationName(Operation* op, VerificationResult& result);

            /// 验证操作数
            bool verifyOperands(Operation* op, VerificationResult& result);

            /// 验证属性
            bool verifyAttributes(Operation* op, VerificationResult& result);

            /// 验证区域
            bool verifyRegions(Operation* op, VerificationResult& result);

            /// 验证特定操作
            bool verifySpecificOperation(Operation* op, VerificationResult& result);
    };

    // ============================================================
    // 控制流验证器
    // ============================================================

    /// 控制流验证器
    class ControlFlowVerifier : public Verifier
    {
        public:
            VerificationResult verify(Operation* op) override;

        private:
            /// 验证基本块
            bool verifyBlock(Block* block, VerificationResult& result);

            /// 验证终止操作
            bool verifyTerminator(Operation* op, VerificationResult& result);

            /// 验证支配关系
            bool verifyDominance(Block* block, VerificationResult& result);

            /// 验证后支配关系
            bool verifyPostDominance(Block* block, VerificationResult& result);

            /// 验证控制流完整性
            bool verifyControlFlowIntegrity(Region* region, VerificationResult& result);
    };

    // ============================================================
    // 并行操作验证器
    // ============================================================

    /// 并行操作验证器
    class ParallelVerifier : public Verifier
    {
        public:
            VerificationResult verify(Operation* op) override;

        private:
            /// 验证并行循环
            bool verifyParallelFor(ParallelForOp* op, VerificationResult& result);

            /// 验证归约操作
            bool verifyReduce(ReduceOp* op, VerificationResult& result);

            /// 验证循环独立性
            bool verifyLoopIndependence(ParallelForOp* op, VerificationResult& result);
    };

    // ============================================================
    // 设备操作验证器
    // ============================================================

    /// 设备操作验证器
    class DeviceVerifier : public Verifier
    {
        public:
            VerificationResult verify(Operation* op) override;

        private:
            /// 验证设备放置
            bool verifyDevicePlacement(Operation* op, VerificationResult& result);

            /// 验证数据迁移
            bool verifyDataMovement(Operation* op, VerificationResult& result);

            /// 验证设备同步
            bool verifyDeviceSync(Operation* op, VerificationResult& result);
    };

    // ============================================================
    // 综合验证器
    // ============================================================

    /// 综合验证器
    class ComprehensiveVerifier : public Verifier
    {
        public:
            ComprehensiveVerifier();

            VerificationResult verify(Operation* op) override;

            /// 添加自定义验证器
            void addVerifier(std::unique_ptr<Verifier> verifier);

            /// 设置是否启用特定验证
            void setTypeVerification(bool enabled) { typeVerification_ = enabled; }
            void setOperationVerification(bool enabled) { operationVerification_ = enabled; }
            void setControlFlowVerification(bool enabled) { controlFlowVerification_ = enabled; }
            void setParallelVerification(bool enabled) { parallelVerification_ = enabled; }
            void setDeviceVerification(bool enabled) { deviceVerification_ = enabled; }

        private:
            std::vector<std::unique_ptr<Verifier>> verifiers_;
            bool typeVerification_ = true;
            bool operationVerification_ = true;
            bool controlFlowVerification_ = true;
            bool parallelVerification_ = true;
            bool deviceVerification_ = true;
    };

    // ============================================================
    // 模块验证器
    // ============================================================

    /// 模块验证器
    class ModuleVerifier
    {
        public:
            /// 验证整个模块
            static VerificationResult verify(Module* module);

            /// 验证模块中的所有函数
            static VerificationResult verifyFunctions(Module* module);

            /// 验证模块中的所有任务
            static VerificationResult verifyTasks(Module* module);

            /// 验证模块中的符号表
            static VerificationResult verifySymbolTable(Module* module);
    };

    // ============================================================
    // 辅助验证函数
    // ============================================================

    /// 验证类型是否有效
    bool isValidType(std::shared_ptr<Type> type);

    /// 验证类型是否兼容
    bool areTypesCompatible(std::shared_ptr<Type> lhs, std::shared_ptr<Type> rhs);

    /// 验证 Buffer 类型形状是否有效
    bool isValidBufferShape(const std::vector<int64_t>& shape);

    /// 验证函数签名
    bool isValidFunctionSignature(std::shared_ptr<FunctionType> funcType);

} // namespace hscir

#endif // HSCIR_VERIFIER_H
