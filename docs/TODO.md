# OpenHC 开发路线图

> 异构计算平台项目开发优先级与实施计划

---

## 项目成熟度评估

| 子项目 | 语言 | 完成度 | 测试覆盖 | 优先级 |
|--------|------|--------|----------|--------|
| HSCC (编译器) | Rust | 40% | ❌ 无 | 🔴 高 |
| HSCIR (中间表示) | C++23 | 50% | ❌ 无 | 🔴 高 |
| HSCLang (语言规范) | - | 80% | - | 🟡 中 |
| HSCMake (构建系统) | Python | 70% | ❌ 无 | 🟡 中 |
| HSCIDE (IDE) | 多语言 | 20% | ❌ 无 | 🟢 低 |

---

## 一、紧急优先级 (P0)

### 1.1 HSCC 编译器核心完善

#### 类型检查器实现
- [x] 实现类型推断引擎
- [x] 实现类型兼容性检查
- [x] 添加 Buffer 类型形状检查
- [x] 添加语义错误报告机制
- [x] 添加类型可视化调试输出

**文件**: `HSCC/hscc/src/typeck.rs` (新建)

#### AST 到 HSCIR 转换
- [x] 设计 AST -> HSCIR 映射规则
- [x] 实现 `Program` -> `Module` 转换
- [x] 实现 `Task` -> `Operation` 转换
- [x] 实现 `Buffer` 类型映射
- [x] 实现控制流转换 (for/if/while -> Block)

**文件**: `HSCC/hscc/src/lower.rs` (新建)

#### 编译器测试框架
- [x] 添加词法分析器单元测试
- [x] 添加语法分析器单元测试
- [x] 添加代码生成器集成测试
- [x] 添加端到端编译测试
- [x] 添加错误恢复测试

**命令**: `cargo test`

**测试覆盖**:
- `lexer.rs`: 关键字、标识符、数字字面量、字符串、运算符、分隔符、注释、位置信息
- `parser.rs`: 函数定义、任务定义、类型解析、语句解析、表达式解析、错误处理
- `codegen.rs`: CUDA 代码生成、Buffer 运行时、任务内核生成、函数生成
- `typeck.rs`: 类型推断、类型兼容性、作用域管理、错误报告
- `main.rs`: 完整编译流水线测试、错误恢复测试、边界情况测试

### 1.2 HSCIR 中间表示完善

#### 操作系统扩展
- [ ] 添加算术操作 (`AddOp`, `SubOp`, `MulOp`, `DivOp`)
- [ ] 添加内存操作 (`LoadOp`, `StoreOp`, `AllocOp`)
- [ ] 添加控制流操作 (`BranchOp`, `CondBranchOp`, `ReturnOp`)
- [ ] 添加并行操作 (`ParallelForOp`, `ReduceOp`)
- [ ] 添加设备操作 (`SpawnOp`, `SyncOp`, `MoveOp`)

**文件**: `HSCIR/include/hscir/Operations.h`

#### IR 验证器
- [ ] 实现类型验证
- [ ] 实现操作数/结果验证
- [ ] 实现控制流完整性验证
- [ ] 实现支配关系验证

**文件**: `HSCIR/include/hscir/Verifier.h` (新建)

#### IR 打印与解析
- [ ] 实现文本格式打印 (`toString()`)
- [ ] 实现文本格式解析
- [ ] 实现二进制序列化
- [ ] 添加 IR 可视化工具 (生成 DOT/Graphviz)

**文件**: `HSCIR/src/Printer.cpp`, `HSCIR/src/Parser.cpp` (新建)

---

## 二、高优先级 (P1)

### 2.1 编译器后端优化

#### CUDA 后端改进
- [ ] 优化 kernel 启动配置 (自动选择 block/grid 大小)
- [ ] 支持共享内存使用
- [ ] 支持流式执行 (CUDA Stream)
- [ ] 生成 cuBLAS/cuRAND 库调用
- [ ] 支持 FP16/BF16 低精度计算

**文件**: `HSCC/hscc/src/codegen/cuda.rs` (重构)

#### 新后端支持
- [ ] 添加 HIP 后端 (AMD GPU 支持)
- [ ] 添加 CPU 后端 (LLVM IR 生成)
- [ ] 设计后端抽象接口 (`Backend` trait)

**文件**: `HSCC/hscc/src/backend/` (新建目录)

### 2.2 HSCMake 构建系统完善

#### 多语言支持
- [ ] 完善 C++ 目标构建规则
- [ ] 完善 Rust 目标构建规则
- [ ] 添加 CUDA 目标支持
- [ ] 添加混合语言目标支持

**文件**: `HSCMake/hscmake/rules.py`

#### 依赖管理
- [ ] 实现依赖图构建
- [ ] 实现增量构建
- [ ] 实现并行构建
- [ ] 添加外部依赖支持 (如 LLVM, CUDA)

**文件**: `HSCMake/hscmake/dependency.py` (新建)

#### 测试覆盖
- [ ] 添加解析器测试
- [ ] 添加构建规则测试
- [ ] 添加端到端构建测试

**命令**: `uv run pytest`

### 2.3 示例项目完善

#### CFD_AI_SIM 示例
- [ ] 实现完整的示例代码
- [ ] 添加示例数据文件
- [ ] 添加运行脚本
- [ ] 添加性能基准测试
- [ ] 编写示例文档

**目录**: `HSCLang/examples/CFD_AI_SIM/`

---

## 三、中优先级 (P2)

### 3.1 MLIR 集成 (详见 `docs/TODO.MLIR.md`)

#### 阶段一：原型验证 (1-2 月)
- [ ] 学习 MLIR 方言定义框架
- [ ] 定义 `hsc` 高层方言
  - `hsc.task` 操作
  - `hsc.parallel_for` 操作
  - `hsc.buffer` 类型
- [ ] 实现 AST -> MLIR 转换
- [ ] 实现 MLIR -> GPU 方言降低
- [ ] 验证 GPU 后端代码生成

#### 阶段二：扩展与优化 (3-6 月)
- [ ] 完善高层方言 (pattern, policy, spawn 等)
- [ ] 实现 pattern/policy 到 MLIR 属性映射
- [ ] 集成 FPGA 后端 (HLS 方言)
- [ ] 复用 MLIR 现有优化 Pass

### 3.2 FPGA 支持 (详见 `docs/TODO.FPGA.md`)

#### HLS 后端实现
- [ ] 设计 HLS C++ 代码生成器
- [ ] 生成 Vitis HLS pragma
  - `#pragma HLS PIPELINE`
  - `#pragma HLS UNROLL`
  - `#pragma HLS INTERFACE`
- [ ] 生成 TCL 构建脚本
- [ ] 集成 Vitis HLS 工具链

**文件**: `HSCC/hscc/src/backend/hls.rs` (新建)

#### 运行时支持
- [ ] 生成 XRT 主机代码
- [ ] 实现设备内存管理
- [ ] 实现 kernel 启动接口

### 3.3 GPU 优化 (详见 `docs/TODO.GPU.md`)

#### 内核优化
- [ ] 引入高性能计算库 (cuBLAS, rocBLAS)
- [ ] 支持低精度计算 (FP16, BF16)
- [ ] 实现算子融合
- [ ] 集成 Triton DSL

#### 执行优化
- [ ] 实现 CUDA Graph 捕获
- [ ] 实现异步任务调度
- [ ] 实现流水线并行

---

## 四、低优先级 (P3)

### 4.1 HSCIDE 开发

#### IDE 核心功能
- [ ] 实现语法高亮
- [ ] 实现代码补全
- [ ] 实现错误诊断
- [ ] 实现跳转定义
- [ ] 实现悬停文档

#### LSP 服务器
- [ ] 定义 LSP 协议实现
- [ ] 集成编译器前端
- [ ] 发布 VSCode 扩展

**目录**: `HSCIDE/ide/`

### 4.2 文档与社区

#### 用户文档
- [ ] 编写快速入门指南
- [ ] 编写语言参考手册
- [ ] 编写 API 文档
- [ ] 编写最佳实践指南

#### 开发者文档
- [ ] 编写架构设计文档
- [ ] 编写贡献指南
- [ ] 编写测试指南

---

## 五、技术债务

### 5.1 代码质量
- [ ] 添加 CI/CD 管道代码覆盖率检查
- [ ] 添加静态分析 (Clippy for Rust, clang-tidy for C++)
- [ ] 统一代码风格 (rustfmt, clang-format)
- [ ] 添加 pre-commit hooks

### 5.2 架构改进
- [ ] 重构 codegen.rs (当前过于庞大)
- [ ] 分离前端与后端
- [ ] 添加编译器选项管理
- [ ] 改进错误处理 (使用 `thiserror` 替代 `anyhow` 在库代码中)

### 5.3 兼容性
- [ ] 支持 Linux 平台
- [ ] 支持 macOS 平台 (Apple Silicon)
- [ ] 测试不同 CUDA 版本兼容性

---

## 六、里程碑规划

### Milestone 1: 最小可用编译器 (MVP)
**目标**: 能够编译并运行简单的 HSCLang 程序
**时间**: 2-3 周

- [ ] 完成类型检查器
- [ ] 完成 HSCIR 基础操作
- [ ] 实现 AST -> HSCIR 转换
- [ ] 添加基础测试

### Milestone 2: 多后端支持
**目标**: 支持 GPU 和 CPU 后端
**时间**: 1-2 月

- [ ] 重构 CUDA 后端
- [ ] 添加 CPU 后端
- [ ] 完善示例项目
- [ ] 性能基准测试

### Milestone 3: MLIR 集成
**目标**: 基于 MLIR 重构编译器基础设施
**时间**: 3-6 月

- [ ] 定义 hsc 方言
- [ ] 实现渐进式降低
- [ ] 支持 GPU/FPGA/NPU 后端

### Milestone 4: 生产就绪
**目标**: 可用于实际项目
**时间**: 6-12 月

- [ ] 完善所有后端
- [ ] 完善文档
- [ ] IDE 支持
- [ ] 社区建设

---

## 七、资源需求

### 开发环境
- CUDA Toolkit 12.0+
- CMake 3.20+
- Rust 1.75+ (Edition 2024)
- Python 3.13+
- C++23 兼容编译器 (MSVC 2022, GCC 13, Clang 16)

### 可选依赖
- Vitis HLS 2023.1+ (FPGA 开发)
- ROCm 6.0+ (AMD GPU)
- MLIR/LLVM 18+

---

## 八、风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| MLIR 学习曲线陡峭 | 高 | 先完成简单后端，积累经验后再集成 |
| 多后端维护复杂 | 中 | 设计良好的后端抽象接口 |
| FPGA 工具链依赖 | 中 | 优先 GPU 后端，FPGA 作为可选功能 |
| 性能优化工作量大 | 中 | 优先正确性，性能渐进优化 |

---

## 九、参考资源

- [MLIR 文档](https://mlir.llvm.org/)
- [Vitis HLS 用户指南](https://docs.xilinx.com/r/en-US/ug1399-vitis-hls)
- [CUDA 编程指南](https://docs.nvidia.com/cuda/cuda-c-programming-guide/)
- [vLLM 项目](https://github.com/vllm-project/vllm)

---

*最后更新: 2026-03-04*
