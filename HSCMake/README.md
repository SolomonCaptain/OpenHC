# HSCMake - HSCLang 构建系统

> 跨语言构建系统，支持 C++、Rust、TypeScript 等多语言项目的统一构建。

---

## 概述

HSCMake 是 OpenHC 项目的构建系统，提供统一的构建接口，支持多语言、多目标的项目构建。目前支持 C++、Rust、TypeScript 等语言，并可通过扩展支持 HSCLang 项目的构建（通过调用 HSCC 编译器）。HSCMake 本身也是 OpenHC 多语言工程（Rust 编译器 + C++ IR + Python 工具 + Go 服务）的统一构建入口。

## 技术栈

- **语言**: Python 3.13+
- **CLI 框架**: Click
- **打包工具**: pex

## 依赖

| 依赖 | 版本 | 用途 |
|------|------|------|
| `click` | >=8.1.0 | 命令行接口 |
| `pex` | >=2.89.1 | 打包工具 |

## 目录结构

```
HSCMake/
├── hscmake/           # 主模块
│   ├── __init__.py
│   ├── __main__.py    # 入口点
│   ├── cli.py         # 命令行接口
│   ├── parser.py      # 配置解析器
│   ├── model.py       # 数据模型
│   ├── builder.py     # 构建执行器
│   └── rules.py       # 构建规则
├── test/              # 测试项目
│   ├── HSCMakeList.txt
│   └── src/
├── pyproject.toml     # 项目配置
└── uv.lock            # 依赖锁定
```

## 安装

### 使用 pip

```bash
cd HSCMake
pip install -e .
```

### 使用 uv

```bash
cd HSCMake
uv sync
# 或开发模式
uv sync --dev
```

## CLI 命令

### configure - 配置项目

解析 `HSCMakeList.txt` 并生成构建规则：

```bash
hscmake configure [--build-dir=build] [HSCMakeList.txt]
```

### build - 构建目标

构建指定目标或所有目标：

```bash
hscmake build [--build-dir=build] [targets...]
```

### clean - 清理构建

删除构建目录：

```bash
hscmake clean [--build-dir=build]
```

### list - 列出目标

列出所有可用的构建目标：

```bash
hscmake list [--build-dir=build]
```

## 配置文件格式

`HSCMakeList.txt` 使用 Python 语法的 DSL：

```python
# 项目定义
project(name="example", version="0.1.0")

# 可执行目标
add_executable("main"
    SOURCES=["src/main.cpp"],
    LANGUAGE="CPP"
)

# 带依赖的目标
cpp_target(
    name="app",
    srcs=["src/app.cpp"],
    deps=[":lib"]
)

# Rust 目标
rust_target(
    name="lib",
    srcs=["src/lib.rs"]
)

# TypeScript 目标
ts_target(
    name="frontend",
    srcs=["src/index.ts"]
)

# HSCLang 目标（通过调用 HSCC 编译器）
hsc_target(
    name="simulation",
    srcs=["src/main.hl"],
    backend="cuda",      # 可选: cuda, hip, triton, npu
    arch="sm_61"
)
```

## 核心模块

### parser.py - 配置解析器

使用 Python `ast` 模块解析 `HSCMakeList.txt`：
- `HSCMakeParser` 类：访问 AST 节点
- `project()` 函数处理
- `add_executable()` 函数处理

### model.py - 数据模型

定义项目模型：
- `Project` - 项目
- `Target` - 构建目标
- `TargetType` - 目标类型（EXECUTABLE, LIBRARY）
- `Language` - 语言类型（CPP, RUST, TYPESCRIPT）
- `SourceFile` - 源文件

### builder.py - 构建执行器

- `BuildPlanner` - 构建规划器，创建依赖图
- `BuildExecutor` - 构建执行器，执行构建规则

### rules.py - 构建规则

定义各语言的构建规则和编译命令。

### cli.py - 命令行接口

使用 `click` 库实现 CLI：
- `configure` 命令
- `build` 命令
- `clean` 命令
- `list` 命令

## 测试

```bash
cd HSCMake

# 使用 pytest
uv run pytest

# 手动测试
hscmake configure --build-dir=test/build test/HSCMakeList.txt
hscmake build --build-dir=test/build
```

## 示例项目

参考 `HSCLang/examples/CFD_AI_SIM/HSCMakeList.txt`：

```python
project("CfdAiSim", VERSION="0.1.0", LANGUAGE=["HSCLang"])

add_executable("cfd_ai_sim"
    SOURCES=["src/main.hl"]
)
```

## 相关文档

- 编译器: `HSCC/README.md`
- 语言设计: `HSCLang/README.md`
