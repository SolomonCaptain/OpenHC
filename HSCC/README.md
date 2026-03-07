# HSCC - HSCLang 编译器

> HSCLang 编译器，将 HSCLang 源代码编译到 CUDA、FPGA、NPU 等异构设备。

---

## 概述

HSCC 是 OpenHC 项目的编译器组件，负责将 HSCLang 源代码（`.hl` 文件）编译为可在异构设备上执行的代码。

## 技术栈

- **语言**: Rust (Edition 2024)
- **构建工具**: Cargo

## 依赖

| 依赖 | 版本 | 用途 |
|------|------|------|
| `toml` | 1.0.1 | 配置文件解析 |
| `serde` | 1.0.228 | 序列化/反序列化 |
| `regex` | 1.12.3 | 词法分析 |
| `anyhow` | 1.0.101 | 错误处理 |

## 编译流程

```
HSCLang 源文件 (.hl)
         ↓
   词法分析器 (lexer.rs)
         ↓
     Token 流
         ↓
   语法分析器 (parser.rs)
         ↓
   抽象语法树 (ast.rs)
         ↓
   类型检查器 (typeck.rs)
         ↓
   静态分析 (analysis.rs, dataflow.rs, semantic.rs)
         ↓
   ┌─────────────────────────────────────────────────────────┐
   │ 多后端代码生成                                          │
   │                                                         │
   ▼                         ▼                               ▼
CUDA/HIP 后端 (codegen.rs)    NPU 后端 (npu/)                Triton 后端 (triton/)
         ↓                         ↓                               ↓
CUDA/HIP C++ 代码 (.cu)      ONNX/OpenVINO IR                Python Triton 代码
         ↓                         ↓                               ↓
nvcc/hipcc 编译               Python 运行时代码               直接执行
         ↓                         ↓
     可执行文件               执行脚本
```

**支持的后端**:
- **CUDA/HIP**: 生成 CUDA/HIP C++ 代码，调用 nvcc/hipcc 编译
- **Triton**: 生成 Python Triton 代码，支持设备无关 GPU 编程
- **NPU**: 生成 ONNX 模型和 OpenVINO IR，支持 Intel NPU 推理加速
- **FPGA**: *规划中*，基于 Vitis HLS 的硬件代码生成

## 核心模块

### 前端模块
- **lexer.rs**: 词法分析器，定义 `TokenKind` 枚举（异构专用关键字、符号、字面量）
- **parser.rs**: 语法分析器，解析 `import`、`fn`、`task`、`pipeline`、`graph` 等声明
- **ast.rs**: 抽象语法树定义，包括 `Program`、`Function`、`Task`、`Pattern`、`Policy`、`Buffer` 等节点
- **typeck.rs**: 类型检查器，实现类型推断、兼容性检查、错误报告

### 静态分析模块
- **analysis.rs**: 静态分析，包括模式-策略检查、依赖分析
- **dataflow.rs**: 数据流分析，跟踪变量定义和使用
- **semantic.rs**: 语义分析，验证语言语义规则
- **target_check.rs**: 目标设备特定检查，确保代码符合设备约束
- **performance.rs**: 性能分析，评估任务计算复杂度和内存访问模式
- **diagnostic.rs**: 诊断信息收集与报告，提供友好的错误信息

### 中间表示模块
- **lower.rs**: AST 到 HSCIR 转换，实现程序、任务、控制流转换
- **hscir/**: HSCIR 中间表示模块，包含 Pass 管理器和分析工具
  - `pass/`: Pass 管理器，支持数据流分析、依赖分析、设备亲和性分析等
  - `builder/`: IR 构建器，提供类型创建、操作创建等 API

### 代码生成模块
- **codegen.rs**: CUDA/HIP 代码生成器，将 AST 转换为 GPU 代码
- **compile.rs**: 编译驱动，调用 nvcc/hipcc 编译 CUDA/HIP 代码
- **triton/**: Triton 后端，生成 Python Triton 代码，支持设备无关 GPU 编程
  - `lowering.rs`: AST 到 Triton Python 代码的转换
  - `codegen.rs`: Triton 内核生成器
- **npu/**: NPU 后端，生成 ONNX/OpenVINO IR 和运行时代码
  - `backend.rs`: NPU 后端抽象接口
  - `intel_npu.rs`: Intel NPU 后端实现
  - `lowering.rs`: AST 到 NPU 计算图的转换
  - `autotuner.rs`: 自动调优器，优化 NPU 执行参数

### 配置与工具模块
- **config.rs**: 配置文件解析（HSCC.toml），命令行参数处理
- **main.rs**: 编译器主入口，协调各模块执行流程

## 构建

```bash
cd HSCC/hscc
cargo build --release
```

## 使用

```bash
hscc <project-directory>
```

项目目录结构示例：
```
my_project/
├── HSCC.toml      # 项目配置
└── src/
    └── main.hl    # 源文件
```

## 配置文件格式

`HSCC.toml` 配置文件示例：

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

[npu]                       # NPU 特定配置（当 device = "npu" 时生效）
vendor = "intel"            # NPU 厂商: intel, huawei, google
precision = "fp16"          # 计算精度: fp32, fp16, int8
memory_layout = "nhwc"      # 内存布局: nhwc, nchw

[triton]                    # Triton 特定配置
use_tensor_cores = true     # 是否使用张量核心
num_warps = 4               # warp 数量
num_stages = 3              # 流水线阶段数
```

## 测试

```bash
cargo test
```

## 相关文档

- **语言设计规范**: `HSCLang/README.md`
- **语言详解**: `HSCLang/README_EXPLAIN.md`
- **中间表示**: `HSCIR/README.md`
- **构建系统**: `HSCMake/README.md`
- **插件系统**: `HSCPlugins/README.md`
- **开发计划**: `docs/TODO.md` (总体路线图)
- **FPGA 开发计划**: `docs/TODO.FPGA.md`
- **NPU 开发计划**: `docs/TODO.NPU.md`
- **MLIR 集成计划**: `docs/TODO.MLIR.md`
- **代码分析与优化设计**: `docs/代码分析与优化.md`
