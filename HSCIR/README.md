# HSCIR - HSCLang 中间表示

> HSCLang 中间表示，定义类型系统和操作 IR，支持多设备代码生成。

---

## 概述

HSCIR 是 OpenHC 项目的中间表示组件，为编译器提供一个设备无关的 IR 层，便于进行优化和代码生成。目前 HSCIR 已实现核心类型系统、操作定义和 Builder API，并提供了 IR 分析框架（Pass 管理器）。与 HSCC 编译器的集成正在进行中，计划作为设备无关优化的核心层。

## 技术栈

- **语言**: C++23
- **构建工具**: CMake

## 目录结构

```
HSCIR/
├── include/hscir/    # 头文件
│   ├── Types.h       # 类型系统
│   ├── Operations.h  # 操作定义
│   ├── Builder.h     # IR 构建器
│   ├── CAPI.h        # C API
│   └── HSCIR.h       # 主头文件
├── src/              # 源文件
│   ├── Types.cpp
│   ├── Operations.cpp
│   ├── Builder.cpp
│   ├── CAPI.cpp
│   └── Module.cpp
└── targets/          # 构建输出
```

## 核心类型系统

### Type 基类

类型基类，支持以下类型种类（`Kind`）：
- `Integer` - 整数类型
- `Float` - 浮点类型
- `Buffer` - 缓冲区类型
- `Function` - 函数类型
- `None` - 空类型

### IntegerType

整数类型，支持：
- 指定位宽（如 8、16、32、64 位）
- 指定符号性（有符号/无符号）

### FloatType

浮点类型，支持：
- 指定位宽（如 16、32、64 位）

### BufferType

缓冲区类型，对应 HSCLang 中的 `Buffer<T>`：
- 元素类型
- 多维形状

### FunctionType

函数类型，用于函数/任务签名：
- 输入类型列表
- 输出类型列表

### TypeManager

类型管理器（单例模式）：
- 确保类型唯一性
- 提供类型缓存机制

## 操作系统

### Operation

操作基类，包含：
- 操作名称
- 操作数列表
- 结果类型列表
- 属性字典
- 区域列表

### Value

值基类，表示操作结果或块参数：
- `OpResult` - 操作结果值
- `BlockArgument` - 块参数值

### Block

基本块：
- 操作序列
- 块参数列表

### Region

区域，包含一个或多个基本块，用于表示函数体、循环体等。

### Module

模块，顶层容器，包含所有顶层操作。

## Builder 模式

`Builder` 类提供 IR 构建 API：

```cpp
// 创建类型
auto i32 = builder.getI32Type();
auto f32 = builder.getF32Type();
auto buffer = builder.getBufferType(f32, {1024, 1024});

// 创建操作
auto func = builder.createFuncOp("main", {i32}, {}, body);
auto task = builder.createTaskOp("compute", {buffer}, {buffer}, body);
auto parallelFor = builder.createParallelForOp(lb, ub, step, loopBody);
```

## 构建

```bash
cd HSCIR
cmake -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build
```

输出：
- `libhscir.dll` - 动态链接库

## 使用示例

```cpp
#include <hscir/Builder.h>
#include <hscir/Types.h>
#include <iostream>

int main() {
    hscir::Builder builder;
    
    // 获取类型
    auto f32 = builder.getF32Type();
    auto bufferType = builder.getBufferType(f32, {1024, 1024});
    
    std::cout << "Type: " << bufferType->toString() << std::endl;
    // 输出: Buffer<f32, [1024, 1024]>
    
    return 0;
}
```

## 设计理念

HSCIR 的设计借鉴了 MLIR 的理念：

1. **分层 IR**: 支持多层抽象，从高层任务到低层指令
2. **类型安全**: 强类型系统确保 IR 正确性
3. **可扩展**: 易于添加新的操作和类型
4. **SSA 形式**: 使用 SSA（静态单赋值）形式

## 当前状态

### 已完成功能
- ✅ 核心类型系统（IntegerType、FloatType、BufferType、FunctionType）
- ✅ 操作基类（Operation）及常用操作（算术、内存、控制流、并行、设备操作）
- ✅ Builder API，提供类型创建、操作创建、区域/块管理
- ✅ TypeManager 单例，确保类型唯一性
- ✅ IR 分析框架（Pass 管理器），支持数据流分析、依赖分析、设备亲和性分析
- ✅ C API 接口，支持与其他语言交互

### 进行中工作
- 🔄 与 HSCC 编译器前端的完整集成（AST → HSCIR 转换）
- 🔄 设备无关优化 Pass 的实现
- 🔄 到各后端（CUDA/NPU/Triton）的代码生成接口

### 规划功能
- ⏳ MLIR 方言集成，作为 `hsc` 方言实现
- ⏳ 更多优化 Pass（循环优化、内存优化、设备放置优化）
- ⏳ 调试信息和性能分析支持

## 相关文档

- 编译器: `HSCC/README.md`
- 语言设计: `HSCLang/README.md`
- MLIR 集成计划: `docs/TODO.MLIR.md`
