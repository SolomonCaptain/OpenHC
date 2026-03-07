# OpenHC 项目上下文

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/SolomonCaptain/OpenHC)

> NPU + FPGA + CUDA GPU 异构计算解决方案，包括编程语言、编译器、中间表示、构建系统和 IDE。

---

## 项目概览

OpenHC 是一个异构计算平台项目，旨在提供一套完整的工具链，让开发者能够使用统一的编程语言（HSCLang）编写代码，并自动映射到 GPU、NPU、FPGA 等异构设备上执行。

### 核心设计理念

- **单一来源，多设备生成**：一套代码，自动映射到 FPGA、NPU、GPU
- **数据流优先**：以数据移动和变换为中心思考问题
- **顺序任务流（STF）**：开发者以顺序方式编写代码，运行时自动解析依赖并构建 DAG 并行执行

---

## 项目结构

```
OpenHC/
├── HSCC/           # 编译器 (Rust)
├── HSCIR/          # 中间表示 (C++23)
├── HSCLang/        # 编程语言设计与规范
├── HSCMake/        # 构建系统 (Python 3.13+)
├── HSCIDE/         # IDE 与渲染管线
└── docs/           # 文档
```

## 当前状态

| 组件 | 成熟度 | 功能状态 | 备注 |
|------|--------|----------|------|
| **HSCC 编译器** | 🟢 可用 | 完整的词法/语法/语义分析，CUDA/HIP/Triton/NPU 后端代码生成 | 支持静态分析、性能分析、IR 分析 |
| **HSCIR 中间表示** | 🟡 开发中 | 类型系统、操作定义、Builder API 完成，IR 分析框架就绪 | 与编译器前端集成进行中 |
| **HSCLang 语言设计** | 🟢 稳定 | 语法规范完整，示例项目可用，语言文档详尽 | 设计理念和语法元素已固化 |
| **HSCMake 构建系统** | 🟢 可用 | 支持 C++/Rust/TypeScript 多语言构建，命令行工具完整 | 与 OpenHC 项目自身构建集成 |
| **HSCIDE 集成开发环境** | 🟡 开发中 | 渲染管线 (gRPC + Vulkan) 实现完整，IDE 前端骨架就绪 | 可视化仿真结果流式传输 |
| **HSCPlugins 插件系统** | 🟡 开发中 | 插件 API 和管理器框架就绪，示例插件（光子学 FDTD）模板 | 支持领域特定扩展 |
| **FPGA 后端** | ⚪ 规划中 | 设计文档完成，Vitis HLS 代码生成方案制定 | 优先级低于 NPU/GPU |
| **NPU 后端** | 🟢 可用 | Intel NPU (OpenVINO) 支持，ONNX 模型生成，运行时集成 | 支持 Meteor Lake NPU 推理加速 |
| **Triton 后端** | 🟢 可用 | Python Triton 代码生成，设备无关 GPU 编程 | 适合快速原型和跨平台 GPU 代码 |

**成熟度说明**: 🟢 可用 (生产就绪) | 🟡 开发中 (功能基本完成) | ⚪ 规划中 (设计阶段)

---

## 子项目详解

### HSCC - 编译器

**路径**: `HSCC/hscc/`

**技术栈**: Rust (Edition 2024)

**依赖**:
- `toml` - 配置文件解析
- `serde` - 序列化/反序列化
- `regex` - 词法分析
- `anyhow` - 错误处理

**编译流程**:
```
HSCLang源文件 (.hl)
       ↓
  前端解析器 (lexer.rs, parser.rs)
       ↓
  抽象语法树 (ast.rs)
       ↓
  类型检查器 (typeck.rs)
       ↓
  HSCIR 中间表示 (lower.rs)
       ↓
  设备无关优化与静态分析
       ↓
  ┌─────────────────────────────────────────────────────┐
  │                                                     │
  ▼                                                     ▼
GPU后端 (CUDA/HIP)                               NPU后端 (Intel NPU/OpenVINO)
       ↓                                                     ↓
CUDA/HIP C++ 代码 (.cu)                              ONNX/OpenVINO IR
       ↓                                                     ↓
nvcc/hipcc 编译                                      Python 运行时代码
       ↓                                                     ↓
  可执行文件                                          执行脚本
  │                                                     │
  └─────────────────────────────────────────────────────┘
       ↓
  Triton后端 (可选)
       ↓
  Python Triton 代码
       ↓
  直接执行
```

**支持的后端**:
- **CUDA/HIP**: 生成 CUDA C++ 代码并调用 nvcc/hipcc 编译，支持 NVIDIA 和 AMD GPU
- **Triton**: 生成 Python Triton 代码，支持设备无关的 GPU 编程
- **NPU**: 生成 ONNX 模型和 OpenVINO IR，支持 Intel NPU (Meteor Lake) 推理加速
- **FPGA**: *规划中*（基于 Vitis HLS）

**核心模块**:
- `lexer.rs` - 词法分析器，定义 `TokenKind` 枚举（异构专用关键字、符号、字面量等）
- `parser.rs` - 语法分析器，解析 `import`、`fn`、`task`、`pipeline`、`graph` 等声明
- `ast.rs` - AST 节点定义，包括 `Program`、`Function`、`Task`、`Pattern`、`Policy`、`Buffer` 等
- `typeck.rs` - 类型检查器，实现类型推断、兼容性检查、错误报告
- `lower.rs` - AST 到 HSCIR 转换，实现程序、任务、控制流转换
- `codegen.rs` - CUDA/HIP 代码生成器，将 AST 转换为 GPU 代码
- `compile.rs` - 编译驱动，调用 nvcc/hipcc 编译 CUDA/HIP 代码
- `config.rs` - 配置文件解析（HSCC.toml）
- `analysis.rs` - 静态分析，包括模式-策略检查
- `dataflow.rs` - 数据流分析
- `semantic.rs` - 语义分析
- `performance.rs` - 性能分析
- `target_check.rs` - 目标设备特定检查
- `diagnostic.rs` - 诊断信息收集与报告
- `triton/` - Triton 后端，生成 Python Triton 代码
- `npu/` - NPU 后端，生成 ONNX/OpenVINO IR 和运行时代码
- `hscir/` - HSCIR 中间表示模块，包含 Pass 管理器和分析工具

**构建命令**:
```bash
cd HSCC/hscc
cargo build --release
```

**运行**:
```bash
hscc <project-directory>
```

**配置文件格式** (`HSCC.toml`):
```toml
[package]
name = "project_name"       # 项目名称
version = "0.1.0"           # 版本号
edition = "2026"            # 语言版本

[target]
device = "cuda"             # 目标设备: cuda, hip, triton, npu
arch = "sm_61"              # 设备架构: sm_61 (CUDA), gfx90a (HIP), intel_meteorlake (NPU)

[backend]                   # 可选后端配置
kind = "cuda"               # 后端类型: cuda, hip, triton, npu
optimization_level = 2      # 优化级别 (0-3)
debug = false               # 调试模式

[analysis]                  # 分析配置
static = true               # 启用静态分析
performance = true          # 启用性能分析
ir = false                  # 启用 IR 分析
```

---

### HSCIR - 中间表示

**路径**: `HSCIR/`

**技术栈**: C++23, CMake

**核心类型系统** (`include/hscir/Types.h`):
- `Type` - 类型基类，支持 `Integer`、`Float`、`Buffer`、`Function`、`None` 五种类型
- `IntegerType` - 整数类型（支持指定宽度和符号性）
- `FloatType` - 浮点类型（支持指定宽度）
- `BufferType` - 缓冲区类型（对应 hsc.buffer），包含元素类型和形状
- `FunctionType` - 函数类型（用于函数/任务签名）
- `TypeManager` - 类型管理器（单例模式，确保类型唯一）

**操作系统** (`include/hscir/Operations.h`):

**基础类**:
- `Operation` - 操作基类，包含操作数、结果类型、属性和区域
- `Value` - 值基类（操作结果或块参数）
- `OpResult` - 操作结果值
- `BlockArgument` - 块参数值
- `Block` - 基本块，包含操作序列和参数
- `Region` - 区域，包含一个或多个块
- `Module` - 模块（顶层容器）

**算术操作**:
- `AddOp`, `SubOp`, `MulOp`, `DivOp`, `ModOp` - 基本算术运算
- `CmpOp` - 比较操作（EQ, NE, LT, LE, GT, GE）

**内存操作**:
- `AllocOp` - 内存分配
- `LoadOp` - 内存加载
- `StoreOp` - 内存存储

**控制流操作**:
- `BranchOp` - 无条件跳转
- `CondBranchOp` - 条件跳转
- `ReturnOp` - 函数返回

**并行操作**:
- `ParallelForOp` - 并行循环
- `ReduceOp` - 归约操作（SUM, PROD, MIN, MAX, AND, OR, XOR）

**设备操作**:
- `SpawnOp` - 任务启动
- `SyncOp` - 设备同步
- `MoveToOp` - 数据迁移
- `PlaceOnOp` - 设备放置

**其他操作**:
- `ConstantOp` - 常量定义
- `FuncOp` - 函数定义
- `TaskOp` - 任务定义

**Builder 模式** (`include/hscir/Builder.h`):
- `Builder` 类提供类型创建、操作创建、区域/块管理等 API
- 支持插入点管理，用于在特定位置插入操作

**主要方法**:
- 类型创建: `getI32Type()`, `getF32Type()`, `getBufferType()`, `getFunctionType()`
- 常量创建: `createI32Constant()`, `createF32Constant()`, `createBoolConstant()`
- 算术操作: `createAddOp()`, `createSubOp()`, `createMulOp()`, `createDivOp()`, `createCmpOp()`
- 内存操作: `createAllocOp()`, `createLoadOp()`, `createStoreOp()`
- 控制流: `createBranchOp()`, `createCondBranchOp()`, `createReturnOp()`
- 并行操作: `createParallelForOp()`, `createReduceOp()`
- 设备操作: `createSpawnOp()`, `createSyncOp()`, `createMoveToOp()`

**构建命令**:
```bash
cd HSCIR
cmake -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build
```

---

### HSCLang - 编程语言

**路径**: `HSCLang/`

**设计哲学**:
- 显式与隐式的平衡：关键异构特征显式表达，底层调度隐式优化
- 渐进式暴露：初学者使用简单范式，专家可深入调优

**借鉴的语言特性**:
| 语言/框架 | 借鉴特性 |
|----------|---------|
| Rust | 表达式导向、强类型、所有权 |
| Kokkos | 执行模式+执行策略+计算体分离 |
| Unison | 内容哈希标识、分布式透明 |
| SYCL/OpenMP | 单一源、目标注解 |

**核心语法元素**:
- `task` - 任务定义
- `pipeline` - 流水线并行
- `graph` - 显式依赖图
- `spawn on <device>` - 设备指定执行
- `Buffer<T>` - 多维缓冲区

**任务三要素**:
1. **执行模式 (Pattern)**: `For`、`Reduce`、`Scan`、`TaskGraph`
2. **执行策略 (Policy)**: `device_hint`、`granularity`、`priority`
3. **计算体 (Body)**: 实际计算代码

**示例项目**: `HSCLang/examples/CFD_AI_SIM/`

---

### HSCMake - 构建系统

**路径**: `HSCMake/`

**技术栈**: Python 3.13+, Click, pex

**安装**:
```bash
cd HSCMake
pip install -e .
# 或使用 uv
uv sync 或 uv sync --dev
```

**CLI 命令**:
```bash
# 配置项目
hscmake configure [--build-dir=build] [HSCMakeList.txt]

# 构建目标
hscmake build [--build-dir=build] [targets...]

# 清理构建
hscmake clean [--build-dir=build]

# 列出所有目标
hscmake list [--build-dir=build]
```

**配置文件格式** (`HSCMakeList.txt`):
```
project(name="example", version="0.1.0")

cpp_target(
    name="main",
    srcs=["src/main.cpp"],
    deps=[":lib"]
)

rust_target(
    name="lib",
    srcs=["src/lib.rs"]
)
```

**核心模块**:
- `parser.py` - HSCMakeList.txt 解析器，使用 Python `ast` 模块解析
- `model.py` - 项目模型，定义 `Project`、`Target`、`Language` 等
- `builder.py` - 构建规划与执行
- `rules.py` - 构建规则
- `cli.py` - 命令行接口

---

### HSCIDE - IDE 与渲染管线

**路径**: `HSCIDE/`

**组件**:
- `ide/HSC Studio/` - 主 IDE（.slnx 解决方案）
- `ide/backend/BBF/` - Python 后端
- `ide/backend/gateway/` - Go 网关
- `RenderPipeline/` - 渲染管线
  - `client/VulkanRenderer/` - Vulkan 渲染器客户端
  - `client/GoDownloader/` - Go 下载器
  - `server/` - gRPC 流式 PNG 服务（Go）

**渲染服务** (端口 50051):
```go
// 流式传输 PNG 帧
rpc GetPNGStream(PNGRequest) returns (stream PNGChunk)
```

---

## 开发约定

### 代码风格

**Rust (HSCC)**:
- 使用 `anyhow::Result` 进行错误处理
- 模块化设计：`lexer`, `parser`, `ast`, `codegen`, `compile`
- 使用 `extern crate` 声明外部依赖

**C++ (HSCIR)**:
- C++23 标准
- 使用智能指针 (`shared_ptr`) 管理类型对象
- 遵循 RAII 原则
- 命名空间: `hscir`

**Python (HSCMake)**:
- Python 3.13+
- 使用 `click` 库构建 CLI
- 类型注解
- 使用 `pickle` 序列化项目状态

**Go (HSCIDE 后端)**:
- gRPC 服务定义
- 标准库优先

### 构建与测试

**编译器测试**:
```bash
cd HSCC/hscc
cargo test
```

**构建系统测试**:
```bash
cd HSCMake
uv run pytest
# 或手动测试
hscmake configure --build-dir=test/build test/HSCMakeList.txt
```

---

## 常用开发任务

### 添加新的编译器前端特性

1. 在 `HSCC/hscc/src/lexer.rs` 添加新词法单元到 `TokenKind` 枚举
2. 在 `HSCC/hscc/src/parser.rs` 添加解析逻辑
3. 在 `HSCC/hscc/src/ast.rs` 定义 AST 节点
4. 在 `HSCC/hscc/src/codegen.rs` 实现代码生成

### 添加新的 HSCIR 类型

1. 在 `HSCIR/include/hscir/Types.h` 定义类型类
2. 在 `HSCIR/src/Types.cpp` 实现 `toString()` 和 `operator==`
3. 在 `TypeManager` 添加类型缓存逻辑

### 添加新的构建目标类型

1. 在 `HSCMake/hscmake/model.py` 定义目标模型
2. 在 `HSCMake/hscmake/rules.py` 添加构建规则
3. 在 `HSCMake/hscmake/builder.py` 实现构建逻辑

---

## 未来路线图

### 短期目标 (2026 Q2)

1. **HSCIR 与编译器集成**
   - 完成 AST 到 HSCIR 的完整转换管道
   - 实现基于 HSCIR 的设备无关优化 Pass
   - 建立 HSCIR 到各后端（CUDA/NPU/Triton）的代码生成

2. **运行时调度器开发**
   - 实现设备探测与特征建模 (`probe_devices()`)
   - 构建基于任务特征的设备选择器 (`select_device()`)
   - 开发跨设备数据迁移优化器

3. **NPU 后端完善**
   - 支持更多 NPU 架构（华为昇腾、Google TPU）
   - 优化 ONNX 模型生成和推理性能
   - 实现 NPU 特定优化（算子融合、内存布局优化）

### 中期目标 (2026 Q3-Q4)

1. **FPGA 后端实现**
   - 基于 Vitis HLS 的代码生成器
   - `task` 到 HLS 顶层模块的映射
   - `Buffer` 到 BRAM/AXI 接口的转换
   - 流水线并行模式的硬件优化

2. **高级语言特性**
   - 内容寻址任务标识（借鉴 Unison）
   - 编译期元编程（模板展开、目标特化）
   - 递归任务与动态图调整

3. **性能优化**
   - 引入 cuBLAS/rocBLAS 等高性能库
   - 支持低精度计算（FP16、BF16、INT8）
   - 实现算子融合和 CUDA Graph 优化

### 长期愿景 (2027+)

1. **MLIR 集成**
   - 定义 `hsc` 方言（`hsc.task`、`hsc.parallel_for`、`hsc.buffer`）
   - 渐进式降低到通用方言（`scf`、`linalg`、`memref`）
   - 利用 MLIR 多后端基础设施

2. **分布式异构计算**
   - 跨节点任务调度与数据迁移
   - 统一内存空间（CXL 2.0+ 支持）
   - 容错与任务恢复机制

3. **生态建设**
   - 光子学仿真、CFD、AI 训练等领域专用库
   - 第三方插件市场
   - 工业界采用与社区贡献

---

## 注意事项

- 此项目为**学习项目**，可能存在较多 BUG 和未完善功能
- 项目处于活跃开发状态，API 可能随时变化，请参考最新代码
- 主要支持 Windows 平台开发，Linux/macOS 平台可能需要进行适配
- HSCC 编译器已实现核心编译流水线，包含词法/语法/语义分析，支持 CUDA/HIP/Triton/NPU 后端代码生成
- 测试覆盖包括单元测试和集成测试，端到端测试正在完善中
- 文档可能与实际实现存在滞后，建议结合代码阅读

---

## 相关文档

### 核心文档
- 项目总览: `README.md` (本文档)
- 语言设计规范: `HSCLang/README.md`
- 语言详解: `HSCLang/README_EXPLAIN.md`
- 编译器文档: `HSCC/README.md`
- 中间表示文档: `HSCIR/README.md`
- 构建系统文档: `HSCMake/README.md`
- IDE 与渲染管线文档: `HSCIDE/README.md`
- 插件开发指南: `HSCPlugins/README.md`

### 设计文档
- 光子学仿真模块设计: `docs/光子学仿真模块设计.md`
- 代码分析与优化设计: `docs/代码分析与优化.md`
- DeepWiki 深度分析: `docs/DeepWiki/2026-03-05.md`

### 开发计划
- FPGA 开发计划: `docs/TODO.FPGA.md`
- NPU 开发计划: `docs/TODO.NPU.md`
- MLIR 集成计划: `docs/TODO.MLIR.md`
- 总体开发路线图: `docs/TODO.md`

### 其他
- 开发者指南: `docs/DEVELOPER_GUIDE.md` (待更新)
- 项目上下文: `docs/AGENTS.md` (与 README 内容相似，可参考)