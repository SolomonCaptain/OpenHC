# NPU 后端设备无关设计方案

> 参考 Triton DSL 实现 GPU 设备无关的设计，为 NPU（神经网络处理器）后端构建统一的抽象层。

---

## 一、背景与动机

### 1.1 Triton DSL 的设备无关设计回顾

当前项目中，Triton DSL 通过以下机制实现 GPU 设备无关：

| 层次 | 抽象机制 | 具体实现 |
|------|---------|---------|
| 类型层 | `TritonType` 统一类型表示 | 自动映射到 NVIDIA/AMD GPU 原生类型 |
| 后端层 | `Backend` 枚举 + 策略模式 | `TritonCodeGenerator`、`ROCmBackend`、`TileIRBackend` |
| 硬件层 | `HardwareSpec` + `AutoTuner` | 针对 A100/H100/MI250X 自动优化 |
| IR 层 | HSCIR 中间表示 | 渐进式降低到不同目标 |

### 1.2 NPU 与 GPU 的关键差异

| 特性 | GPU | NPU |
|------|-----|-----|
| 编程模型 | SIMT（单指令多线程） | 图执行 / 张量流式 |
| 并行模式 | 线程块、Warp、线程 | 矩阵单元、向量单元、计算核心 |
| 内存层次 | Global → Shared → Registers | HBM → 片上 SRAM → Local Buffer |
| 计算单元 | CUDA Core、Tensor Core | MXU（矩阵乘单元）、Vector Unit |
| 调度方式 | 程序员控制 Block/Grid | 编译器/运行时自动调度 |
| 典型厂商 | NVIDIA、AMD | Google TPU、华为昇腾、寒武纪、地平线 |

### 1.3 设计目标

1. **统一抽象**：通过 IR 和类型系统屏蔽 NPU 硬件差异
2. **算子映射**：将 HSCLang 并行模式映射到 NPU 原生算子
3. **自动调优**：根据 NPU 架构特性选择最优执行策略
4. **渐进降低**：HSCIR → NPU IR → 厂商工具链

---

## 二、架构设计

### 2.1 编译流程

```
HSCLang 源文件 (.hl)
       │
       ▼
  前端解析器 (lexer.rs, parser.rs)
       │
       ▼
     AST
       │
       ▼
  类型检查器 (typeck.rs)
       │
       ├──────────────┬──────────────┬──────────────┐
       ▼              ▼              ▼              ▼
   GPU 路径      Triton 路径      NPU 路径      FPGA 路径
  (codegen.rs)  (triton/)       (npu/)        (待实现)
       │              │              │
       ▼              ▼              ▼
  CUDA C++      Triton Python   NPU Graph
   (.cu)          (.py)         (.json/.bin)
       │              │              │
       ▼              ▼              ▼
   NVCC         triton runtime   厂商运行时
```

### 2.2 NPU 后端模块结构

```
HSCC/hscc/src/npu/
├── mod.rs           # 模块入口，导出公共接口
├── types.rs         # NPU 类型系统
├── kernel.rs        # NPU 内核/图表示
├── graph.rs         # 计算图构建
├── lowering.rs      # AST → NPU IR 转换
├── codegen.rs       # NPU 代码生成
├── memory.rs        # 内存规划与布局
├── tiling.rs        # 张量分块策略
├── autotuner.rs     # NPU 自动调优
├── backends/
│   ├── mod.rs       # 后端抽象 trait
│   ├── tpu.rs       # Google TPU 后端
│   ├── ascend.rs    # 华为昇腾后端
│   ├── cambrian.rs  # 寒武纪后端
│   └── generic.rs   # 通用 NPU 后端
└── runtime.rs       # 运行时接口生成
```

---

## 三、核心抽象设计

### 3.1 NPU 类型系统

参考 Triton 的 `TritonType`，设计统一的 NPU 类型系统：

```rust
// HSCC/hscc/src/npu/types.rs

/// NPU 类型种类
#[derive(Debug, Clone, PartialEq)]
pub enum NpuTypeKind {
    /// 整数类型
    Integer { width: u32, signed: bool },
    /// 浮点类型
    Float { width: u32 },
    /// 量化类型（NPU 特有）
    Quantized { 
        base: QuantBase,
        scale: f32,
        zero_point: i32,
    },
    /// 张量类型
    Tensor { 
        element: Box<NpuType>,
        shape: Vec<i64>,
        layout: TensorLayout,
    },
    /// 张量切片（NPU 优化关键）
    TensorSlice {
        source: Box<NpuType>,
        offsets: Vec<i64>,
        sizes: Vec<i64>,
    },
}

/// 量化基础类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuantBase {
    Int8,
    UInt8,
    Int4,   // 低精度推理
    UInt4,
    FP8,    // FP8 训练/推理
    BF16,   // Brain Float
}

/// 张量布局（影响 NPU 性能关键）
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TensorLayout {
    /// 行主序 (C-style)
    RowMajor,
    /// 列主序 (Fortran-style)
    ColMajor,
    /// NCHW (Batch, Channel, Height, Width)
    NCHW,
    /// NHWC (NPU 通常更友好)
    NHWC,
    /// NCxHxW (块化布局，昇腾专用)
    NCHWc { c_block: u32 },
    /// 压缩稀疏格式
    Compressed { format: SparseFormat },
}

/// NPU 类型
#[derive(Debug, Clone)]
pub struct NpuType {
    kind: NpuTypeKind,
    /// 设备提示（可选）
    device_hint: Option<NpuDevice>,
}

impl NpuType {
    // 工厂方法
    pub fn i32() -> Self { /* ... */ }
    pub fn f32() -> Self { /* ... */ }
    pub fn int8() -> Self { /* ... */ }
    pub fn fp16() -> Self { /* ... */ }
    pub fn bf16() -> Self { /* ... */ }
    pub fn quant_int8(scale: f32, zp: i32) -> Self { /* ... */ }
    
    // 张量构造
    pub fn tensor(element: NpuType, shape: Vec<i64>, layout: TensorLayout) -> Self { /* ... */ }
    pub fn tensor_nchw(element: NpuType, n: i64, c: i64, h: i64, w: i64) -> Self { /* ... */ }
    pub fn tensor_nhwc(element: NpuType, n: i64, c: i64, h: i64, w: i64) -> Self { /* ... */ }
    
    // 类型属性查询
    pub fn size_in_bytes(&self) -> usize { /* ... */ }
    pub fn is_quantized(&self) -> bool { /* ... */ }
    pub fn layout(&self) -> Option<TensorLayout> { /* ... */ }
}
```

### 3.2 NPU 设备抽象

参考 Triton 的后端枚举和 `HardwareSpec`：

```rust
// HSCC/hscc/src/npu/backends/mod.rs

/// NPU 设备类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NpuDevice {
    /// Google TPU
    TPU(TpuGeneration),
    /// 华为昇腾
    Ascend(AscendSoc),
    /// 寒武纪
    Cambrian(CambrianGeneration),
    /// 地平线 BPU
    Horizon(HorizonGeneration),
    /// 通用 NPU（通过 ONNX/MLIR 支持更多）
    Generic,
}

/// TPU 代次
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TpuGeneration {
    V2,    // Cloud TPU v2
    V3,    // Cloud TPU v3
    V4,    // Cloud TPU v4
    V5,    // Cloud TPU v5
    Edge,  // Edge TPU
}

/// 昇腾 SoC
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AscendSoc {
    Ascend310,   // 推理
    Ascend310P,  // 推理增强
    Ascend910,   // 训练
    Ascend910B,  // 训练增强
}

/// NPU 硬件规格
#[derive(Debug, Clone)]
pub struct NpuHardwareSpec {
    /// 设备类型
    pub device: NpuDevice,
    /// 计算核心数量
    pub num_cores: u32,
    /// 矩阵单元规格
    pub matrix_unit: MatrixUnitSpec,
    /// 向量单元规格
    pub vector_unit: VectorUnitSpec,
    /// 片上 SRAM 大小 (KB)
    pub local_memory_kb: u32,
    /// HBM 大小 (GB)
    pub hbm_size_gb: u32,
    /// HBM 带宽 (GB/s)
    pub memory_bandwidth: f64,
    /// 支持的数据类型
    pub supported_dtypes: Vec<NpuTypeKind>,
    /// 量化支持
    pub quant_support: QuantSupport,
    /// 稀疏计算支持
    pub sparse_support: bool,
}

/// 矩阵单元规格
#[derive(Debug, Clone)]
pub struct MatrixUnitSpec {
    /// 单次矩阵乘形状 (M, N, K)
    pub systolic_array: (u32, u32, u32),
    /// 支持的数据类型组合
    pub supported_combinations: Vec<(NpuTypeKind, NpuTypeKind, NpuTypeKind)>,
    /// 峰值算力 (TOPS)
    pub peak_tops: f64,
}

/// 向量单元规格
#[derive(Debug, Clone)]
pub struct VectorUnitSpec {
    /// 向量宽度
    pub width: u32,
    /// 支持的操作
    pub supported_ops: Vec<VectorOp>,
}

/// 量化支持级别
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuantSupport {
    None,
    Int8,
    Int4Int8,
    FullDynamic,
}

/// 向量操作
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VectorOp {
    Add, Sub, Mul, Div,
    Exp, Log, Sqrt,
    ReLU, Sigmoid, Tanh,
    Max, Min,
}

impl NpuHardwareSpec {
    // 预定义硬件配置
    pub fn tpu_v4() -> Self { /* ... */ }
    pub fn ascend_910b() -> Self { /* ... */ }
    pub fn ascend_310p() -> Self { /* ... */ }
    pub fn cambrian_ml370() -> Self { /* ... */ }
}
```

### 3.3 NPU 计算图表示

NPU 执行模型通常是**图执行**而非内核执行：

```rust
// HSCC/hscc/src/npu/graph.rs

/// NPU 计算图
#[derive(Debug, Default)]
pub struct NpuGraph {
    /// 图名称
    pub name: String,
    /// 输入节点
    pub inputs: Vec<NpuValue>,
    /// 输出节点
    pub outputs: Vec<NpuValue>,
    /// 操作节点
    pub operations: Vec<NpuOperation>,
    /// 中间张量
    pub tensors: Vec<NpuTensor>,
    /// 内存规划
    pub memory_plan: Option<MemoryPlan>,
    /// 执行策略
    pub execution_policy: ExecutionPolicy,
}

/// NPU 操作
#[derive(Debug, Clone)]
pub struct NpuOperation {
    /// 操作类型
    pub op_type: NpuOpType,
    /// 操作名称
    pub name: String,
    /// 输入张量
    pub inputs: Vec<NpuValue>,
    /// 输出张量
    pub outputs: Vec<NpuValue>,
    /// 属性
    pub attributes: HashMap<String, NpuAttribute>,
    /// 执行提示
    pub hints: OpHints,
}

/// NPU 操作类型
#[derive(Debug, Clone, PartialEq)]
pub enum NpuOpType {
    // ─── 矩阵运算 ───
    /// 矩阵乘 (GEMM)
    MatMul,
    /// 批量矩阵乘
    BatchMatMul,
    /// 矩阵向量乘
    MatVec,
    
    // ─── 卷积运算 ───
    /// 2D 卷积
    Conv2D { 
        padding: Padding,
        stride: (u32, u32),
        dilation: (u32, u32),
        groups: u32,
    },
    /// 深度可分离卷积
    DepthwiseConv2D,
    /// 转置卷积
    ConvTranspose2D,
    
    // ─── 激活函数 ───
    ReLU,
    ReLU6,
    Sigmoid,
    Tanh,
    GELU,
    Swish,
    Softmax { axis: i32 },
    
    // ─── 归一化 ───
    BatchNorm,
    LayerNorm,
    InstanceNorm,
    GroupNorm,
    
    // ─── 池化 ───
    MaxPool2D,
    AvgPool2D,
    GlobalAvgPool,
    AdaptivePool,
    
    // ─── 逐元素运算 ───
    Add, Sub, Mul, Div,
    Exp, Log, Sqrt, Pow,
    Min, Max, Clip,
    
    // ─── 归约运算 ───
    ReduceSum { axes: Vec<i32>, keep_dims: bool },
    ReduceMean { axes: Vec<i32>, keep_dims: bool },
    ReduceMax { axes: Vec<i32>, keep_dims: bool },
    ReduceMin { axes: Vec<i32>, keep_dims: bool },
    
    // ─── 张量变换 ───
    Reshape,
    Transpose { perm: Vec<i32> },
    Slice { starts: Vec<i64>, ends: Vec<i64> },
    Concat { axis: i32 },
    Split { axis: i32 },
    Expand,
    Squeeze,
    
    // ─── 注意力机制 ───
    /// Flash Attention（NPU 优化关键）
    FlashAttention {
        scale: f32,
        causal: bool,
    },
    MultiHeadAttention {
        num_heads: u32,
    },
    
    // ─── 量化相关 ───
    Quantize,
    Dequantize,
    Requantize,
    
    // ─── 控制流 ───
    If,
    Loop,
    
    // ─── 自定义算子 ───
    Custom { op_name: String },
}

/// 操作提示（影响性能关键）
#[derive(Debug, Clone, Default)]
pub struct OpHints {
    /// 是否融合到上游算子
    pub fuse_with_upstream: bool,
    /// 是否融合到下游算子
    pub fuse_with_downstream: bool,
    /// 首选内存布局
    pub preferred_layout: Option<TensorLayout>,
    /// 流水线深度
    pub pipeline_depth: Option<u32>,
    /// 双缓冲策略
    pub double_buffer: bool,
}

/// 执行策略
#[derive(Debug, Clone)]
pub struct ExecutionPolicy {
    /// 流水线模式
    pub pipeline_mode: PipelineMode,
    /// 内存复用策略
    pub memory_reuse: MemoryReuseStrategy,
    /// 并行策略
    pub parallel_strategy: ParallelStrategy,
}

/// 流水线模式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PipelineMode {
    /// 单流执行
    SingleStream,
    /// 多流并行
    MultiStream { num_streams: u32 },
    /// 流水线并行
    Pipelined { stages: u32 },
}
```

### 3.4 NPU 后端 Trait

参考 Triton 的后端抽象模式：

```rust
// HSCC/hscc/src/npu/backends/mod.rs

/// NPU 后端抽象 trait
pub trait NpuBackend: Send + Sync {
    /// 后端名称
    fn name(&self) -> &str;
    
    /// 支持的设备
    fn supported_devices(&self) -> Vec<NpuDevice>;
    
    /// 获取硬件规格
    fn hardware_spec(&self, device: NpuDevice) -> NpuHardwareSpec;
    
    /// 检查操作是否支持
    fn is_op_supported(&self, op: &NpuOpType, spec: &NpuHardwareSpec) -> bool;
    
    /// 获取操作的性能估计
    fn estimate_op_latency(
        &self, 
        op: &NpuOperation, 
        spec: &NpuHardwareSpec
    ) -> Duration;
    
    /// 优化计算图
    fn optimize_graph(
        &self, 
        graph: &mut NpuGraph, 
        spec: &NpuHardwareSpec
    ) -> Result<(), NpuError>;
    
    /// 内存规划
    fn plan_memory(
        &self, 
        graph: &mut NpuGraph, 
        spec: &NpuHardwareSpec
    ) -> Result<MemoryPlan, NpuError>;
    
    /// 生成设备代码
    fn generate_code(
        &self, 
        graph: &NpuGraph, 
        spec: &NpuHardwareSpec
    ) -> Result<NpuCode, NpuError>;
    
    /// 生成运行时配置
    fn generate_runtime_config(
        &self,
        graph: &NpuGraph,
        spec: &NpuHardwareSpec
    ) -> Result<RuntimeConfig, NpuError>;
}

/// 生成的 NPU 代码
#[derive(Debug)]
pub enum NpuCode {
    /// ONNX 模型
    OnnxModel(Vec<u8>),
    /// TensorFlow Lite 模型
    TFLiteModel(Vec<u8>),
    /// 厂商格式模型
    VendorModel { 
        format: String,
        data: Vec<u8>,
    },
    /// C++ 运行时代码
    CppCode {
        header: String,
        source: String,
    },
    /// JSON 图定义
    JsonGraph(String),
}

/// 运行时配置
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// 输入张量描述
    pub inputs: Vec<TensorDesc>,
    /// 输出张量描述
    pub outputs: Vec<TensorDesc>,
    /// 执行配置
    pub execution: ExecutionConfig,
    /// 内存池配置
    pub memory_pool: MemoryPoolConfig,
}
```

---

## 四、HSCLang 到 NPU 的映射

### 4.1 并行模式映射

HSCLang 的 `pattern` 和 `parallel for` 映射到 NPU 执行：

```rust
// HSCC/hscc/src/npu/lowering.rs

/// AST 到 NPU IR 转换
pub struct NpuLowering {
    graph: NpuGraph,
    backend: Box<dyn NpuBackend>,
    spec: NpuHardwareSpec,
}

impl NpuLowering {
    /// 转换并行循环
    pub fn lower_parallel_for(&mut self, pf: &ParallelFor) -> Result<NpuValue, NpuError> {
        match pf.pattern {
            // 独立循环 → 向量化 / 张量化
            Pattern::For { independent: true, .. } => {
                self.lower_independent_for(pf)
            }
            
            // 归约 → NPU 归约算子
            Pattern::Reduce { kind, .. } => {
                self.lower_reduce_for(pf, kind)
            }
            
            // 扫描 → 特殊处理
            Pattern::Scan { .. } => {
                self.lower_scan_for(pf)
            }
            
            // 任务图 → 子图
            Pattern::TaskGraph { .. } => {
                self.lower_task_graph(pf)
            }
        }
    }
    
    /// 独立循环 → NPU 向量/张量操作
    fn lower_independent_for(&mut self, pf: &ParallelFor) -> Result<NpuValue, NpuError> {
        // 识别循环模式
        if let Some(matmul_pattern) = self.try_match_matmul(pf) {
            // 生成 MatMul 算子
            return self.emit_matmul(&matmul_pattern);
        }
        
        if let Some(conv_pattern) = self.try_match_conv(pf) {
            // 生成 Conv2D 算子
            return self.emit_conv2d(&conv_pattern);
        }
        
        if let Some(elementwise) = self.try_match_elementwise(pf) {
            // 生成逐元素操作
            return self.emit_elementwise(&elementwise);
        }
        
        // 无法识别的模式，生成通用循环
        self.emit_generic_loop(pf)
    }
    
    /// 归约循环 → NPU 归约算子
    fn lower_reduce_for(
        &mut self, 
        pf: &ParallelFor, 
        kind: ReduceKind
    ) -> Result<NpuValue, NpuError> {
        let axes = self.infer_reduce_axes(pf);
        
        let op_type = match kind {
            ReduceKind::Sum => NpuOpType::ReduceSum { axes, keep_dims: false },
            ReduceKind::Prod => NpuOpType::ReduceProd { axes, keep_dims: false },
            ReduceKind::Min => NpuOpType::ReduceMin { axes, keep_dims: false },
            ReduceKind::Max => NpuOpType::ReduceMax { axes, keep_dims: false },
        };
        
        self.emit_reduce_op(op_type, pf)
    }
}
```

### 4.2 任务映射

HSCLang 的 `task` 映射到 NPU 计算图：

```hl
// HSCLang 源代码
task matmul_relu {
    body(A: Buffer<f32>, B: Buffer<f32>) -> Buffer<f32> {
        let C = parallel for (i, j) in 0..M, 0..N reduce sum {
            for k in 0..K {
                sum += A[i, k] * B[k, j];
            }
        }
        
        parallel for (i, j) in 0..M, 0..N {
            C[i, j] = relu(C[i, j]);
        }
    }
}
```

```rust
// 生成的 NPU 计算图
NpuGraph {
    inputs: [
        NpuValue::Tensor("A", shape=[M, K], layout=NCHW),
        NpuValue::Tensor("B", shape=[K, N], layout=NCHW),
    ],
    operations: [
        // MatMul 算子
        NpuOperation {
            op_type: NpuOpType::MatMul,
            inputs: ["A", "B"],
            outputs: ["C_temp"],
        },
        // ReLU 算子（自动融合）
        NpuOperation {
            op_type: NpuOpType::ReLU,
            inputs: ["C_temp"],
            outputs: ["C"],
            hints: OpHints { fuse_with_upstream: true },
        },
    ],
    outputs: [
        NpuValue::Tensor("C", shape=[M, N], layout=NCHW),
    ],
}
```

### 4.3 内存布局转换

```rust
// HSCC/hscc/src/npu/memory.rs

/// 内存布局规划器
pub struct MemoryPlanner {
    spec: NpuHardwareSpec,
}

impl MemoryPlanner {
    /// 规划张量布局
    pub fn plan_layout(&self, tensor: &NpuTensor, usage: &TensorUsage) -> TensorLayout {
        match self.spec.device {
            // TPU: NHWC 更高效
            NpuDevice::TPU(_) => TensorLayout::NHWC,
            
            // 昇腾: NCxHxW 块化布局
            NpuDevice::Ascend(AscendSoc::Ascend910 | AscendSoc::Ascend910B) => {
                TensorLayout::NCHWc { c_block: 16 }
            }
            NpuDevice::Ascend(AscendSoc::Ascend310 | AscendSoc::Ascend310P) => {
                TensorLayout::NCHWc { c_block: 8 }
            }
            
            // 寒武纪: NHWC
            NpuDevice::Cambrian(_) => TensorLayout::NHWC,
            
            // 默认: NCHW
            _ => TensorLayout::NCHW,
        }
    }
    
    /// 规划内存复用
    pub fn plan_memory_reuse(&self, graph: &mut NpuGraph) -> MemoryPlan {
        // 计算张量生命周期
        let lifetimes = self.compute_lifetimes(graph);
        
        // 使用图着色算法分配内存
        let mut allocator = MemoryAllocator::new(self.spec.local_memory_kb * 1024);
        
        for tensor in &graph.tensors {
            let lifetime = &lifetimes[&tensor.id];
            allocator.allocate(tensor, lifetime);
        }
        
        allocator.into_plan()
    }
}
```

---

## 五、算子融合与优化

### 5.1 融合模式定义

参考 Triton 的融合优化：

```rust
// HSCC/hscc/src/npu/fusion.rs

/// NPU 算子融合模式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NpuFusionPattern {
    /// MatMul + Bias + Activation
    MatMulBiasActivation,
    /// Conv + BN + ReLU
    ConvBNReLU,
    /// Conv + Bias + Activation
    ConvBiasActivation,
    /// LayerNorm + Linear + Residual
    TransformerBlock,
    /// Softmax 融合
    SoftmaxFusion,
    /// Element-wise 链式融合
    ElementWiseChain,
    /// Flash Attention
    FlashAttention,
}

/// 融合规则
pub struct FusionRule {
    /// 融合模式
    pub pattern: NpuFusionPattern,
    /// 条件检查
    pub condition: fn(&[NpuOperation], &NpuHardwareSpec) -> bool,
    /// 融合变换
    pub transform: fn(&mut [NpuOperation]) -> NpuOperation,
    /// 性能收益估计
    pub benefit: fn(&[NpuOperation], &NpuHardwareSpec) -> f64,
}

/// 融合优化器
pub struct FusionOptimizer {
    rules: Vec<FusionRule>,
    spec: NpuHardwareSpec,
}

impl FusionOptimizer {
    /// 执行融合优化
    pub fn optimize(&self, graph: &mut NpuGraph) {
        let mut changed = true;
        
        while changed {
            changed = false;
            
            // 尝试所有融合规则
            for rule in &self.rules {
                if let Some(ops) = self.find_fusion_pattern(graph, rule) {
                    if (rule.condition)(&ops, &self.spec) {
                        self.apply_fusion(graph, ops, rule);
                        changed = true;
                    }
                }
            }
        }
    }
}
```

### 5.2 自动调优

参考 Triton 的自动调优框架：

```rust
// HSCC/hscc/src/npu/autotuner.rs

/// NPU 自动调优器
pub struct NpuAutoTuner {
    spec: NpuHardwareSpec,
    tuning_history: HashMap<String, TuningResult>,
}

/// 调优参数
#[derive(Debug, Clone)]
pub struct NpuTuningParams {
    /// 分块大小
    pub tile_sizes: Vec<Vec<i64>>,
    /// 流水线深度
    pub pipeline_depth: u32,
    /// 双缓冲
    pub double_buffer: bool,
    /// 内存布局
    pub layout: TensorLayout,
    /// 量化策略
    pub quant_strategy: Option<QuantStrategy>,
}

/// 调优空间
impl NpuAutoTuner {
    /// 获取调优空间
    pub fn get_tuning_space(&self, op: &NpuOpType) -> Vec<NpuTuningParams> {
        match op {
            NpuOpType::MatMul => self.matmul_tuning_space(),
            NpuOpType::Conv2D { .. } => self.conv_tuning_space(),
            NpuOpType::FlashAttention { .. } => self.attention_tuning_space(),
            _ => vec![NpuTuningParams::default()],
        }
    }
    
    /// MatMul 调优空间
    fn matmul_tuning_space(&self) -> Vec<NpuTuningParams> {
        let systolic = self.spec.matrix_unit.systolic_array;
        
        // 基于硬件规格生成候选配置
        vec![
            // 分块匹配 systolic array
            NpuTuningParams {
                tile_sizes: vec![
                    vec![systolic.0 as i64, systolic.2 as i64],  // A 分块
                    vec![systolic.2 as i64, systolic.1 as i64],  // B 分块
                ],
                pipeline_depth: 2,
                double_buffer: true,
                layout: self.spec.preferred_layout(),
                quant_strategy: None,
            },
            // 其他候选配置...
        ]
    }
    
    /// 运行调优
    pub fn tune(&mut self, graph: &NpuGraph) -> NpuTuningParams {
        // 基于图特征选择调优策略
        let key = self.graph_signature(graph);
        
        if let Some(result) = self.tuning_history.get(&key) {
            return result.params.clone();
        }
        
        // 执行调优搜索
        let best = self.search_best_config(graph);
        self.tuning_history.insert(key, best.clone());
        
        best.params
    }
}
```

---

## 六、厂商后端实现示例

### 6.1 华为昇腾后端

```rust
// HSCC/hscc/src/npu/backends/ascend.rs

/// 华为昇腾后端
pub struct AscendBackend;

impl NpuBackend for AscendBackend {
    fn name(&self) -> &str { "ascend" }
    
    fn supported_devices(&self) -> Vec<NpuDevice> {
        vec![
            NpuDevice::Ascend(AscendSoc::Ascend310),
            NpuDevice::Ascend(AscendSoc::Ascend310P),
            NpuDevice::Ascend(AscendSoc::Ascend910),
            NpuDevice::Ascend(AscendSoc::Ascend910B),
        ]
    }
    
    fn optimize_graph(&self, graph: &mut NpuGraph, spec: &NpuHardwareSpec) -> Result<(), NpuError> {
        // 昇腾特定优化
        
        // 1. 布局转换: NCHW → NCxHxW
        self.transform_layout(graph, spec);
        
        // 2. 算子融合
        let fusion = FusionOptimizer::new_for_ascend(spec);
        fusion.optimize(graph);
        
        // 3. 量化优化
        if spec.quant_support != QuantSupport::None {
            self.apply_quantization(graph, spec)?;
        }
        
        // 4. 内存规划
        let planner = AscendMemoryPlanner::new(spec);
        graph.memory_plan = Some(planner.plan(graph)?);
        
        Ok(())
    }
    
    fn generate_code(&self, graph: &NpuGraph, spec: &NpuHardwareSpec) -> Result<NpuCode, NpuError> {
        // 生成 OM (离线模型) 格式
        let om_model = self.generate_om(graph, spec)?;
        Ok(NpuCode::VendorModel {
            format: "om".to_string(),
            data: om_model,
        })
    }
}

impl AscendBackend {
    /// NCxHxW 布局转换
    fn transform_layout(&self, graph: &mut NpuGraph, spec: &NpuHardwareSpec) {
        let c_block = match spec.device {
            NpuDevice::Ascend(AscendSoc::Ascend910 | AscendSoc::Ascend910B) => 16,
            NpuDevice::Ascend(AscendSoc::Ascend310 | AscendSoc::Ascend310P) => 8,
            _ => return,
        };
        
        for tensor in &mut graph.tensors {
            tensor.layout = TensorLayout::NCHWc { c_block };
        }
        
        // 插入必要的 Transpose 操作
        self.insert_transposes(graph);
    }
    
    /// 生成 OM 模型
    fn generate_om(&self, graph: &NpuGraph, spec: &NpuHardwareSpec) -> Result<Vec<u8>, NpuError> {
        // 转换为昇腾 IR
        let ascend_ir = self.to_ascend_ir(graph)?;
        
        // 调用 ATC 工具编译
        // 或者直接生成 JSON 配置，由用户调用 atc
        self.serialize_to_json(&ascend_ir)
    }
}
```

### 6.2 Google TPU 后端

```rust
// HSCC/hscc/src/npu/backends/tpu.rs

/// Google TPU 后端
pub struct TpuBackend;

impl NpuBackend for TpuBackend {
    fn name(&self) -> &str { "tpu" }
    
    fn supported_devices(&self) -> Vec<NpuDevice> {
        vec![
            NpuDevice::TPU(TpuGeneration::V4),
            NpuDevice::TPU(TpuGeneration::V5),
            NpuDevice::TPU(TpuGeneration::Edge),
        ]
    }
    
    fn optimize_graph(&self, graph: &mut NpuGraph, spec: &NpuHardwareSpec) -> Result<(), NpuError> {
        // TPU 特定优化
        
        // 1. 布局转换: NCHW → NHWC
        self.transform_to_nhwc(graph);
        
        // 2. XLA 兼容性检查
        self.ensure_xla_compatible(graph)?;
        
        // 3. 算子融合
        let fusion = FusionOptimizer::new_for_tpu(spec);
        fusion.optimize(graph);
        
        // 4. TPU 特定优化（SPMD 分片等）
        if let NpuDevice::TPU(TpuGeneration::V4 | TpuGeneration::V5) = spec.device {
            self.apply_spmd_sharding(graph, spec)?;
        }
        
        Ok(())
    }
    
    fn generate_code(&self, graph: &NpuGraph, spec: &NpuHardwareSpec) -> Result<NpuCode, NpuError> {
        // 生成 TensorFlow Lite 或 SavedModel
        let tflite = self.generate_tflite(graph, spec)?;
        Ok(NpuCode::TFLiteModel(tflite))
    }
}
```

---

## 七、运行时接口

### 7.1 统一运行时 API

```rust
// HSCC/hscc/src/npu/runtime.rs

/// NPU 运行时接口生成
pub struct RuntimeGenerator {
    backend: Box<dyn NpuBackend>,
}

/// 生成的运行时代码
pub struct RuntimeCode {
    /// C++ 头文件
    pub header: String,
    /// C++ 实现文件
    pub source: String,
    /// Python 绑定（可选）
    pub python_binding: Option<String>,
}

impl RuntimeGenerator {
    /// 生成统一运行时接口
    pub fn generate(&self, graph: &NpuGraph, spec: &NpuHardwareSpec) -> RuntimeCode {
        let header = self.generate_header(graph);
        let source = self.generate_source(graph, spec);
        let python = self.generate_python_binding(graph);
        
        RuntimeCode {
            header,
            source,
            python_binding: Some(python),
        }
    }
    
    /// 生成 C++ 头文件
    fn generate_header(&self, graph: &NpuGraph) -> String {
        let inputs = graph.inputs.iter().map(|i| {
            format!("    void* {}_ptr;  // {}", i.name, i.tensor_type)
        }).collect::<Vec<_>>().join("\n");
        
        let outputs = graph.outputs.iter().map(|o| {
            format!("    void* {}_ptr;  // {}", o.name, o.tensor_type)
        }).collect::<Vec<_>>().join("\n");
        
        format!(r#"// Auto-generated by HSCC NPU Backend
#pragma once
#include <cstdint>
#include <vector>

namespace hsc::npu::{graph_name} {{

struct Inputs {{
{inputs}
}};

struct Outputs {{
{outputs}
}};

class Executor {{
public:
    Executor();
    ~Executor();
    
    bool initialize();
    bool execute(const Inputs& inputs, Outputs& outputs);
    void finalize();
    
private:
    class Impl;
    std::unique_ptr<Impl> impl_;
}};

}}  // namespace hsc::npu::{graph_name}
"#,
            graph_name = graph.name,
            inputs = inputs,
            outputs = outputs,
        )
    }
}
```

---

## 八、配置文件扩展

### 8.1 HSCC.toml NPU 配置

```toml
[package]
name = "npu_demo"
version = "0.1.0"

[target]
device = "npu"
arch = "ascend_910b"  # ascend_310, ascend_910, tpu_v4, tpu_v5, cambrian_ml370

[backend]
kind = "npu"

[backend.npu]
# 量化配置
quantization = "dynamic"  # none, static, dynamic
precision = "bf16"        # fp32, fp16, bf16, int8

# 内存配置
memory_pool_mb = 1024
enable_memory_reuse = true

# 性能调优
auto_tune = true
tune_iterations = 10

# 昇腾特定配置
[backend.npu.ascend]
soc_version = "Ascend910B"
core_type = "AICore"  # AICore, VectorCore
optimization_level = "O3"  # O0, O1, O2, O3
```

---

## 九、实施路线图

### 阶段一：基础框架（原型）

1. **类型系统实现**
   - `NpuType` 和 `NpuTypeKind`
   - 张量布局支持
   - 量化类型支持

2. **计算图表示**
   - `NpuGraph` 基础结构
   - 核心算子类型定义
   - 图序列化/反序列化

3. **AST 到 NPU IR 转换**
   - `parallel for` 模式识别
   - MatMul/Conv 模式匹配
   - 基础归约转换

### 阶段二：后端实现

1. **通用 NPU 后端**
   - ONNX 导出支持
   - 通用内存规划
   - 基础融合优化

2. **昇腾后端**
   - NCxHxW 布局转换
   - OM 模型生成
   - 量化支持

3. **TPU 后端**
   - NHWC 布局转换
   - TFLite 模型生成
   - SPMD 分片支持

### 阶段三：优化与调优

1. **算子融合**
   - 融合模式检测
   - 后端特定融合规则
   - 性能收益建模

2. **自动调优**
   - 调优空间搜索
   - 硬件感知配置选择
   - 调优结果缓存

3. **内存优化**
   - 内存复用规划
   - 双缓冲支持
   - HBM 分配策略

### 阶段四：运行时与集成

1. **统一运行时接口**
   - C++ API 生成
   - Python 绑定
   - 跨后端抽象

2. **性能分析工具**
   - 算子级性能分析
   - 内存使用追踪
   - 调优建议生成

---

## 十、与现有架构的集成

### 10.1 编译器集成

```rust
// HSCC/hscc/src/main.rs

fn compile(project_dir: &Path, backend_override: Option<&str>) -> Result<()> {
    // ... 前端解析 ...
    
    match backend {
        Backend::Cuda => { /* ... */ }
        Backend::Triton => { /* ... */ }
        Backend::Npu => {
            // NPU 后端编译流程
            let npu_config = NpuConfig::from_toml(&config)?;
            let npu_backend = create_npu_backend(&npu_config)?;
            
            // AST → NPU Graph
            let lowering = NpuLowering::new(npu_backend.clone());
            let graph = lowering.lower(&ast)?;
            
            // 优化
            npu_backend.optimize_graph(&mut graph)?;
            
            // 内存规划
            let memory_plan = npu_backend.plan_memory(&graph)?;
            
            // 代码生成
            let npu_code = npu_backend.generate_code(&graph)?;
            let runtime = RuntimeGenerator::new(npu_backend).generate(&graph)?;
            
            // 输出
            write_output(project_dir, &npu_code, &runtime)?;
        }
    }
    
    Ok(())
}
```

### 10.2 HSCIR 扩展

```rust
// HSCIR 支持的 NPU 操作扩展

// 并行执行模式
enum class ParallelMode {
    SIMD,      // 单指令多数据
    SIMT,      // 单指令多线程 (GPU)
    MIMD,      // 多指令多数据
    Tensor,    // 张量并行 (NPU)
    Pipeline,  // 流水线并行
};

// NPU 特化操作
class TensorComputeOp : public Operation {
    TensorOpKind kind;      // MatMul, Conv, etc.
    TensorLayout layout;
    ParallelMode parallel_mode;
    // ...
};

// 设备放置扩展
class PlaceOnOp : public Operation {
    Value buffer;
    DeviceKind device;      // GPU, NPU, FPGA
    NpuDevice npu_device;   // 如果是 NPU
    MemorySpace memory;     // HBM, SRAM, etc.
};
```

---

## 十一、总结

通过借鉴 Triton DSL 的设备无关设计，NPU 后端可以通过以下抽象层次实现统一：

| 层次 | 抽象 | 实现 |
|------|------|------|
| 类型层 | `NpuType` | 统一类型表示，支持量化 |
| 设备层 | `NpuDevice` + `NpuHardwareSpec` | 硬件规格抽象 |
| 后端层 | `NpuBackend` trait | 策略模式，支持多厂商 |
| 图层 | `NpuGraph` | 计算图统一表示 |
| 算子层 | `NpuOpType` | 算子抽象与映射 |
| 内存层 | `TensorLayout` + `MemoryPlan` | 布局与内存规划 |
| 调优层 | `NpuAutoTuner` | 自动调优框架 |

**关键设计决策**：

1. **图执行模型**：NPU 采用计算图而非内核模型
2. **张量布局**：不同 NPU 有不同的最优布局
3. **算子融合**：融合是 NPU 性能的关键
4. **量化支持**：量化是 NPU 的核心优势
5. **内存规划**：片上内存有限，需要精细规划

这样，HSCLang 可以实现 "一次编写，多设备运行"，支持 GPU、NPU、FPGA 等异构设备。

---

## 十二、Intel NPU 集成方案

### 12.1 Intel NPU 硬件架构分析

Intel NPU（Neural Processing Unit）源自 2016 年收购的 Movidius，首次集成于 Meteor Lake（Intel Core Ultra）处理器中。

#### 硬件规格

| 特性 | Intel NPU 3720 (Meteor Lake) |
|------|------------------------------|
| 架构来源 | Movidius Myriad X |
| 时钟频率 | 1.16 GHz |
| 计算单元 | 2 × NCE (Neural Compute Engine) tiles |
| MAC 阵列 | 512 MPEs/tile × 2 = 1024 MPEs |
| INT8 算力 | 4096 MACs/cycle = **9.5 TOPS** |
| FP16 算力 | 2048 MACs/cycle = **4.7 TFLOPS** |
| 片上 SRAM | 2 MB/tile × 2 = **4 MB** |
| 控制器 | LEON SPARC (LeonRT + LeonNN) |
| DSP 核心 | SHAVE (Streaming Hybrid Architecture Vector Engine) |

#### 数据类型支持

| 数据类型 | 支持程度 | 备注 |
|---------|---------|------|
| INT8 | ✅ 全速 | 主要优化目标 |
| FP16 | ✅ 半速 | |
| FP32 | ⚠️ 有限 | 通过 SHAVE DSP，约 50+ GFLOPS |
| FP64 | ❌ 不支持 | 无法运行部分模型 |
| BF16 | ⚠️ 取决于代次 | Lunar Lake+ 支持 |

#### 架构特点

```
┌─────────────────────────────────────────────────────────────┐
│                    Intel NPU 3720                           │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐    ┌─────────────┐                        │
│  │   LeonRT    │    │   LeonNN    │  ← LEON SPARC 控制器   │
│  │ (命令处理)   │    │ (任务调度)   │                        │
│  └──────┬──────┘    └──────┬──────┘                        │
│         │                  │                                │
│  ┌──────▼──────────────────▼──────┐                        │
│  │           NCE Tile 0           │                        │
│  │  ┌─────────────────────────┐  │                        │
│  │  │   MAC Array (512 MPEs)  │  │  ← 矩阵乘加速          │
│  │  │   4096 INT8 MACs/cycle  │  │                        │
│  │  └─────────────────────────┘  │                        │
│  │  ┌─────────────────────────┐  │                        │
│  │  │   SHAVE DSP Cores       │  │  ← 通用向量运算        │
│  │  │   FP32, Transcendentals │  │                        │
│  │  └─────────────────────────┘  │                        │
│  │  ┌─────────────────────────┐  │                        │
│  │  │   SRAM (2 MB)           │  │  ← 软件管理内存        │
│  │  └─────────────────────────┘  │                        │
│  └───────────────────────────────┘                        │
│  ┌───────────────────────────────┐                        │
│  │           NCE Tile 1           │  ← 对称结构           │
│  └───────────────────────────────┘                        │
├─────────────────────────────────────────────────────────────┤
│                    Scalable Fabric                          │
│              (共享 LPDDR5 内存子系统)                        │
└─────────────────────────────────────────────────────────────┘
```

#### 与 GPU 的性能对比

根据实测数据（Meteor Lake iGPU vs NPU）：

| 工作负载 | NPU (INT8) | iGPU (FP32) | 备注 |
|---------|-----------|-------------|------|
| Stable Diffusion UNET | 0.85 iter/s | 1.38 iter/s | iGPU 快 62% |
| 功耗 | < 7W | ~20W | NPU 更省电 |
| 启动延迟 | 较高 | 较低 | NPU 需要编译 |
| 模型兼容性 | 有限 | 广泛 | FP64 不支持 |

**关键结论**：Intel NPU 的优势在于**功耗效率**而非绝对性能。

### 12.2 集成方案对比

#### 方案 A：OpenVINO 官方集成（推荐）

```
HSCLang 源文件
       │
       ▼
  HSCC 编译器
       │
       ▼
    NpuGraph
       │
       ▼
   ONNX 模型  ←── 统一中间格式
       │
       ▼
  OpenVINO IR
       │
       ▼
  NPU Blob (编译后模型)
       │
       ▼
  OpenVINO Runtime
```

**优点**：
- 官方支持，稳定可靠
- 自动优化和算子融合
- 模型缓存机制（UMD Caching）
- 支持 AUTO 设备选择（CPU/GPU/NPU 自动调度）

**缺点**：
- 只支持静态形状模型
- 依赖 Intel NPU Driver
- 跨版本 Blob 不兼容
- FP64 支持缺失

**适用场景**：生产环境、推理部署、功耗敏感应用

#### 方案 B：ONNX Runtime + DirectML

```
HSCLang 源文件
       │
       ▼
  HSCC 编译器
       │
       ▼
    NpuGraph
       │
       ▼
   ONNX 模型
       │
       ▼
  ONNX Runtime
       │
       ▼
  DirectML EP
       │
       ▼
  Intel NPU (通过 DirectX 12)
```

**优点**：
- 跨平台兼容性好
- 与 Windows 生态集成

**缺点**：
- NPU 不支持 DXGI，需要特殊处理
- 性能可能不如 OpenVINO
- 文档和社区支持较少

**适用场景**：Windows 平台、需要与其他 AI 框架集成

#### 方案 C：oneAPI + Level Zero

```
HSCLang 源文件
       │
       ▼
  HSCC 编译器
       │
       ▼
  MLIR (自定义方言)
       │
       ▼
  Level Zero API
       │
       ▼
  Intel NPU Driver
```

**优点**：
- 更底层的控制
- 与 oneAPI 生态集成
- 可扩展到其他 Intel 硬件

**缺点**：
- 开发复杂度高
- oneDNN 对 NPU 支持有限
- 需要处理更多硬件细节

**适用场景**：研究项目、需要底层控制的高级用户

#### 方案 D：MLIR 自定义方言

```
HSCLang 源文件
       │
       ▼
  HSCC 编译器
       │
       ▼
  HSCIR (自定义 MLIR 方言)
       │
       ▼
  NPU 方言 (自定义)
       │
       ▼
  OpenVINO IR / ONNX
       │
       ▼
  目标后端
```

**优点**：
- 最大灵活性
- 可复用 MLIR 生态
- 渐进式降低，保留优化机会

**缺点**：
- 开发工作量最大
- 需要维护自定义方言
- 与 OpenVINO 集成仍需适配

**适用场景**：长期战略、需要支持多种 NPU 厂商

### 12.3 推荐技术路线

#### 推荐方案：OpenVINO 作为主要后端

基于以下考虑，**推荐使用 OpenVINO** 作为 Intel NPU 的主要集成方案：

1. **官方支持**：Intel 官方维护，与 NPU Driver 紧密集成
2. **成熟度**：2024+ 版本已支持 Core Ultra NPU
3. **优化能力**：内置算子融合、量化、内存规划
4. **生态兼容**：支持 PyTorch、TensorFlow、ONNX 模型导入

#### 实现架构

```rust
// HSCC/hscc/src/npu/backends/intel_npu.rs

/// Intel NPU 后端 (基于 OpenVINO)
pub struct IntelNpuBackend {
    /// OpenVINO 配置
    openvino_config: OpenVinoConfig,
}

/// Intel NPU 代次
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntelNpuGeneration {
    /// Meteor Lake (Core Ultra Series 1)
    MeteorLake,
    /// Lunar Lake (Core Ultra Series 2)
    LunarLake,
    /// Arrow Lake
    ArrowLake,
}

/// Intel NPU 设备
#[derive(Debug, Clone)]
pub struct IntelNpuDevice {
    pub generation: IntelNpuGeneration,
    pub num_tiles: u32,
    pub sram_per_tile_kb: u32,
    pub peak_tops: f64,
}

impl NpuBackend for IntelNpuBackend {
    fn name(&self) -> &str { "intel_npu" }
    
    fn supported_devices(&self) -> Vec<NpuDevice> {
        vec![
            NpuDevice::IntelNPU(IntelNpuGeneration::MeteorLake),
            NpuDevice::IntelNPU(IntelNpuGeneration::LunarLake),
            NpuDevice::IntelNPU(IntelNpuGeneration::ArrowLake),
        ]
    }
    
    fn hardware_spec(&self, device: NpuDevice) -> NpuHardwareSpec {
        match device {
            NpuDevice::IntelNPU(IntelNpuGeneration::MeteorLake) => NpuHardwareSpec {
                device,
                num_cores: 2,  // 2 NCE tiles
                matrix_unit: MatrixUnitSpec {
                    systolic_array: (64, 64, 1),  // 512 MPEs = 64×8 或其他配置
                    supported_combinations: vec![
                        (NpuTypeKind::Integer { width: 8, signed: true }, 
                         NpuTypeKind::Integer { width: 8, signed: true },
                         NpuTypeKind::Integer { width: 32, signed: true }),
                    ],
                    peak_tops: 9.5,
                },
                vector_unit: VectorUnitSpec {
                    width: 128,  // SHAVE DSP 向量宽度
                    supported_ops: vec![
                        VectorOp::Add, VectorOp::Mul,
                        VectorOp::Exp, VectorOp::Log,
                        VectorOp::ReLU, VectorOp::Sigmoid,
                    ],
                },
                local_memory_kb: 4 * 1024,  // 4 MB SRAM
                hbm_size_gb: 0,  // 共享系统内存
                memory_bandwidth: 0.0,  // 取决于 LPDDR5 配置
                supported_dtypes: vec![
                    NpuTypeKind::Integer { width: 8, signed: true },
                    NpuTypeKind::Float { width: 16 },
                ],
                quant_support: QuantSupport::Int8,
                sparse_support: false,
            },
            
            NpuDevice::IntelNPU(IntelNpuGeneration::LunarLake) => {
                // Lunar Lake 有更强的 NPU (约 48 TOPS)
                let mut spec = Self::meteor_lake_spec();
                spec.matrix_unit.peak_tops = 48.0;
                spec.num_cores = 4;  // 更多 NCE tiles
                spec
            }
            
            _ => panic!("Unsupported Intel NPU device"),
        }
    }
    
    fn is_op_supported(&self, op: &NpuOpType, spec: &NpuHardwareSpec) -> bool {
        // OpenVINO NPU Plugin 支持的操作
        matches!(op,
            // 矩阵运算
            NpuOpType::MatMul |
            NpuOpType::BatchMatMul |
            
            // 卷积
            NpuOpType::Conv2D { .. } |
            NpuOpType::DepthwiseConv2D |
            NpuOpType::ConvTranspose2D |
            
            // 激活函数
            NpuOpType::ReLU |
            NpuOpType::ReLU6 |
            NpuOpType::Sigmoid |
            NpuOpType::Tanh |
            NpuOpType::GELU |
            NpuOpType::Swish |
            NpuOpType::Softmax { .. } |
            
            // 归一化
            NpuOpType::BatchNorm |
            NpuOpType::LayerNorm |
            
            // 池化
            NpuOpType::MaxPool2D |
            NpuOpType::AvgPool2D |
            NpuOpType::GlobalAvgPool |
            
            // 逐元素
            NpuOpType::Add |
            NpuType::Sub |
            NpuOpType::Mul |
            NpuOpType::Div |
            
            // 归约
            NpuOpType::ReduceSum { .. } |
            NpuOpType::ReduceMean { .. } |
            NpuOpType::ReduceMax { .. } |
            NpuOpType::ReduceMin { .. } |
            
            // 张量变换
            NpuOpType::Reshape |
            NpuOpType::Transpose { .. } |
            NpuOpType::Slice { .. } |
            NpuOpType::Concat { .. } |
            
            // 量化
            NpuOpType::Quantize |
            NpuOpType::Dequantize
        )
    }
    
    fn optimize_graph(&self, graph: &mut NpuGraph, spec: &NpuHardwareSpec) -> Result<(), NpuError> {
        // Intel NPU 特定优化
        
        // 1. 静态形状检查（OpenVINO NPU 要求）
        self.ensure_static_shapes(graph)?;
        
        // 2. 数据类型检查（避免 FP64）
        self.ensure_supported_dtypes(graph, spec)?;
        
        // 3. INT8 量化建议
        if spec.quant_support == QuantSupport::Int8 {
            self.suggest_quantization(graph)?;
        }
        
        // 4. 算子融合（利用 OpenVINO 自动融合）
        let fusion = FusionOptimizer::new_for_intel_npu(spec);
        fusion.optimize(graph);
        
        // 5. 内存布局优化
        self.optimize_layout_for_npu(graph, spec);
        
        Ok(())
    }
    
    fn generate_code(&self, graph: &NpuGraph, spec: &NpuHardwareSpec) -> Result<NpuCode, NpuError> {
        // 转换为 ONNX 作为中间格式
        let onnx_model = self.graph_to_onnx(graph)?;
        
        // 生成 OpenVINO 编译脚本
        let compile_script = self.generate_compile_script(graph, spec);
        
        // 运行时加载 ONNX 并编译
        Ok(NpuCode::OnnxModel(onnx_model))
    }
}

impl IntelNpuBackend {
    /// 确保静态形状
    fn ensure_static_shapes(&self, graph: &NpuGraph) -> Result<(), NpuError> {
        for tensor in &graph.tensors {
            if tensor.shape.iter().any(|d| *d < 0) {
                return Err(NpuError::UnsupportedDynamicShape {
                    tensor: tensor.name.clone(),
                    reason: "OpenVINO NPU Plugin only supports static shapes".to_string(),
                });
            }
        }
        Ok(())
    }
    
    /// 确保支持的数据类型
    fn ensure_supported_dtypes(&self, graph: &NpuGraph, spec: &NpuHardwareSpec) -> Result<(), NpuError> {
        for tensor in &graph.tensors {
            match &tensor.element_type {
                NpuTypeKind::Float { width: 64 } => {
                    return Err(NpuError::UnsupportedDataType {
                        dtype: "FP64".to_string(),
                        reason: "Intel NPU does not support FP64 operations".to_string(),
                    });
                }
                NpuTypeKind::Float { width: 32 } => {
                    // FP32 支持，但建议使用 FP16 或 INT8
                    // 可以通过 SHAVE DSP 执行
                }
                _ => {}
            }
        }
        Ok(())
    }
    
    /// 转换 NpuGraph 到 ONNX
    fn graph_to_onnx(&self, graph: &NpuGraph) -> Result<Vec<u8>, NpuError> {
        // 使用 tract-onnx 或手动构建 ONNX ProtoBuf
        let mut onnx_graph = onnx::GraphProto::new();
        
        // 添加输入
        for input in &graph.inputs {
            let value_info = self.tensor_to_value_info(input);
            onnx_graph.input.push(value_info);
        }
        
        // 添加输出
        for output in &graph.outputs {
            let value_info = self.tensor_to_value_info(output);
            onnx_graph.output.push(value_info);
        }
        
        // 添加节点
        for op in &graph.operations {
            let node = self.operation_to_onnx_node(op)?;
            onnx_graph.node.push(node);
        }
        
        // 序列化
        let model = onnx::ModelProto {
            graph: Some(onnx_graph),
            ..Default::default()
        };
        
        Ok(model.write_to_bytes()?)
    }
    
    /// 生成 OpenVINO 编译/运行脚本
    fn generate_compile_script(&self, graph: &NpuGraph, spec: &NpuHardwareSpec) -> String {
        format!(r#"#!/usr/bin/env python3
# Auto-generated by HSCC Intel NPU Backend
# Compile ONNX model for Intel NPU using OpenVINO

from openvino.runtime import Core, Dimension, Type

# Initialize OpenVINO
core = Core()

# Check NPU availability
devices = core.available_devices
if "NPU" not in devices:
    raise RuntimeError(f"NPU not available. Available devices: {{devices}}")

# Load ONNX model
model = core.read_model(model="{}.onnx")

# Compile for NPU
# Enable model caching for faster subsequent loads
compiled_model = core.compile_model(
    model, 
    "NPU",
    config={{
        "PERFORMANCE_HINT": "LATENCY",
        "CACHE_DIR": "./model_cache",
    }}
)

# Create inference request
infer_request = compiled_model.create_infer_request()

print("Model compiled successfully for Intel NPU")
print(f"Input ports: {{len(compiled_model.inputs)}}")
print(f"Output ports: {{len(compiled_model.outputs)}}")
"#,
            graph.name
        )
    }
}
```

### 12.4 OpenVINO 运行时集成

#### Python 运行时

```python
# hsc_runtime/intel_npu.py
"""Intel NPU Runtime via OpenVINO"""

from openvino.runtime import Core, Type, Tensor
import numpy as np
from typing import Dict, List, Any

class IntelNpuExecutor:
    """Intel NPU 执行器"""
    
    def __init__(self, model_path: str, cache_dir: str = "./cache"):
        self.core = Core()
        self.model = self.core.read_model(model_path)
        
        # 配置 NPU 编译
        self.compiled_model = self.core.compile_model(
            self.model,
            "NPU",
            config={
                "PERFORMANCE_HINT": "LATENCY",
                "CACHE_DIR": cache_dir,
                # Intel NPU 特定配置
                "NPU_TURBO": "YES",  # 最大频率
            }
        )
        
        self.infer_request = self.compiled_model.create_infer_request()
    
    def get_input_shapes(self) -> Dict[str, List[int]]:
        """获取输入形状"""
        return {
            port.get_any_name(): port.get_shape()
            for port in self.compiled_model.inputs
        }
    
    def get_output_shapes(self) -> Dict[str, List[int]]:
        """获取输出形状"""
        return {
            port.get_any_name(): port.get_shape()
            for port in self.compiled_model.outputs
        }
    
    def infer(self, inputs: Dict[str, np.ndarray]) -> Dict[str, np.ndarray]:
        """执行推理"""
        # 准备输入张量
        input_tensors = {
            name: Tensor(array) 
            for name, array in inputs.items()
        }
        
        # 设置输入
        self.infer_request.set_input_tensors(input_tensors)
        
        # 执行推理
        self.infer_request.infer()
        
        # 获取输出
        outputs = {}
        for port in self.compiled_model.outputs:
            name = port.get_any_name()
            outputs[name] = self.infer_request.get_output_tensor(
                port.get_index()
            ).data
        
        return outputs


class AutoDeviceExecutor:
    """自动设备选择执行器（CPU/GPU/NPU）"""
    
    def __init__(self, model_path: str, preference: str = "NPU"):
        self.core = Core()
        self.model = self.core.read_model(model_path)
        
        # 设备优先级
        device_priority = self._get_device_priority(preference)
        
        # 使用 AUTO 设备
        self.compiled_model = self.core.compile_model(
            self.model,
            "AUTO",
            config={
                "DEVICE_PRIORITY": device_priority,
                "PERFORMANCE_HINT": "LATENCY",
            }
        )
        
        self.infer_request = self.compiled_model.create_infer_request()
        self._actual_device = self.compiled_model.get_property("EXECUTION_DEVICES")[0]
    
    def _get_device_priority(self, preference: str) -> str:
        """获取设备优先级字符串"""
        devices = ["NPU", "GPU", "CPU"]
        if preference in devices:
            devices.remove(preference)
            devices.insert(0, preference)
        return ",".join(devices)
    
    @property
    def actual_device(self) -> str:
        """实际执行的设备"""
        return self._actual_device
    
    def infer(self, inputs: Dict[str, np.ndarray]) -> Dict[str, np.ndarray]:
        """执行推理"""
        input_tensors = {name: Tensor(arr) for name, arr in inputs.items()}
        self.infer_request.set_input_tensors(input_tensors)
        self.infer_request.infer()
        
        return {
            port.get_any_name(): self.infer_request.get_output_tensor(
                port.get_index()
            ).data
            for port in self.compiled_model.outputs
        }
```

#### C++ 运行时

```cpp
// hsc_runtime/intel_npu.hpp
#pragma once

#include <openvino/openvino.hpp>
#include <string>
#include <vector>
#include <unordered_map>
#include <memory>

namespace hsc::npu {

/// Intel NPU 执行器
class IntelNpuExecutor {
public:
    /// 构造函数
    /// @param model_path ONNX 模型路径
    /// @param cache_dir 模型缓存目录
    explicit IntelNpuExecutor(
        const std::string& model_path,
        const std::string& cache_dir = "./cache"
    );
    
    /// 获取输入形状
    std::unordered_map<std::string, std::vector<size_t>> get_input_shapes() const;
    
    /// 获取输出形状
    std::unordered_map<std::string, std::vector<size_t>> get_output_shapes() const;
    
    /// 执行推理
    /// @param inputs 输入张量（名称 -> 数据）
    /// @return 输出张量（名称 -> 数据）
    std::unordered_map<std::string, std::vector<float>> infer(
        const std::unordered_map<std::string, std::vector<float>>& inputs
    );
    
    /// 异步推理
    void infer_async(
        const std::unordered_map<std::string, std::vector<float>>& inputs,
        std::function<void(std::unordered_map<std::string, std::vector<float>>)> callback
    );

private:
    ov::Core core_;
    std::shared_ptr<ov::Model> model_;
    ov::CompiledModel compiled_model_;
    ov::InferRequest infer_request_;
};

/// 自动设备选择执行器
class AutoDeviceExecutor {
public:
    explicit AutoDeviceExecutor(
        const std::string& model_path,
        const std::string& preference = "NPU"  // NPU, GPU, CPU
    );
    
    std::string get_actual_device() const;
    
    std::unordered_map<std::string, std::vector<float>> infer(
        const std::unordered_map<std::string, std::vector<float>>& inputs
    );

private:
    ov::Core core_;
    ov::CompiledModel compiled_model_;
    ov::InferRequest infer_request_;
    std::string actual_device_;
};

}  // namespace hsc::npu
```

### 12.5 配置文件扩展

```toml
# HSCC.toml - Intel NPU 配置

[package]
name = "intel_npu_demo"
version = "0.1.0"

[target]
device = "npu"
arch = "intel_meteor_lake"  # intel_meteor_lake, intel_lunar_lake, intel_arrow_lake

[backend]
kind = "npu"

[backend.npu]
# OpenVINO 配置
runtime = "openvino"
cache_dir = "./model_cache"

# 性能配置
performance_hint = "LATENCY"  # LATENCY, THROUGHPUT
enable_turbo = true

# 模型格式
output_format = "onnx"  # onnx, openvino_ir

# 量化配置
[backend.npu.quantization]
enabled = true
precision = "int8"  # int8, fp16
calibration_dataset = "./calibration_data"

# Intel NPU 特定配置
[backend.npu.intel]
# 设备选择策略（用于 AUTO 模式）
device_priority = "NPU,GPU,CPU"

# 编译优化级别
optimization_level = 2  # 0, 1, 2

# 内存配置
defer_weights_load = false  # 延迟权重加载（大模型）
```

### 12.6 与其他 NPU 后端的统一

```rust
// 更新 NpuDevice 枚举

/// NPU 设备类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NpuDevice {
    /// Google TPU
    TPU(TpuGeneration),
    /// 华为昇腾
    Ascend(AscendSoc),
    /// 寒武纪
    Cambrian(CambrianGeneration),
    /// 地平线 BPU
    Horizon(HorizonGeneration),
    /// Intel NPU
    IntelNPU(IntelNpuGeneration),  // 新增
    /// 通用 NPU（通过 ONNX/OpenVINO）
    Generic,
}

/// 后端工厂函数
pub fn create_npu_backend(device: NpuDevice) -> Box<dyn NpuBackend> {
    match device {
        NpuDevice::TPU(_) => Box::new(TpuBackend),
        NpuDevice::Ascend(_) => Box::new(AscendBackend),
        NpuDevice::Cambrian(_) => Box::new(CambrianBackend),
        NpuDevice::Horizon(_) => Box::new(HorizonBackend),
        NpuDevice::IntelNPU(_) => Box::new(IntelNpuBackend::new()),  // 新增
        NpuDevice::Generic => Box::new(GenericNpuBackend),
    }
}
```

### 12.7 Intel NPU 集成的技术限制

| 限制 | 影响 | 应对策略 |
|------|------|---------|
| 仅支持静态形状 | 动态批次/序列长度不支持 | 编译时确定形状，生成多个模型变体 |
| 不支持 FP64 | 部分模型无法运行 | 模型转换时使用 FP32/FP16 |
| 启动延迟 | 首次推理较慢 | 使用模型缓存，预编译 |
| 算子覆盖有限 | 部分自定义算子不支持 | 回退到 CPU/GPU，或使用 SHAVE DSP |
| Blob 跨版本不兼容 | 升级后需重新编译 | 存储 ONNX，运行时编译 |

### 12.8 实施优先级

| 阶段 | 任务 | 优先级 |
|------|------|--------|
| 1 | ONNX 导出支持 | 🔴 高 |
| 2 | OpenVINO 运行时集成 | 🔴 高 |
| 3 | 静态形状验证 | 🔴 高 |
| 4 | 模型缓存机制 | 🟡 中 |
| 5 | INT8 量化支持 | 🟡 中 |
| 6 | AUTO 设备选择 | 🟢 低 |
| 7 | 性能分析工具 | 🟢 低 |

---

## 十三、总结

### 多 NPU 厂商支持矩阵

| 厂商 | 设备 | 主要后端 | 输出格式 | 特点 |
|------|------|---------|---------|------|
| **Google** | TPU v4/v5 | TensorFlow/XLA | TFLite, SavedModel | 云端训练+推理 |
| **华为** | 昇腾 310/910 | CANN, ATC | OM (离线模型) | 边缘+云端，国产自主 |
| **寒武纪** | MLU 系列 | Neuware | Cambricon DL | 国产 AI 芯片 |
| **地平线** | BPU | Horizon NN | Horizon Model | 边缘 AIoT |
| **Intel** | NPU (Meteor/Lunar) | OpenVINO | ONNX → Blob | 低功耗，PC 集成 |

### 统一抽象的价值

通过 `NpuBackend` trait 和 `NpuGraph` 中间表示：

1. **一次编写，多设备运行**：HSCLang 代码可编译到任意支持的 NPU
2. **渐进式优化**：在 IR 层统一优化，后端只负责设备特化
3. **灵活扩展**：新增 NPU 厂商只需实现 `NpuBackend` trait
4. **性能可移植**：自动调优器针对不同硬件选择最优配置

这样，OpenHC 项目可以构建完整的异构计算支持矩阵：GPU (CUDA/Triton) + NPU (多厂商) + FPGA (HLS)，真正实现"单一来源，多设备生成"的设计目标。
