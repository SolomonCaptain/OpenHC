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
   代码生成器 (codegen.rs)
         ↓
    CUDA 代码 (.cu)
         ↓
    NVCC 编译 (compile.rs)
         ↓
     可执行文件
```

## 核心模块

### lexer.rs - 词法分析器

定义 `TokenKind` 枚举，包括：
- **关键字**: `fn`, `let`, `mut`, `if`, `else`, `while`, `loop`, `task`, `spawn`, `pipeline`, `graph`, `parallel`, `for` 等
- **类型关键字**: `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `f32`, `f64`, `bool`, `char`, `Buffer` 等
- **设备关键字**: `GPU`, `NPU`, `FPGA`, `CPU`, `Host`, `DeviceLocal` 等
- **符号**: `+`, `-`, `*`, `/`, `=`, `==`, `!=`, `<`, `>`, `<=`, `>=`, `->`, `=>`, `::` 等

### parser.rs - 语法分析器

解析顶层声明：
- `import` - 导入声明
- `fn` - 函数定义
- `task` - 任务定义

### ast.rs - 抽象语法树

核心 AST 节点：
- `Program` - 程序（包含 imports、functions、tasks）
- `Function` - 函数定义
- `Task` - 任务定义（包含 pattern、policy、body）
- `Pattern` - 执行模式
- `Policy` - 执行策略
- `Type` - 类型系统

### codegen.rs - 代码生成器

将 AST 转换为目标代码，当前主要生成 CUDA C++ 代码。

### compile.rs - 编译驱动

调用 NVCC 编译器将生成的 CUDA 代码编译为可执行文件。

### config.rs - 配置解析

解析项目配置文件 `HSCC.toml`。

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

`HSCC.toml`:

```toml
[package]
name = "project_name"
version = "0.1.0"
edition = "2026"

[target]
device = "cuda"    # 目标设备: cuda, fpga, npu
arch = "sm_61"     # 设备架构
```

## 测试

```bash
cargo test
```

## 相关文档

- 语言设计规范: `HSCLang/README.md`
- 中间表示: `HSCIR/README.md`
- FPGA 开发计划: `docs/TODO.FPGA.md`
- GPU 开发计划: `docs/TODO.GPU.md`
- MLIR 集成计划: `docs/TODO.MLIR.md`
