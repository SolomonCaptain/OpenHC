# OpenHC 开发者指南

本文档详细介绍如何参与 OpenHC 异构计算平台项目的开发。

---

## 目录

1. [快速入门](#1-快速入门)
2. [开发环境配置](#2-开发环境配置)
3. [项目架构](#3-项目架构)
4. [编译器开发 (HSCC)](#4-编译器开发-hscc)
5. [中间表示开发 (HSCIR)](#5-中间表示开发-hscir)
6. [构建系统开发 (HSCMake)](#6-构建系统开发-hscmake)
7. [IDE 开发 (HSCIDE)](#7-ide-开发-hscide)
8. [测试指南](#8-测试指南)
9. [代码风格](#9-代码风格)
10. [贡献流程](#10-贡献流程)

---

## 1. 快速入门

### 1.1 克隆仓库

```bash
git clone https://github.com/SolomonCaptain/OpenHC.git
cd OpenHC
```

### 1.2 构建编译器

```bash
cd HSCC/hscc
cargo build --release
```

### 1.3 构建中间表示

```bash
cd HSCIR
cmake -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build
```

### 1.4 安装构建系统

```bash
cd HSCMake
pip install -e .
# 或使用 uv
uv sync
```

### 1.5 运行测试

```bash
# 编译器测试
cd HSCC/hscc
cargo test

# 构建系统测试
cd HSCMake
uv run pytest
```

---

## 2. 开发环境配置

### 2.1 必需工具

| 工具 | 版本要求 | 用途 |
|------|---------|------|
| Rust | 1.75+ (Edition 2024) | HSCC 编译器开发 |
| Python | 3.13+ | HSCMake 构建系统 |
| CMake | 3.25+ | HSCIR 构建 |
| C++ 编译器 | MSVC 2022 / GCC 13 / Clang 16 | HSCIR 开发 |
| Git | 2.40+ | 版本控制 |

### 2.2 可选工具

| 工具 | 版本要求 | 用途 |
|------|---------|------|
| CUDA Toolkit | 12.0+ | GPU 后端开发 |
| ROCm | 6.0+ | AMD GPU 支持 |
| Vitis HLS | 2023.1+ | FPGA 后端开发 |
| .NET SDK | 8.0+ | IDE 开发 |
| Go | 1.21+ | 渲染管线后端 |
| Vulkan SDK | 1.3+ | 渲染器开发 |

### 2.3 IDE 配置

**推荐 IDE**:
- **Rust**: RustRover / VS Code + rust-analyzer
- **C++**: CLion / Visual Studio / VS Code + clangd
- **Python**: PyCharm / VS Code + Python 扩展
- **Go**: GoLand / VS Code + Go 扩展

**IDE 配置文件**:
- `.idea/` - JetBrains IDE 配置
- `.vscode/` - VS Code 配置（如果存在）

---

## 3. 项目架构

### 3.1 整体架构

```
                    ┌─────────────────────────────────────────┐
                    │            HSCLang 源代码                │
                    └─────────────────────────────────────────┘
                                        │
                                        ▼
┌─────────────────────────────────────────────────────────────────┐
│                      HSCC 编译器 (Rust)                          │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐            │
│  │  Lexer  │─▶│ Parser  │─▶│ Typeck  │─▶│ Lower   │            │
│  └─────────┘  └─────────┘  └─────────┘  └─────────┘            │
│                                        │                        │
│                                        ▼                        │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │                    HSCIR (C++ IR)                        │  │
│  └─────────────────────────────────────────────────────────┘  │
│                                        │                        │
│           ┌────────────┬───────────────┼───────────────┐       │
│           ▼            ▼               ▼               ▼       │
│  ┌─────────────┐ ┌───────────┐ ┌───────────┐ ┌───────────┐   │
│  │ CUDA 后端   │ │Triton 后端│ │ NPU 后端  │ │ FPGA 后端 │   │
│  └─────────────┘ └───────────┘ └───────────┘ └───────────┘   │
└─────────────────────────────────────────────────────────────────┘
                                        │
                                        ▼
                    ┌─────────────────────────────────────────┐
                    │              可执行文件                  │
                    └─────────────────────────────────────────┘
```

### 3.2 模块依赖关系

```
HSCC (编译器)
    │
    ├── 依赖 HSCIR (通过 C API)
    │
    └── 生成代码供 HSCMake 构建

HSCIR (中间表示)
    │
    └── 独立的 C++ 库

HSCMake (构建系统)
    │
    ├── 构建 HSCIR
    ├── 构建 HSCC 生成的代码
    └── 构建示例项目

HSCIDE (IDE)
    │
    ├── 调用 HSCC 进行编译
    ├── 调用 HSCMake 进行构建
    └── 使用 RenderPipeline 进行可视化
```

---

## 4. 编译器开发 (HSCC)

### 4.1 目录结构

```
HSCC/hscc/src/
├── main.rs           # 入口点
├── lexer.rs          # 词法分析器
├── parser.rs         # 语法分析器
├── ast.rs            # 抽象语法树定义
├── typeck.rs         # 类型检查器
├── lower.rs          # AST → HSCIR 转换
├── codegen.rs        # CUDA/HIP 代码生成
├── compile.rs        # 编译驱动
├── config.rs         # 配置文件解析
├── hscir/            # HSCIR Rust 绑定
│   ├── mod.rs
│   └── ffi.rs
├── triton/           # Triton DSL 后端
│   ├── mod.rs
│   ├── types.rs
│   └── generator.rs
└── npu/              # NPU 后端
    ├── mod.rs
    ├── types.rs
    ├── graph.rs
    └── backends/
        ├── mod.rs
        ├── ascend.rs
        └── tpu.rs
```

### 4.2 添加新词法单元

1. 在 `lexer.rs` 中添加到 `TokenKind` 枚举：

```rust
#[derive(Debug, PartialEq, Clone)]
pub enum TokenKind {
    // 关键字
    // ... 现有关键字 ...
    MyNewKeyword,  // 新增关键字
    
    // ... 其他类型 ...
}
```

2. 在 `lexer.rs` 的关键字映射中添加：

```rust
fn lookup_keyword(ident: &str) -> TokenKind {
    match ident {
        // ... 现有映射 ...
        "mynewkeyword" => TokenKind::MyNewKeyword,
        _ => TokenKind::Identifier(ident.to_string()),
    }
}
```

### 4.3 添加新语法节点

1. 在 `ast.rs` 中定义 AST 节点：

```rust
#[derive(Debug)]
pub struct MyNewNode {
    pub name: String,
    pub body: Block,
}
```

2. 在 `parser.rs` 中添加解析逻辑：

```rust
fn parse_my_new_node(&mut self) -> Result<MyNewNode, ParseError> {
    // 解析逻辑
}
```

3. 在 `typeck.rs` 中添加类型检查：

```rust
fn check_my_new_node(&mut self, node: &MyNewNode) -> Result<Type, TypeckError> {
    // 类型检查逻辑
}
```

4. 在 `lower.rs` 中添加 IR 转换：

```rust
fn lower_my_new_node(&mut self, node: &MyNewNode) -> Result<Operation, LowerError> {
    // IR 转换逻辑
}
```

### 4.4 添加新后端

1. 创建新后端模块：

```rust
// HSCC/hscc/src/mybackend/mod.rs
pub mod types;
pub mod generator;

use crate::ast::Program;

pub struct MyBackend {
    // 后端状态
}

impl MyBackend {
    pub fn new() -> Self { /* ... */ }
    
    pub fn generate(&self, program: &Program) -> String {
        // 代码生成逻辑
    }
}
```

2. 在 `main.rs` 中集成：

```rust
match backend {
    Backend::Cuda => { /* ... */ }
    Backend::Triton => { /* ... */ }
    Backend::Npu => { /* ... */ }
    Backend::MyBackend => {
        let backend = mybackend::MyBackend::new();
        let code = backend.generate(&ast);
        // 输出代码
    }
}
```

---

## 5. 中间表示开发 (HSCIR)

### 5.1 目录结构

```
HSCIR/
├── include/hscir/
│   ├── Types.h       # 类型系统
│   ├── Operations.h  # 操作定义
│   ├── Builder.h     # IR 构建器
│   └── CAPI.h        # C API
├── src/
│   ├── Types.cpp
│   ├── Operations.cpp
│   ├── Builder.cpp
│   ├── Module.cpp
│   └── CAPI.cpp
└── targets/          # 构建输出
```

### 5.2 添加新类型

1. 在 `Types.h` 中定义类型类：

```cpp
class MyNewType : public Type {
public:
    MyNewType(/* 参数 */) 
        : Type(Kind::MyNew), /* 初始化成员 */ {}
    
    // 访问器
    const std::string& getName() const { return name_; }
    
    // 必须实现的接口
    std::string toString() const override;
    bool operator==(const MyNewType&) const;

private:
    std::string name_;
    // 其他成员
};
```

2. 在 `Type::Kind` 枚举中添加：

```cpp
enum class Kind {
    Integer,
    Float,
    Buffer,
    Function,
    None,
    MyNew,  // 新增
};
```

3. 在 `Types.cpp` 中实现：

```cpp
std::string MyNewType::toString() const {
    return "my_new_type";
}

bool MyNewType::operator==(const MyNewType& other) const {
    return name_ == other.name_;
}
```

### 5.3 添加新操作

1. 在 `Operations.h` 中定义操作类：

```cpp
class MyNewOp : public Operation {
public:
    MyNewOp(/* 参数 */) 
        : Operation("my_new") {
        // 初始化操作
    }
    
    // 特定于操作的接口
    Value* getInput() { return getOperands()[0].get(); }
    Value* getOutput() { return getResults()[0].get(); }
    
    void print(std::ostream& os, unsigned indent = 0) const override;
};
```

2. 在 `Builder.h` 中添加创建方法：

```cpp
class Builder {
public:
    // ...
    
    std::shared_ptr<MyNewOp> createMyNewOp(
        std::shared_ptr<Value> input,
        std::shared_ptr<Type> resultType
    );
};
```

3. 在 `Operations.cpp` 中实现：

```cpp
void MyNewOp::print(std::ostream& os, unsigned indent) const {
    os << std::string(indent, ' ') << "my_new ";
    // 打印操作数和结果
}
```

---

## 6. 构建系统开发 (HSCMake)

### 6.1 目录结构

```
HSCMake/hscmake/
├── __init__.py
├── __main__.py       # 入口点
├── cli.py            # 命令行接口
├── parser.py         # HSCMakeList.txt 解析器
├── model.py          # 项目模型
├── builder.py        # 构建执行器
└── rules.py          # 构建规则
```

### 6.2 添加新目标类型

1. 在 `model.py` 中定义目标类型：

```python
from dataclasses import dataclass
from enum import Enum

class TargetType(Enum):
    CPP = "cpp"
    RUST = "rust"
    TYPESCRIPT = "typescript"
    MYNEW = "mynew"  # 新增

@dataclass
class Target:
    name: str
    type: TargetType
    language: Language
    sources: List[SourceFile] = field(default_factory=list)
    # ...
```

2. 在 `parser.py` 中添加解析支持：

```python
def parse_mynew_target(self, call_node: ast.Call) -> Target:
    """解析 mynew_target() 调用"""
    # 解析参数
    name = self.get_kwarg(call_node, "name")
    srcs = self.get_kwarg(call_node, "srcs", [])
    # ...
    return Target(
        name=name,
        type=TargetType.MYNEW,
        # ...
    )
```

3. 在 `rules.py` 中添加构建规则：

```python
def get_mynew_build_commands(target: Target, project: Project) -> List[BuildCommand]:
    """生成 MyNew 目标的构建命令"""
    commands = []
    # 构建逻辑
    return commands
```

4. 在 `builder.py` 中集成：

```python
def build_target(self, target: Target) -> bool:
    if target.type == TargetType.MYNEW:
        commands = rules.get_mynew_build_commands(target, self.project)
        return self.execute_commands(commands)
    # ...
```

---

## 7. IDE 开发 (HSCIDE)

### 7.1 目录结构

```
HSCIDE/
├── ide/
│   ├── HSC Studio/       # 主 IDE (C#/.NET)
│   │   ├── HSC Studio.slnx
│   │   └── ...
│   └── backend/
│       ├── BBF/          # Python 后端
│       └── gateway/      # Go 网关
└── RenderPipeline/
    ├── client/
    │   ├── VulkanRenderer/   # Vulkan 渲染器
    │   └── GoDownloader/     # Go 下载器
    └── server/               # gRPC 服务端 (Go)
```

### 7.2 渲染管线架构

```
┌──────────────┐     gRPC      ┌──────────────┐
│   Client     │◄─────────────►│    Server    │
│  (Python)    │               │    (Go)      │
└──────────────┘               └──────────────┘
       │                              │
       │ 共享内存                     │ 文件系统
       ▼                              ▼
┌──────────────┐               ┌──────────────┐
│ VulkanRenderer│              │   PNG 文件   │
│    (C++)      │              │              │
└──────────────┘               └──────────────┘
```

---

## 8. 测试指南

### 8.1 编译器测试

```bash
cd HSCC/hscc

# 运行所有测试
cargo test

# 运行特定测试
cargo test test_lexer
cargo test test_parser
cargo test test_codegen

# 运行并显示输出
cargo test -- --nocapture

# 测试覆盖率
cargo tarpaulin --out Html
```

### 8.2 构建系统测试

```bash
cd HSCMake

# 运行所有测试
uv run pytest

# 运行特定测试
uv run pytest tests/test_parser.py

# 显示覆盖率
uv run pytest --cov=hscmake
```

### 8.3 端到端测试

```bash
# 编译示例项目
cd HSCC/hscc
cargo run -- ../../HSCLang/examples/simple

# 验证输出
ls output/
```

---

## 9. 代码风格

### 9.1 Rust 代码风格

```bash
# 格式化代码
cargo fmt

# 静态检查
cargo clippy
```

**命名约定**:
- 类型: `PascalCase`
- 函数/变量: `snake_case`
- 常量: `SCREAMING_SNAKE_CASE`
- 模块: `snake_case`

### 9.2 C++ 代码风格

```bash
# 格式化代码
clang-format -i src/*.cpp include/hscir/*.h
```

**命名约定**:
- 类: `PascalCase`
- 函数: `camelCase`
- 变量: `snake_case_`
- 常量: `kPascalCase`
- 命名空间: `lowercase`

### 9.3 Python 代码风格

```bash
# 格式化代码
black hscmake/

# 类型检查
mypy hscmake/
```

**命名约定**:
- 类: `PascalCase`
- 函数/变量: `snake_case`
- 常量: `SCREAMING_SNAKE_CASE`

---

## 10. 贡献流程

### 10.1 创建分支

```bash
# 从 main 创建功能分支
git checkout main
git pull origin main
git checkout -b feature/my-feature
```

### 10.2 提交代码

```bash
# 添加修改
git add .

# 提交（使用语义化提交信息）
git commit -m "feat(compiler): add support for new keyword"

# 推送到远程
git push origin feature/my-feature
```

### 10.3 提交信息格式

使用 [Conventional Commits](https://www.conventionalcommits.org/) 格式：

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

**类型**:
- `feat`: 新功能
- `fix`: Bug 修复
- `docs`: 文档更新
- `style`: 代码风格（不影响功能）
- `refactor`: 重构
- `test`: 测试相关
- `chore`: 构建/工具相关

**范围**:
- `compiler`: HSCC 编译器
- `ir`: HSCIR 中间表示
- `build`: HSCMake 构建系统
- `ide`: HSCIDE
- `docs`: 文档

### 10.4 创建 Pull Request

1. 在 GitHub 上创建 Pull Request
2. 填写 PR 模板
3. 等待 CI 通过
4. 等待代码审查
5. 合并到 main

---

## 常见问题

### Q: 编译 HSCIR 时找不到 C++23 编译器？

确保安装了支持的编译器：
- Windows: Visual Studio 2022 (17.8+)
- Linux: GCC 13+ 或 Clang 16+
- macOS: Clang 16+ (Xcode 15+)

### Q: Rust 编译失败？

检查 Rust 版本：
```bash
rustc --version  # 应该是 1.75+
rustup update
```

### Q: Python 依赖问题？

使用 uv 管理依赖：
```bash
cd HSCMake
uv sync --dev
```

### Q: 如何调试编译器？

使用 `RUST_LOG` 环境变量：
```bash
RUST_LOG=debug cargo run -- <project-dir>
```

---

*最后更新: 2026-03-07*