# HSCIR MLIR 集成实施方案

> 将现有 HSCIR 模块与 MLIR 框架集成，实现分层 IR 体系和多端特化代码生成。

---

## 一、现状分析

### 1.1 现有 HSCIR 架构

```
HSCIR/
├── include/hscir/
│   ├── Types.h        # 类型系统（Integer, Float, Buffer, Function）
│   ├── Operations.h   # 操作定义（算术、内存、控制流、并行、设备操作）
│   ├── Builder.h      # IR 构建 API
│   └── CAPI.h         # C API 接口
└── src/               # 实现
```

**核心特点**：
- 自定义 SSA 形式的 IR
- 类型管理器确保类型唯一性
- 支持 `hsc.task`、`hsc.parallel_for`、`hsc.reduce` 等高层操作
- 已有 `hsc.spawn`、`hsc.place_on`、`hsc.move_to` 等设备操作

### 1.2 集成目标

| 目标 | 说明 |
|------|------|
| **分层 IR** | 从高层 HSCIR 逐步降低到 MLIR 通用方言 |
| **复用优化** | 利用 MLIR 现有优化 Pass（循环优化、内存优化等） |
| **多端生成** | 支持 GPU（CUDA/HIP）、FPGA（HLS）、NPU 代码生成 |
| **向后兼容** | 保留现有 HSCIR API，渐进式迁移 |

---

## 二、集成策略

### 2.1 两种集成路径对比

| 策略 | 优点 | 缺点 | 推荐度 |
|------|------|------|--------|
| **A. HSCIR 作为 MLIR 方言** | 完全复用 MLIR 生态，统一工具链 | 需要重写现有 IR | ⭐⭐⭐⭐ |
| **B. HSCIR → MLIR 转换层** | 保留现有代码，增量迁移 | 需要维护双向转换 | ⭐⭐⭐⭐⭐ |
| **C. 混合模式** | 平衡迁移成本和收益 | 架构复杂 | ⭐⭐⭐ |

### 2.2 推荐方案：B. HSCIR → MLIR 转换层

**核心思路**：
1. 保留现有 HSCIR 作为前端 IR
2. 新增 HSCIR → MLIR 转换层
3. 在 MLIR 层进行优化和后端代码生成
4. 渐进式将 HSCIR 迁移为 MLIR 方言

```
HSCLang AST
     ↓
  HSCIR (现有)
     ↓
[转换层] ← 新增
     ↓
  MLIR (hsc 方言)
     ↓
  优化 Pass
     ↓
  后端代码生成
```

---

## 三、详细实施方案

### 3.1 阶段一：基础设施搭建

#### 3.1.1 添加 MLIR 依赖

修改 `HSCIR/CMakeLists.txt`：

```cmake
cmake_minimum_required(VERSION 3.25)
project(hscir LANGUAGES CXX C)

set(CMAKE_CXX_STANDARD 17)  # MLIR 要求 C++17
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# 查找 LLVM/MLIR
find_package(LLVM REQUIRED CONFIG)
find_package(MLIR REQUIRED CONFIG)

message(STATUS "Found LLVM ${LLVM_PACKAGE_VERSION}")
message(STATUS "Using LLVMConfig.cmake in: ${LLVM_DIR}")

# 添加 MLIR 包含路径
include_directories(${LLVM_INCLUDE_DIRS})
include_directories(${MLIR_INCLUDE_DIRS})

# 定义 MLIR 库链接
set(MLIR_LIBS
    MLIRIR
    MLIRPass
    MLIRTransforms
    MLIRSupport
    MLIRAnalysis
    MLIRParser
    MLIRPrinter
)

# 现有 HSCIR 库
add_library(hscir SHARED
    src/Types.cpp
    src/Operations.cpp
    src/Builder.cpp
    src/CAPI.cpp
    src/Module.cpp
    src/Verifier.cpp
)

# 新增 MLIR 转换库
add_library(hscir_mlir
    src/mlir/HSCIRToMLIR.cpp
    src/mlir/HSCIRDialect.cpp
    src/mlir/HSCIROps.cpp
    src/mlir/HSCIRTypes.cpp
)

target_link_libraries(hscir_mlir PUBLIC
    hscir
    ${MLIR_LIBS}
)

target_link_libraries(hscir PUBLIC ${LLVM_LIBRARIES})
```

#### 3.1.2 目录结构调整

```
HSCIR/
├── include/hscir/
│   ├── Types.h
│   ├── Operations.h
│   ├── Builder.h
│   ├── CAPI.h
│   └── mlir/                    # 新增
│       ├── HSCIRDialect.h       # HSC 方言定义
│       ├── HSCIROps.h           # HSC 操作
│       ├── HSCIRTypes.h         # HSC 类型
│       └── HSCIRToMLIR.h        # 转换接口
├── src/
│   ├── Types.cpp
│   ├── Operations.cpp
│   ├── Builder.cpp
│   ├── CAPI.cpp
│   ├── Module.cpp
│   ├── Verifier.cpp
│   └── mlir/                    # 新增
│       ├── HSCIRDialect.cpp
│       ├── HSCIROps.cpp
│       ├── HSCIRTypes.cpp
│       └── HSCIRToMLIR.cpp
└── CMakeLists.txt
```

### 3.2 阶段二：定义 HSC MLIR 方言

#### 3.2.1 方言注册 (`HSCIRDialect.h`)

```cpp
#ifndef HSCIR_MLIR_HSCIRDIALECT_H
#define HSCIR_MLIR_HSCIRDIALECT_H

#include "mlir/IR/Dialect.h"
#include "mlir/IR/Types.h"
#include "mlir/IR/Attributes.h"

// 前向声明生成的定义
#include "HSCIROps.h.inc"  // TableGen 生成

namespace hscir::mlir_dialect {

// HSC 方言
class HSCDialect : public ::mlir::Dialect {
public:
    explicit HSCDialect(::mlir::MLIRContext *context);
    ~HSCDialect() override;
    
    static constexpr ::llvm::StringLiteral getDialectNamespace() {
        return ::llvm::StringLiteral("hsc");
    }
    
    // 类型解析
    ::mlir::Type parseType(::mlir::DialectAsmParser &parser) const override;
    void printType(::mlir::Type type, ::mlir::DialectAsmPrinter &os) const override;
    
    // 属性解析
    ::mlir::Attribute parseAttribute(::mlir::DialectAsmParser &parser,
                                      ::mlir::Type type) const override;
    void printAttribute(::mlir::Attribute attr, 
                        ::mlir::DialectAsmPrinter &os) const override;
};

} // namespace hscir::mlir_dialect

#endif // HSCIR_MLIR_HSCIRDIALECT_H
```

#### 3.2.2 类型定义 (`HSCIRTypes.td` - TableGen)

```tablegen
//===-- HSCIRTypes.td - HSC 类型定义 --------------------------===//

#ifndef HSCIR_TYPES
#define HSCIR_TYPES

include "mlir/IR/OpBase.td"

def HSC_Dialect : Dialect {
  let name = "hsc";
  let cppNamespace = "::hscir::mlir_dialect";
  let description = [{
    HSC (Heterogeneous Computing) 方言，用于表示异构计算的高层抽象。
    支持任务并行、设备管理、数据移动等操作。
  }];
}

// Buffer 类型 - 多维缓冲区
def HSC_BufferType : TypeDef<HSC_Dialect, "Buffer"> {
  let summary = "多维缓冲区类型";
  let description = [{
    表示一个多维缓冲区，对应 HSCLang 中的 Buffer<T>。
    支持静态和动态形状。
  }];
  
  let parameters = (ins
    "Type":$elementType,
    ArrayRefParameter<"int64_t">:$shape,
    "std::optional<DeviceKind>":$device
  );
  
  let mnemonic = "buffer";
  let assemblyFormat = "`<` $elementType `,` $shape (`on` $device^)? `>`";
}

// Device 类型 - 设备标识
def HSC_DeviceType : TypeDef<HSC_Dialect, "Device"> {
  let summary = "设备标识类型";
  let parameters = (ins
    "DeviceKind":$kind  // CPU, GPU, FPGA, NPU
  );
  let mnemonic = "device";
}

// Pattern 属性 - 并行模式
def HSC_PatternAttr : AttrDef<HSC_Dialect, "Pattern"> {
  let parameters = (ins
    "PatternKind":$kind,        // For, Reduce, Scan, TaskGraph
    "bool":$independent,
    "std::optional<int64_t>":$tileSize
  );
  let mnemonic = "pattern";
}

// Policy 属性 - 执行策略
def HSC_PolicyAttr : AttrDef<HSC_Dialect, "Policy"> {
  let parameters = (ins
    "std::optional<DeviceKind>":$deviceHint,
    "Granularity":$granularity,  // Fine, Medium, Coarse
    "int":$priority
  );
  let mnemonic = "policy";
}

#endif // HSCIR_TYPES
```

#### 3.2.3 操作定义 (`HSCIROps.td` - TableGen)

```tablegen
//===-- HSCIROps.td - HSC 操作定义 ---------------------------===//

#ifndef HSCIR_OPS
#define HSCIR_OPS

include "HSCIRTypes.td"

//===----------------------------------------------------------------------===//
// 任务操作
//===----------------------------------------------------------------------===//

def HSC_TaskOp : HSC_Op<"task"> {
  let summary = "定义一个异构计算任务";
  let description = [{
    定义一个可在异构设备上执行的任务。
    任务包含执行模式（pattern）和执行策略（policy）。
  }];
  
  let arguments = (ins
    StrAttr:$sym_name,
    OptionalAttr<HSC_PatternAttr>:$pattern,
    OptionalAttr<HSC_PolicyAttr>:$policy
  );
  
  let results = (outs
    FunctionType:$type
  );
  
  let regions = (region AnyRegion:$body);
  
  let assemblyFormat = [{
    $sym_name (`pattern` $pattern^)? (`policy` $policy^)?
      `(` $type.inputs `)` `->` $type.results
      $body attr-dict
  }];
}

def HSC_SpawnOp : HSC_Op<"spawn"> {
  let summary = "在指定设备上启动任务";
  
  let arguments = (ins
    HSC_DeviceType:$device,
    FlatSymbolRefAttr:$task,
    Variadic<AnyType>:$args,
    BoolAttr:$await
  );
  
  let results = (outs
    Optional<AnyType>:$result
  );
  
  let assemblyFormat = [{
    $device `,` $task `(` $args `)` (`await` $await^)? attr-dict
      `:` functional-type(operands, results)
  }];
}

//===----------------------------------------------------------------------===//
// 并行操作
//===----------------------------------------------------------------------===//

def HSC_ParallelForOp : HSC_Op<"parallel_for", [AutomaticAllocationScope]> {
  let summary = "并行循环";
  let description = [{
    表示可并行执行的循环。循环迭代之间无依赖。
  }];
  
  let arguments = (ins
    Index:$lowerBound,
    Index:$upperBound,
    Index:$step,
    OptionalAttr<HSC_PatternAttr>:$pattern
  );
  
  let regions = (region SizedRegion<1>:$body);
  
  let assemblyFormat = [{
    $lowerBound `:` type($lowerBound) `to` $upperBound `:` type($upperBound)
      `step` $step `:` type($step)
      (`pattern` $pattern^)?
      $body attr-dict
  }];
  
  let extraClassDeclaration = [{
    BlockArgument getInductionVar() { return getBody()->getArgument(0); }
  }];
}

def HSC_ReduceOp : HSC_Op<"reduce"> {
  let summary = "归约操作";
  
  let arguments = (ins
    AnyType:$input,
    AnyType:$initValue,
    ArrayAttr:$axes,
    I32Attr:$kind  // SUM, PROD, MIN, MAX, AND, OR, XOR
  );
  
  let results = (outs
    AnyType:$result
  );
}

//===----------------------------------------------------------------------===//
// 设备操作
//===----------------------------------------------------------------------===//

def HSC_PlaceOnOp : HSC_Op<"place_on", [Pure]> {
  let summary = "标记数据放置位置";
  let description = [{
    标记缓冲区应该放置在指定设备上。这是一个纯操作，不产生实际数据移动。
  }];
  
  let arguments = (ins
    HSC_BufferType:$buffer,
    HSC_DeviceType:$device
  );
  
  let results = (outs
    HSC_BufferType:$result
  );
  
  let assemblyFormat = "$buffer `,` $device attr-dict `:` type($buffer)";
}

def HSC_MoveToOp : HSC_Op<"move_to"> {
  let summary = "在设备间移动数据";
  let description = [{
    产生实际的数据传输，将缓冲区从当前设备移动到目标设备。
  }];
  
  let arguments = (ins
    HSC_BufferType:$buffer,
    HSC_DeviceType:$targetDevice
  );
  
  let results = (outs
    HSC_BufferType:$result
  );
  
  let assemblyFormat = "$buffer `->` $targetDevice attr-dict `:` type($buffer)";
}

def HSC_SyncOp : HSC_Op<"sync"> {
  let summary = "设备同步";
  let arguments = (ins
    Optional<HSC_DeviceType>:$device
  );
}

//===----------------------------------------------------------------------===//
// 内存操作
//===----------------------------------------------------------------------===//

def HSC_BufferAllocOp : HSC_Op<"buffer.alloc"> {
  let summary = "分配缓冲区";
  
  let arguments = (ins
    TypeAttr:$elementType,
    Variadic<Index>:$dims,
    OptionalAttr<HSC_DeviceType>:$device
  );
  
  let results = (outs
    HSC_BufferType:$result
  );
  
  let assemblyFormat = [{
    $elementType `<` $dims `>` (`on` $device^)? attr-dict
  }];
}

def HSC_BufferLoadOp : HSC_Op<"buffer.load", [Pure]> {
  let summary = "从缓冲区加载元素";
  
  let arguments = (ins
    HSC_BufferType:$buffer,
    Variadic<Index>:$indices
  );
  
  let results = (outs
    AnyType:$result
  );
}

def HSC_BufferStoreOp : HSC_Op<"buffer.store"> {
  let summary = "存储元素到缓冲区";
  
  let arguments = (ins
    AnyType:$value,
    HSC_BufferType:$buffer,
    Variadic<Index>:$indices
  );
}

#endif // HSCIR_OPS
```

### 3.3 阶段三：实现转换层

#### 3.3.1 HSCIR → MLIR 转换器

```cpp
// include/hscir/mlir/HSCIRToMLIR.h
#ifndef HSCIR_MLIR_HSCIR_TO_MLIR_H
#define HSCIR_MLIR_HSCIR_TO_MLIR_H

#include "mlir/IR/Builders.h"
#include "mlir/IR/MLIRContext.h"
#include "mlir/IR/Operation.h"
#include "hscir/Operations.h"
#include "hscir/Types.h"
#include <memory>

namespace hscir::mlir_dialect {

/// HSCIR 到 MLIR 的转换器
class HSCIRToMLIRConverter {
public:
    explicit HSCIRToMLIRConverter(::mlir::MLIRContext* context);
    ~HSCIRToMLIRConverter() = default;
    
    /// 转换整个模块
    ::mlir::OwningOpRef<::mlir::ModuleOp> convertModule(const ::hscir::Module* hscirModule);
    
    /// 转换类型
    ::mlir::Type convertType(::hscir::Type* type);
    
    /// 转换操作
    ::mlir::Operation* convertOperation(::hscir::Operation* op, ::mlir::Builder& builder);
    
    /// 转换区域
    void convertRegion(::hscir::Region* region, ::mlir::Region& mlirRegion, ::mlir::Builder& builder);
    
    /// 转换块
    ::mlir::Block* convertBlock(::hscir::Block* block, ::mlir::Builder& builder);
    
    /// 获取值映射
    ::mlir::Value getMappedValue(::hscir::Value* value);
    void mapValue(::hscir::Value* hscirValue, ::mlir::Value mlirValue);
    
private:
    ::mlir::MLIRContext* context_;
    
    // 值映射表
    llvm::DenseMap<::hscir::Value*, ::mlir::Value> valueMap_;
    
    // 块映射表
    llvm::DenseMap<::hscir::Block*, ::mlir::Block*> blockMap_;
    
    // 类型转换辅助
    ::mlir::Type convertIntegerType(::hscir::IntegerType* type);
    ::mlir::Type convertFloatType(::hscir::FloatType* type);
    ::mlir::Type convertBufferType(::hscir::BufferType* type);
    ::mlir::Type convertFunctionType(::hscir::FunctionType* type);
    
    // 操作转换辅助
    ::mlir::Operation* convertFuncOp(::hscir::FuncOp* op, ::mlir::Builder& builder);
    ::mlir::Operation* convertTaskOp(::hscir::TaskOp* op, ::mlir::Builder& builder);
    ::mlir::Operation* convertParallelForOp(::hscir::ParallelForOp* op, ::mlir::Builder& builder);
    ::mlir::Operation* convertReduceOp(::hscir::ReduceOp* op, ::mlir::Builder& builder);
    ::mlir::Operation* convertSpawnOp(::hscir::SpawnOp* op, ::mlir::Builder& builder);
    ::mlir::Operation* convertPlaceOnOp(::hscir::PlaceOnOp* op, ::mlir::Builder& builder);
    ::mlir::Operation* convertMoveToOp(::hscir::MoveToOp* op, ::mlir::Builder& builder);
    ::mlir::Operation* convertSyncOp(::hscir::SyncOp* op, ::mlir::Builder& builder);
};

} // namespace hscir::mlir_dialect

#endif // HSCIR_MLIR_HSCIR_TO_MLIR_H
```

#### 3.3.2 转换器实现 (`HSCIRToMLIR.cpp`)

```cpp
#include "hscir/mlir/HSCIRToMLIR.h"
#include "hscir/mlir/HSCIRDialect.h"
#include "mlir/IR/BuiltinOps.h"
#include "mlir/IR/BuiltinTypes.h"
#include "mlir/IR/Verifier.h"
#include "llvm/Support/Debug.h"

#define DEBUG_TYPE "hscir-to-mlir"

namespace hscir::mlir_dialect {

HSCIRToMLIRConverter::HSCIRToMLIRConverter(::mlir::MLIRContext* context)
    : context_(context) {
    // 注册 HSC 方言
    context_->getOrLoadDialect<HSCDialect>();
    // 加载必要的标准方言
    context_->getOrLoadDialect<::mlir::BuiltinDialect>();
}

::mlir::Type HSCIRToMLIRConverter::convertType(::hscir::Type* type) {
    if (auto* intType = dynamic_cast<::hscir::IntegerType*>(type)) {
        return convertIntegerType(intType);
    }
    if (auto* floatType = dynamic_cast<::hscir::FloatType*>(type)) {
        return convertFloatType(floatType);
    }
    if (auto* bufferType = dynamic_cast<::hscir::BufferType*>(type)) {
        return convertBufferType(bufferType);
    }
    if (auto* funcType = dynamic_cast<::hscir::FunctionType*>(type)) {
        return convertFunctionType(funcType);
    }
    
    LLVM_DEBUG(llvm::dbgs() << "Unknown type kind: " << static_cast<int>(type->getKind()) << "\n");
    return {};
}

::mlir::Type HSCIRToMLIRConverter::convertIntegerType(::hscir::IntegerType* type) {
    unsigned width = type->getWidth();
    if (type->isSigned()) {
        return ::mlir::IntegerType::get(context_, width, ::mlir::IntegerType::Signed);
    } else {
        return ::mlir::IntegerType::get(context_, width, ::mlir::IntegerType::Unsigned);
    }
}

::mlir::Type HSCIRToMLIRConverter::convertFloatType(::hscir::FloatType* type) {
    switch (type->getWidth()) {
        case 16: return ::mlir::Float16Type::get(context_);
        case 32: return ::mlir::Float32Type::get(context_);
        case 64: return ::mlir::Float64Type::get(context_);
        default: 
            LLVM_DEBUG(llvm::dbgs() << "Unsupported float width: " << type->getWidth() << "\n");
            return {};
    }
}

::mlir::Type HSCIRToMLIRConverter::convertBufferType(::hscir::BufferType* type) {
    // 将 Buffer 转换为 MemRef
    auto elemType = convertType(type->getElementType().get());
    auto shape = type->getShape();
    
    // 使用 MemRefType 作为内存表示
    return ::mlir::MemRefType::get(shape, elemType);
}

::mlir::Type HSCIRToMLIRConverter::convertFunctionType(::hscir::FunctionType* type) {
    llvm::SmallVector<::mlir::Type, 4> inputs;
    llvm::SmallVector<::mlir::Type, 4> results;
    
    for (const auto& input : type->getInputs()) {
        inputs.push_back(convertType(input.get()));
    }
    for (const auto& result : type->getOutputs()) {
        results.push_back(convertType(result.get()));
    }
    
    return ::mlir::FunctionType::get(context_, inputs, results);
}

::mlir::OwningOpRef<::mlir::ModuleOp> HSCIRToMLIRConverter::convertModule(
    const ::hscir::Module* hscirModule) {
    
    ::mlir::Builder builder(context_);
    
    // 创建 MLIR 模块
    auto moduleOp = ::mlir::ModuleOp::create(builder.getUnknownLoc());
    ::mlir::OpBuilder opBuilder(moduleOp.getBodyRegion());
    
    // 转换模块中的所有操作
    for (size_t i = 0; i < hscirModule->getNumOperations(); ++i) {
        auto* op = hscirModule->getOperation(i);
        if (auto* convertedOp = convertOperation(op, opBuilder)) {
            opBuilder.insert(convertedOp);
        }
    }
    
    // 验证模块
    if (::mlir::failed(::mlir::verify(moduleOp))) {
        LLVM_DEBUG(llvm::dbgs() << "Module verification failed\n");
        return {};
    }
    
    return moduleOp;
}

::mlir::Operation* HSCIRToMLIRConverter::convertOperation(
    ::hscir::Operation* op, ::mlir::Builder& builder) {
    
    const std::string& name = op->getName();
    
    if (name == "func") {
        return convertFuncOp(dynamic_cast<::hscir::FuncOp*>(op), builder);
    }
    if (name == "hsc.task") {
        return convertTaskOp(dynamic_cast<::hscir::TaskOp*>(op), builder);
    }
    if (name == "hsc.parallel_for") {
        return convertParallelForOp(dynamic_cast<::hscir::ParallelForOp*>(op), builder);
    }
    if (name == "hsc.reduce") {
        return convertReduceOp(dynamic_cast<::hscir::ReduceOp*>(op), builder);
    }
    if (name == "hsc.spawn") {
        return convertSpawnOp(dynamic_cast<::hscir::SpawnOp*>(op), builder);
    }
    if (name == "hsc.place_on") {
        return convertPlaceOnOp(dynamic_cast<::hscir::PlaceOnOp*>(op), builder);
    }
    if (name == "hsc.move_to") {
        return convertMoveToOp(dynamic_cast<::hscir::MoveToOp*>(op), builder);
    }
    if (name == "hsc.sync") {
        return convertSyncOp(dynamic_cast<::hscir::SyncOp*>(op), builder);
    }
    
    // 处理基本算术操作
    if (name == "add") {
        auto lhs = getMappedValue(op->getOperand(0).get());
        auto rhs = getMappedValue(op->getOperand(1).get());
        return builder.create<::mlir::arith::AddIOp>(builder.getUnknownLoc(), lhs, rhs);
    }
    // ... 其他操作
    
    LLVM_DEBUG(llvm::dbgs() << "Unknown operation: " << name << "\n");
    return nullptr;
}

::mlir::Operation* HSCIRToMLIRConverter::convertTaskOp(
    ::hscir::TaskOp* op, ::mlir::Builder& builder) {
    
    auto funcType = convertType(op->getFunctionType().get())
        .dyn_cast_or_null<::mlir::FunctionType>();
    if (!funcType) return nullptr;
    
    // 创建 funcop 作为任务的载体
    auto funcOp = builder.create<::mlir::func::FuncOp>(
        builder.getUnknownLoc(), 
        op->getSymName(), 
        funcType
    );
    
    // 添加任务属性
    funcOp->setAttr("hsc.task", builder.getUnitAttr());
    
    // 转换任务体
    if (auto* body = op->getBody()) {
        auto& entryBlock = funcOp.addEntryBlock();
        ::mlir::OpBuilder bodyBuilder(entryBlock);
        
        // 映射参数
        for (size_t i = 0; i < entryBlock.getNumArguments(); ++i) {
            mapValue(op->getEntryBlock()->getArgument(i).get(), 
                    entryBlock.getArgument(i));
        }
        
        // 转换块中的操作
        convertRegion(body, funcOp.getBody(), bodyBuilder);
    }
    
    return funcOp;
}

::mlir::Operation* HSCIRToMLIRConverter::convertParallelForOp(
    ::hscir::ParallelForOp* op, ::mlir::Builder& builder) {
    
    auto lb = getMappedValue(op->getLowerBound().get());
    auto ub = getMappedValue(op->getUpperBound().get());
    auto step = getMappedValue(op->getStep().get());
    
    // 使用 scf.for 或 scf.parallel
    auto parallelOp = builder.create<::mlir::scf::ParallelOp>(
        builder.getUnknownLoc(),
        llvm::ArrayRef<::mlir::Value>{lb},
        llvm::ArrayRef<::mlir::Value>{ub},
        llvm::ArrayRef<::mlir::Value>{step}
    );
    
    // 转换循环体
    if (auto* body = op->getBody()) {
        convertRegion(body, parallelOp.getRegion(), builder);
    }
    
    return parallelOp;
}

// ... 其他操作转换实现

void HSCIRToMLIRConverter::mapValue(::hscir::Value* hscirValue, ::mlir::Value mlirValue) {
    valueMap_[hscirValue] = mlirValue;
}

::mlir::Value HSCIRToMLIRConverter::getMappedValue(::hscir::Value* value) {
    auto it = valueMap_.find(value);
    if (it != valueMap_.end()) {
        return it->second;
    }
    return {};
}

} // namespace hscir::mlir_dialect
```

### 3.4 阶段四：优化和后端 Pass

#### 3.4.1 降低 Pass 管道

```cpp
// include/hscir/mlir/Passes.h
#ifndef HSCIR_MLIR_PASSES_H
#define HSCIR_MLIR_PASSES_H

#include "mlir/Pass/Pass.h"
#include "mlir/Pass/PassManager.h"

namespace hscir::mlir_dialect {

/// 注册所有 HSC Pass
void registerHSCPasses();

/// 创建 HSC → 标准方言的降低 Pass
std::unique_ptr<::mlir::Pass> createLowerHSCToStandardPass();

/// 创建 HSC → GPU 方言的 Pass
std::unique_ptr<::mlir::Pass> createLowerHSCToGPUPass();

/// 创建 HSC → HLS（FPGA）的 Pass  
std::unique_ptr<::mlir::Pass> createLowerHSCToHLSPass();

/// 创建 HSC → NPU 的 Pass
std::unique_ptr<::mlir::Pass> createLowerHSCToNPUPass();

/// 构建标准优化管道
void buildStandardOptimizationPipeline(::mlir::PassManager& pm);

/// 构建 GPU 后端管道
void buildGPUBackendPipeline(::mlir::PassManager& pm);

/// 构建 FPGA 后端管道
void buildFPGABackendPipeline(::mlir::PassManager& pm);

/// 构建 NPU 后端管道
void buildNPUBackendPipeline(::mlir::PassManager& pm);

} // namespace hscir::mlir_dialect

#endif // HSCIR_MLIR_PASSES_H
```

#### 3.4.2 Pass 实现

```cpp
// src/mlir/Passes.cpp
#include "hscir/mlir/Passes.h"
#include "hscir/mlir/HSCIRDialect.h"
#include "mlir/Conversion/AffineToStandard/AffineToStandard.h"
#include "mlir/Conversion/SCFToControlFlow/SCFToControlFlow.h"
#include "mlir/Conversion/StandardToLLVM/ConvertStandardToLLVM.h"
#include "mlir/Conversion/GPUToNVVM/GPUToNVVMPass.h"
#include "mlir/Dialect/Affine/Passes.h"
#include "mlir/Dialect/SCF/Passes.h"
#include "mlir/Dialect/GPU/Passes.h"
#include "mlir/Transforms/Passes.h"

namespace hscir::mlir_dialect {

void registerHSCPasses() {
    ::mlir::registerPass(createLowerHSCToStandardPass);
    ::mlir::registerPass(createLowerHSCToGPUPass);
    ::mlir::registerPass(createLowerHSCToHLSPass);
    ::mlir::registerPass(createLowerHSCToNPUPass);
}

void buildStandardOptimizationPipeline(::mlir::PassManager& pm) {
    // 内联
    pm.addPass(::mlir::createInlinerPass());
    
    // 死代码消除
    pm.addPass(::mlir::createSymbolDCEPass());
    
    // 通用优化
    ::mlir::GreedyRewriteConfig config;
    config.useTopDownTraversal = true;
    pm.addPass(::mlir::createCSEPass());
    pm.addPass(::mlir::createCanonicalizerPass());
    
    // 循环优化
    pm.addPass(::mlir::createLoopInvariantCodeMotionPass());
    pm.addPass(::mlir::affine::createAffineLoopInvariantCodeMotionPass());
}

void buildGPUBackendPipeline(::mlir::PassManager& pm) {
    // 1. HSC → 标准方言
    pm.addPass(createLowerHSCToStandardPass());
    
    // 2. 标准优化
    buildStandardOptimizationPipeline(pm);
    
    // 3. 降低到 GPU 方言
    pm.addPass(createLowerHSCToGPUPass());
    
    // 4. GPU 特定优化
    pm.addPass(::mlir::createGpuKernelOutliningPass());
    
    // 5. 降低到 NVVM/ROCDL
    pm.addPass(::mlir::createConvertGPUToNVVMPass());
    
    // 6. 降低到 LLVM
    pm.addPass(::mlir::createLowerToLLVMPass());
}

void buildFPGABackendPipeline(::mlir::PassManager& pm) {
    // 1. HSC → 标准方言
    pm.addPass(createLowerHSCToStandardPass());
    
    // 2. 标准优化
    buildStandardOptimizationPipeline(pm);
    
    // 3. 循环变换（适合 FPGA）
    pm.addPass(::mlir::affine::createAffineLoopTilingPass(64));
    pm.addPass(::mlir::affine::createAffineLoopFusionPass());
    
    // 4. 降低到 HLS
    pm.addPass(createLowerHSCToHLSPass());
}

void buildNPUBackendPipeline(::mlir::PassManager& pm) {
    // 1. HSC → 标准方言
    pm.addPass(createLowerHSCToStandardPass());
    
    // 2. 标准优化
    buildStandardOptimizationPipeline(pm);
    
    // 3. 算子融合
    // TODO: 实现算子融合 Pass
    
    // 4. 降低到 NPU
    pm.addPass(createLowerHSCToNPUPass());
}

} // namespace hscir::mlir_dialect
```

### 3.5 阶段五：与 HSCC 编译器集成

#### 3.5.1 修改编译流程

在 `HSCC/hscc/src/compile.rs` 中添加 MLIR 后端：

```rust
// HSCC/hscc/src/compile.rs

use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Backend {
    Cuda,
    Triton,
    Npu,
    MlirGpu,    // 新增
    MlirFpga,   // 新增
    MlirNpu,    // 新增
}

pub fn compile_project(project_dir: &Path, backend: Backend) -> Result<(), CompileError> {
    // ... 现有代码 ...
    
    match backend {
        Backend::MlirGpu | Backend::MlirFpga | Backend::MlirNpu => {
            compile_with_mlir(project_dir, backend)
        }
        _ => {
            // 现有后端逻辑
        }
    }
}

fn compile_with_mlir(project_dir: &Path, backend: Backend) -> Result<(), CompileError> {
    // 1. 解析 HSCLang 生成 AST
    let ast = parse_hsclang(project_dir)?;
    
    // 2. 类型检查
    let typed_ast = type_check(&ast)?;
    
    // 3. 生成 HSCIR
    let hscir_module = lower_to_hscir(&typed_ast)?;
    
    // 4. 序列化 HSCIR
    let hscir_path = project_dir.join("build").join("module.hscir");
    hscir_module.serialize(&hscir_path)?;
    
    // 5. 调用 HSCIR MLIR 转换工具
    let mlir_path = project_dir.join("build").join("module.mlir");
    let output_path = project_dir.join("build").join("output");
    
    let backend_flag = match backend {
        Backend::MlirGpu => "gpu",
        Backend::MlirFpga => "fpga",
        Backend::MlirNpu => "npu",
        _ => unreachable!(),
    };
    
    let status = Command::new("hscir-mlir-opt")
        .arg("--convert-hscir-to-mlir")
        .arg(&hscir_path)
        .arg("-o")
        .arg(&mlir_path)
        .status()
        .map_err(|e| CompileError::ToolNotFound("hscir-mlir-opt".into()))?;
    
    if !status.success() {
        return Err(CompileError::ConversionFailed("HSCIR to MLIR".into()));
    }
    
    // 6. 运行 MLIR 优化管道
    let status = Command::new("mlir-opt")
        .arg("--pass-pipeline")
        .arg(format!("hsc.lower-to-{}", backend_flag))
        .arg(&mlir_path)
        .arg("-o")
        .arg(&output_path)
        .status()
        .map_err(|e| CompileError::ToolNotFound("mlir-opt".into()))?;
    
    if !status.success() {
        return Err(CompileError::OptimizationFailed);
    }
    
    // 7. 生成最终代码
    match backend {
        Backend::MlirGpu => {
            let status = Command::new("mlir-translate")
                .arg("--mlir-to-llvmir")
                .arg(&output_path)
                .arg("-o")
                .arg(project_dir.join("build").join("kernel.ll"))
                .status()?;
            // 继续编译为 PTX...
        }
        Backend::MlirFpga => {
            let status = Command::new("hscir-hls-emitter")
                .arg(&output_path)
                .arg("-o")
                .arg(project_dir.join("build").join("kernel.cpp"))
                .status()?;
        }
        Backend::MlirNpu => {
            // NPU 特定代码生成
        }
        _ => {}
    }
    
    Ok(())
}
```

---

## 四、后端特化实现

### 4.1 GPU 后端

```
HSC IR (hsc.task, hsc.parallel_for)
              ↓
        设备亲和性分析
              ↓
    ┌─────────────────────┐
    │ GPU Kernel 提取      │
    └─────────────────────┘
              ↓
    ┌─────────────────────┐
    │ scf.parallel →       │
    │ gpu.launch           │
    └─────────────────────┘
              ↓
    ┌─────────────────────┐
    │ GPU 方言优化         │
    │ - 线程块映射         │
    │ - 共享内存提升       │
    │ - 循环展开           │
    └─────────────────────┘
              ↓
    ┌─────────────────────┐
    │ gpu → nvvm/rocdl    │
    └─────────────────────┘
              ↓
    ┌─────────────────────┐
    │ nvvm → LLVM IR      │
    │ → PTX/HSACO         │
    └─────────────────────┘
```

### 4.2 FPGA 后端

```
HSC IR (hsc.task, hsc.parallel_for)
              ↓
        Pattern/Policy 分析
              ↓
    ┌─────────────────────┐
    │ HLS 变换             │
    │ - 循环流水线化       │
    │ - 数组分区           │
    │ - 接口推断           │
    └─────────────────────┘
              ↓
    ┌─────────────────────┐
    │ 生成 HLS C++        │
    │ + #pragma HLS       │
    └─────────────────────┘
              ↓
    ┌─────────────────────┐
    │ Vitis HLS 综合       │
    │ → RTL               │
    └─────────────────────┘
              ↓
    ┌─────────────────────┐
    │ FPGA 实现            │
    │ → Bitstream         │
    └─────────────────────┘
```

**HLS 代码生成示例**：

```cpp
// 输入 HSC IR
hsc.task @matmul(%A: memref<1024x1024xf32>, %B: memref<1024x1024xf32>) 
          -> memref<1024x1024xf32> {
  %C = memref.alloc<1024x1024xf32> : memref<1024x1024xf32>
  hsc.parallel_for %i = 0 to 1024 {
    hsc.for %j = 0 to 1024 {
      %sum = hsc.reduce add over %k = 0 to 1024 {
        %a = memref.load %A[%i, %k] : memref<1024x1024xf32>
        %b = memref.load %B[%k, %j] : memref<1024x1024xf32>
        hsc.yield %a * %b : f32
      }
      memref.store %sum, %C[%i, %j] : memref<1024x1024xf32>
    }
  }
  hsc.return %C : memref<1024x1024xf32>
}

// 生成的 HLS C++
void matmul(float A[1024][1024], float B[1024][1024], float C[1024][1024]) {
    #pragma HLS INTERFACE m_axi port=A depth=1048576
    #pragma HLS INTERFACE m_axi port=B depth=1048576
    #pragma HLS INTERFACE m_axi port=C depth=1048576
    #pragma HLS INTERFACE ap_ctrl_hs port=return
    
    // 循环流水线化
    for (int i = 0; i < 1024; ++i) {
        for (int j = 0; j < 1024; ++j) {
            #pragma HLS PIPELINE II=1
            float sum = 0.0f;
            for (int k = 0; k < 1024; ++k) {
                #pragma HLS UNROLL factor=4
                sum += A[i][k] * B[k][j];
            }
            C[i][j] = sum;
        }
    }
}
```

### 4.3 NPU 后端

```
HSC IR (hsc.task, hsc.parallel_for)
              ↓
        算子识别与融合
              ↓
    ┌─────────────────────┐
    │ 计算图构建           │
    │ - 算子映射           │
    │ - 内存规划           │
    └─────────────────────┘
              ↓
    ┌─────────────────────┐
    │ NPU 特定优化         │
    │ - 张量分块           │
    │ - 流水线调度         │
    │ - 数据重排           │
    └─────────────────────┘
              ↓
    ┌─────────────────────┐
    │ 生成 NPU 指令        │
    │ 或算子调用           │
    └─────────────────────┘
```

**支持的 NPU 架构**：

| 架构 | 框架 | 状态 |
|------|------|------|
| 华为昇腾 | CANN/AscendCL | 规划中 |
| Google TPU | XLA | 规划中 |
| Intel NPU | OpenVINO | 规划中 |
| 寒武纪 | Neuware | 规划中 |

---

## 五、构建系统集成

### 5.1 添加 MLIR 构建目标

修改 `HSCIR/CMakeLists.txt`：

```cmake
# MLIR 相关构建

# 使用 TableGen 生成操作和类型
set(LLVM_TARGET_DEFINITIONS HSCIROps.td)
mlir_tablegen(HSCIROps.h.inc -gen-op-decls)
mlir_tablegen(HSCIROps.cpp.inc -gen-op-defs)
mlir_tablegen(HSCIRTypes.h.inc -gen-type-decls)
mlir_tablegen(HSCIRTypes.cpp.inc -gen-type-defs)
mlir_tablegen(HSCIRDialect.h.inc -gen-dialect-decls)
mlir_tablegen(HSCIRDialect.cpp.inc -gen-dialect-defs)
add_public_tablegen_target(HSCIROpsIncGen)

# MLIR 库
add_mlir_dialect_library(HSCIRMlir
    src/mlir/HSCIRDialect.cpp
    src/mlir/HSCIROps.cpp
    src/mlir/HSCIRTypes.cpp
    src/mlir/HSCIRToMLIR.cpp
    src/mlir/Passes.cpp

    DEPENDS
    HSCIROpsIncGen
    
    LINK_LIBS PUBLIC
    MLIRIR
    MLIRPass
    MLIRTransforms
    MLIRSupport
    MLIRAnalysis
)

# hscir-mlir-opt 工具
add_llvm_executable(hscir-mlir-opt
    tools/hscir-mlir-opt.cpp
)
target_link_libraries(hscir-mlir-opt PRIVATE
    HSCIRMlir
    hscir
    MLIRParser
    MLIRPrinter
)
```

### 5.2 hscir-mlir-opt 工具

```cpp
// tools/hscir-mlir-opt.cpp
#include "mlir/IR/MLIRContext.h"
#include "mlir/IR/OwningOpRef.h"
#include "mlir/Parser/Parser.h"
#include "mlir/Pass/PassManager.h"
#include "mlir/Support/FileUtilities.h"
#include "llvm/Support/CommandLine.h"
#include "llvm/Support/InitLLVM.h"
#include "llvm/Support/raw_ostream.h"

#include "hscir/mlir/HSCIRDialect.h"
#include "hscir/mlir/Passes.h"

using namespace llvm;
using namespace mlir;

int main(int argc, char** argv) {
    InitLLVM y(argc, argv);
    
    // 注册命令行选项
    cl::opt<std::string> inputFilename(cl::Positional, cl::desc("<input file>"), cl::init("-"));
    cl::opt<std::string> outputFilename("o", cl::desc("Output filename"), cl::value_desc("filename"), cl::init("-"));
    cl::opt<bool> verify("verify", cl::desc("Verify the output"), cl::init(true));
    
    cl::ParseCommandLineOptions(argc, argv, "HSCIR MLIR optimizer\n");
    
    // 初始化 MLIR 上下文
    MLIRContext context;
    context.getOrLoadDialect<hscir::mlir_dialect::HSCDialect>();
    
    // 解析输入文件
    auto fileOrErr = MemoryBuffer::getFileOrSTDIN(inputFilename);
    if (auto ec = fileOrErr.getError()) {
        errs() << "Could not open input file: " << ec.message() << "\n";
        return 1;
    }
    
    // 解析 MLIR
    OwningOpRef<ModuleOp> module = parseSourceString<ModuleOp>(
        (*fileOrErr)->getBuffer(), &context);
    if (!module) {
        errs() << "Failed to parse MLIR\n";
        return 1;
    }
    
    // 注册 Pass
    hscir::mlir_dialect::registerHSCPasses();
    
    // 运行 Pass 管道
    PassManager pm(&context);
    hscir::mlir_dialect::buildStandardOptimizationPipeline(pm);
    
    if (failed(pm.run(*module))) {
        errs() << "Pass pipeline failed\n";
        return 1;
    }
    
    // 输出结果
    auto output = openOutputFile(outputFilename);
    if (!output) {
        return 1;
    }
    
    module->print(output->os());
    output->keep();
    
    return 0;
}
```

---

## 六、实施路线图

### 第一阶段：基础设施（1-2 月）

| 任务 | 优先级 | 依赖 |
|------|--------|------|
| 配置 MLIR 构建依赖 | 🔴 高 | LLVM |
| 创建 HSC 方言基础框架 | 🔴 高 | MLIR |
| 实现 TableGen 定义 | 🔴 高 | 方言框架 |
| 实现基础类型转换 | 🔴 高 | TableGen |

### 第二阶段：转换层（2-3 月）

| 任务 | 优先级 | 依赖 |
|------|--------|------|
| 实现 HSCIR → MLIR 转换 | 🔴 高 | 方言定义 |
| 实现值映射和块映射 | 🔴 高 | 转换框架 |
| 添加 Round-trip 测试 | 🟡 中 | 转换实现 |
| 集成到 HSCC 编译器 | 🟡 中 | 转换测试 |

### 第三阶段：GPU 后端（2-3 月）

| 任务 | 优先级 | 依赖 |
|------|--------|------|
| 实现 HSC → SCF 降低 | 🔴 高 | 转换层 |
| 实现 SCF → GPU 降低 | 🔴 高 | SCF Pass |
| 实现 GPU 优化 Pass | 🟡 中 | GPU 方言 |
| 实现 PTX 代码生成 | 🟡 中 | NVVM Pass |

### 第四阶段：FPGA/NPU 后端（3-6 月）

| 任务 | 优先级 | 依赖 |
|------|--------|------|
| 设计 HLS 方言 | 🟡 中 | MLIR 基础 |
| 实现 HLS 代码生成 | 🟡 中 | HLS 方言 |
| 设计 NPU 方言 | 🟢 低 | 硬件支持 |
| 实现算子映射 | 🟢 低 | NPU 方言 |

---

## 七、技术挑战与解决方案

### 7.1 C++ 与 Rust 的互操作

**挑战**：MLIR 是 C++ 库，HSCC 是 Rust 编译器。

**解决方案**：
1. **方案 A**：通过 C API 调用 MLIR（推荐）
   - HSCIR 提供 C API 封装 MLIR 功能
   - HSCC 通过 FFI 调用
   
2. **方案 B**：通过命令行工具
   - HSCC 调用 `hscir-mlir-opt` 等工具
   - 通过文件或管道传递 IR
   
3. **方案 C**：使用 `mlir-sys` crate
   - 社区提供的 Rust MLIR 绑定
   - 可能需要更新以支持最新版本

### 7.2 Pattern/Policy 的语义保持

**挑战**：在降低过程中保持高层语义信息。

**解决方案**：
- 将 Pattern/Policy 编码为 MLIR 属性
- 在 Pass 中读取属性并应用相应优化
- 对于无法表达的优化，发出警告

```cpp
// 保留 Pattern 属性
auto parallelFor = builder.create<HSCParallelForOp>(...);
parallelFor->setAttr("hsc.pattern", 
    PatternAttr::get(context, PatternKind::For, true, std::nullopt));
```

### 7.3 设备亲和性分析

**挑战**：确定任务应该在哪个设备上执行。

**解决方案**：
- 实现设备亲和性分析 Pass
- 根据任务特征（计算密度、内存访问模式）推荐设备
- 支持 `policy.device_hint` 覆盖自动选择

---

## 八、测试策略

### 8.1 单元测试

```cpp
// tests/mlir/test_type_conversion.cpp
#include "gtest/gtest.h"
#include "hscir/mlir/HSCIRToMLIR.h"

TEST(HSCIRToMLIR, IntegerTypeConversion) {
    mlir::MLIRContext context;
    hscir::mlir_dialect::HSCIRToMLIRConverter converter(&context);
    
    auto hscIntType = hscir::TypeManager::getInstance().getIntegerType(32, true);
    auto mlirType = converter.convertType(hscIntType.get());
    
    ASSERT_TRUE(mlirType.isa<mlir::IntegerType>());
    EXPECT_EQ(mlirType.getIntOrFloatBitWidth(), 32);
}

TEST(HSCIRToMLIR, BufferTypeConversion) {
    mlir::MLIRContext context;
    hscir::mlir_dialect::HSCIRToMLIRConverter converter(&context);
    
    auto f32 = hscir::TypeManager::getInstance().getFloatType(32);
    auto hscBufferType = hscir::TypeManager::getInstance().getBufferType(f32, {1024, 1024});
    auto mlirType = converter.convertType(hscBufferType.get());
    
    ASSERT_TRUE(mlirType.isa<mlir::MemRefType>());
    auto memref = mlirType.cast<mlir::MemRefType>();
    EXPECT_EQ(memref.getRank(), 2);
}
```

### 8.2 集成测试

```bash
# tests/integration/test_gpu_backend.sh
#!/bin/bash

# 编译 HSCLang 示例
hscc examples/matmul.hl --backend=mlir-gpu -o build/

# 检查 MLIR 生成
mlir-opt build/matmul.mlir --verify-diagnostics

# 运行生成的程序
./build/matmul
```

### 8.3 端到端测试

使用现有 HSCC 测试框架扩展 MLIR 测试：

```rust
// HSCC/hscc/tests/integration_test.rs

#[test]
fn test_mlir_gpu_backend() {
    let project = create_test_project(r#"
        task @matmul(%a: Buffer<f32>, %b: Buffer<f32>) -> Buffer<f32> {
            pattern: For { independent: true }
            policy: Policy { device_hint: GPU }
            // ... 实现
        }
    "#);
    
    let result = compile_project(&project.path(), Backend::MlirGpu);
    assert!(result.is_ok());
    
    // 验证生成的 MLIR
    let mlir_content = std::fs::read_to_string(project.path().join("build/module.mlir")).unwrap();
    assert!(mlir_content.contains("gpu.launch"));
}
```

---

## 九、文档和资源

### 9.1 官方文档

- [MLIR Documentation](https://mlir.llvm.org/)
- [MLIR Tutorial](https://mlir.llvm.org/getting_started/)
- [MLIR Language Reference](https://mlir.llvm.org/docs/LangRef/)

### 9.2 推荐学习路径

1. **入门**：MLIR Tutorial → 理解基本概念
2. **方言开发**：Defining Dialects → 学习 TableGen
3. **Pass 开发**：Pass Infrastructure → 实现优化 Pass
4. **后端开发**：Conversion Passes → 实现代码生成

### 9.3 参考项目

| 项目 | 说明 |
|------|------|
| [IREE](https://github.com/iree-org/iree) | MLIR 端到端编译器 |
| [CIRCT](https://github.com/llvm/circt) | 硬件设计编译器 |
| [MLIR-HLO](https://github.com/tensorflow/mlir-hlo) | 高层优化方言 |
| [Linalg](https://mlir.llvm.org/docs/Dialects/Linalg/) | 线性代数方言 |

---

## 十、总结

通过以上方案，可以将现有 HSCIR 模块与 MLIR 框架深度集成：

1. **保留现有投资**：通过转换层保留 HSCIR 代码，渐进式迁移
2. **复用 MLIR 生态**：利用成熟的优化 Pass 和后端支持
3. **支持多端生成**：统一 IR 支持多种异构设备
4. **保持语义信息**：通过属性系统保留 Pattern/Policy 信息

预计完成全部集成需要 **6-12 个月**，建议优先完成 GPU 后端验证整个流程。
