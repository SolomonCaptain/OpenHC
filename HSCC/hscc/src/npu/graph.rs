//! NPU 计算图表示
//!
//! 定义 NPU 计算图的数据结构，包括：
//! - 图节点（操作）
//! - 张量（数据）
//! - 执行策略

use std::collections::HashMap;
use super::types::{NpuType, TensorLayout};

/// NPU 计算图
#[derive(Debug, Default, Clone)]
pub struct NpuGraph {
    /// 图名称
    pub name: String,
    /// 输入节点
    pub inputs: Vec<NpuValue>,
    /// 输出节点
    pub outputs: Vec<NpuValue>,
    /// 操作节点（按拓扑排序）
    pub operations: Vec<NpuOperation>,
    /// 中间张量
    pub tensors: HashMap<String, NpuTensor>,
    /// 内存规划
    pub memory_plan: Option<super::memory::MemoryPlan>,
    /// 执行策略
    pub execution_policy: ExecutionPolicy,
    /// 元数据
    pub metadata: GraphMetadata,
}

/// 图元数据
#[derive(Debug, Clone, Default)]
pub struct GraphMetadata {
    /// 生成时间
    pub generated_at: String,
    /// 编译器版本
    pub compiler_version: String,
    /// 源文件
    pub source_file: String,
    /// 额外属性
    pub attributes: HashMap<String, String>,
}

/// NPU 值（图中的数据流）
#[derive(Debug, Clone)]
pub struct NpuValue {
    /// 值名称
    pub name: String,
    /// 值类型
    pub value_type: NpuValueType,
    /// 数据类型
    pub dtype: NpuType,
    /// 形状
    pub shape: Vec<i64>,
    /// 来源操作索引（-1 表示输入）
    pub producer: i32,
}

/// 值类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpuValueType {
    /// 输入张量
    Input,
    /// 输出张量
    Output,
    /// 中间张量
    Intermediate,
    /// 常量
    Constant,
}

/// NPU 张量
#[derive(Debug, Clone)]
pub struct NpuTensor {
    /// 张量 ID
    pub id: u64,
    /// 张量名称
    pub name: String,
    /// 数据类型
    pub dtype: NpuType,
    /// 形状
    pub shape: Vec<i64>,
    /// 内存布局
    pub layout: TensorLayout,
    /// 内存偏移（由内存规划器分配）
    pub memory_offset: Option<usize>,
    /// 是否需要量化
    pub requires_quantization: bool,
    /// 量化参数
    pub quant_params: Option<QuantParams>,
    /// 生命周期起始（操作索引）
    pub lifetime_start: usize,
    /// 生命周期结束（操作索引）
    pub lifetime_end: usize,
}

/// 量化参数
#[derive(Debug, Clone)]
pub struct QuantParams {
    /// 缩放因子
    pub scale: f32,
    /// 零点
    pub zero_point: i32,
    /// 量化轴（-1 表示 per-tensor）
    pub axis: i32,
}

/// NPU 操作
#[derive(Debug, Clone)]
pub struct NpuOperation {
    /// 操作索引
    pub index: usize,
    /// 操作类型
    pub op_type: NpuOpType,
    /// 操作名称
    pub name: String,
    /// 输入张量名称
    pub inputs: Vec<String>,
    /// 输出张量名称
    pub outputs: Vec<String>,
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
    /// 矩阵转置
    Transpose { perm: Vec<i32> },

    // ─── 卷积运算 ───
    /// 2D 卷积
    Conv2D {
        padding: Padding,
        stride: (u32, u32),
        dilation: (u32, u32),
        groups: u32,
    },
    /// 深度可分离卷积
    DepthwiseConv2D {
        padding: Padding,
        stride: (u32, u32),
        depth_multiplier: u32,
    },
    /// 转置卷积（反卷积）
    ConvTranspose2D {
        padding: Padding,
        stride: (u32, u32),
        output_padding: (u32, u32),
    },

    // ─── 激活函数 ───
    ReLU,
    ReLU6,
    LeakyReLU { alpha: f32 },
    Sigmoid,
    Tanh,
    GELU,
    Swish,
    SiLU,
    HardSwish,
    Softmax { axis: i32 },
    LogSoftmax { axis: i32 },

    // ─── 归一化 ───
    BatchNorm { epsilon: f32, momentum: f32 },
    LayerNorm { epsilon: f32, axis: i32 },
    InstanceNorm { epsilon: f32 },
    GroupNorm { num_groups: u32, epsilon: f32 },

    // ─── 池化 ───
    MaxPool2D { kernel: (u32, u32), stride: (u32, u32), padding: Padding },
    AvgPool2D { kernel: (u32, u32), stride: (u32, u32), padding: Padding },
    GlobalAvgPool,
    GlobalMaxPool,
    AdaptiveAvgPool { output_size: (u32, u32) },
    AdaptiveMaxPool { output_size: (u32, u32) },

    // ─── 逐元素运算 ───
    Add,
    BiasAdd,  // 偏置加法（常用于卷积后）
    Sub,
    Mul,
    Div,
    Exp,
    Log,
    Sqrt,
    Pow,
    Neg,
    Abs,
    Min,
    Max,
    Clip { min: f32, max: f32 },
    // 三角函数
    Sin,
    Cos,
    Tan,

    // ─── 归约运算 ───
    ReduceSum { axes: Vec<i32>, keep_dims: bool },
    ReduceMean { axes: Vec<i32>, keep_dims: bool },
    ReduceMax { axes: Vec<i32>, keep_dims: bool },
    ReduceMin { axes: Vec<i32>, keep_dims: bool },
    ReduceProd { axes: Vec<i32>, keep_dims: bool },
    ReduceL2 { axes: Vec<i32>, keep_dims: bool },

    // ─── 张量变换 ───
    Reshape,
    Flatten { axis: i32 },
    Squeeze { axes: Vec<i32> },
    Unsqueeze { axes: Vec<i32> },
    Expand,
    Concat { axis: i32 },
    Split { axis: i32, split_sizes: Vec<i64> },
    Slice {
        starts: Vec<i64>,
        ends: Vec<i64>,
        axes: Option<Vec<i32>>,
        steps: Option<Vec<i64>>,
    },
    Tile,
    Gather { axis: i32 },
    ScatterND,
    NonZero,
    TopK { k: i64, axis: i32 },

    // ─── 注意力机制 ───
    /// Flash Attention（NPU 优化关键）
    FlashAttention {
        scale: f32,
        causal: bool,
    },
    MultiHeadAttention {
        num_heads: u32,
    },
    ScaledDotProductAttention {
        scale: f32,
    },

    // ─── 量化相关 ───
    Quantize { scale: f32, zero_point: i32 },
    Dequantize { scale: f32, zero_point: i32 },
    Requantize { scale_input: f32, scale_output: f32, zero_point: i32 },

    // ─── 控制流 ───
    If { then_branch: String, else_branch: String },
    Loop { trip_count: i64 },

    // ─── 嵌入和查找 ───
    GatherEmbedding,
    EmbeddingBag,

    // ─── 循环神经网络 ───
    LSTM {
        hidden_size: u32,
        num_layers: u32,
        bidirectional: bool,
    },
    GRU {
        hidden_size: u32,
        num_layers: u32,
        bidirectional: bool,
    },

    // ─── 其他 ───
    Dropout { ratio: f32 },
    Identity,
    Cast,
    Constant { value: Vec<f32> },
    Custom { op_name: String },
}

/// 填充类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Padding {
    /// 无填充
    Valid,
    /// 相同大小（自动填充）
    Same,
    /// 显式填充 (top, bottom, left, right)
    Explicit(u32, u32, u32, u32),
}

impl Default for Padding {
    fn default() -> Self {
        Padding::Valid
    }
}

/// 操作属性值
#[derive(Debug, Clone)]
pub enum NpuAttribute {
    Int(i64),
    Float(f32),
    String(String),
    Ints(Vec<i64>),
    Floats(Vec<f32>),
    Tensor(NpuTensor),
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
    /// 目标设备提示
    pub device_hint: Option<String>,
    /// 量化提示
    pub quant_hint: Option<QuantHint>,
}

/// 量化提示
#[derive(Debug, Clone)]
pub struct QuantHint {
    /// 是否使用 INT8
    pub use_int8: bool,
    /// 是否使用对称量化
    pub symmetric: bool,
    /// 每通道量化
    pub per_channel: bool,
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

impl Default for ExecutionPolicy {
    fn default() -> Self {
        Self {
            pipeline_mode: PipelineMode::SingleStream,
            memory_reuse: MemoryReuseStrategy::Greedy,
            parallel_strategy: ParallelStrategy::Sequential,
        }
    }
}

/// 流水线模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineMode {
    /// 单流执行
    SingleStream,
    /// 多流并行
    MultiStream { num_streams: u32 },
    /// 流水线并行
    Pipelined { stages: u32 },
}

/// 内存复用策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryReuseStrategy {
    /// 无复用
    None,
    /// 贪婪复用（基于生命周期）
    Greedy,
    /// 最优化复用（可能增加编译时间）
    Optimal,
}

/// 并行策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParallelStrategy {
    /// 顺序执行
    Sequential,
    /// 数据并行
    DataParallel { batch_size: u32 },
    /// 模型并行
    ModelParallel { num_partitions: u32 },
}

impl NpuGraph {
    /// 创建新的计算图
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Default::default()
        }
    }

    /// 添加输入
    pub fn add_input(&mut self, name: &str, dtype: NpuType, shape: Vec<i64>) -> &mut NpuValue {
        let value = NpuValue {
            name: name.to_string(),
            value_type: NpuValueType::Input,
            dtype,
            shape,
            producer: -1,
        };
        self.inputs.push(value);
        self.inputs.last_mut().unwrap()
    }

    /// 添加输出
    pub fn add_output(&mut self, name: &str, dtype: NpuType, shape: Vec<i64>) -> &mut NpuValue {
        let value = NpuValue {
            name: name.to_string(),
            value_type: NpuValueType::Output,
            dtype,
            shape,
            producer: -1,
        };
        self.outputs.push(value);
        self.outputs.last_mut().unwrap()
    }

    /// 添加操作
    pub fn add_operation(&mut self, op: NpuOperation) {
        let idx = self.operations.len();
        self.operations.push(op);
        // 更新输出张量的 producer
        for output_name in &self.operations[idx].outputs.clone() {
            if let Some(tensor) = self.tensors.get_mut(output_name) {
                tensor.lifetime_start = idx;
            }
        }
    }

    /// 添加中间张量
    pub fn add_tensor(&mut self, tensor: NpuTensor) {
        self.tensors.insert(tensor.name.clone(), tensor);
    }

    /// 获取拓扑排序后的操作列表
    pub fn topological_sort(&self) -> Vec<&NpuOperation> {
        // 简单实现：假设已经是拓扑排序的
        self.operations.iter().collect()
    }

    /// 计算张量生命周期
    pub fn compute_lifetimes(&mut self) {
        for (idx, op) in self.operations.iter().enumerate() {
            // 更新输出张量的生命周期开始
            for output_name in &op.outputs {
                if let Some(tensor) = self.tensors.get_mut(output_name) {
                    tensor.lifetime_start = idx;
                }
            }
            // 更新输入张量的生命周期结束
            for input_name in &op.inputs {
                if let Some(tensor) = self.tensors.get_mut(input_name) {
                    tensor.lifetime_end = tensor.lifetime_end.max(idx);
                }
            }
        }
        // 输出张量的生命周期到末尾
        for output in &self.outputs {
            if let Some(tensor) = self.tensors.get_mut(&output.name) {
                tensor.lifetime_end = self.operations.len();
            }
        }
    }

    /// 验证图的有效性
    pub fn validate(&self) -> Result<(), String> {
        // 检查所有输入都有对应的张量
        for input in &self.inputs {
            if !self.tensors.contains_key(&input.name) {
                return Err(format!("Input '{}' not found in tensors", input.name));
            }
        }

        // 检查所有操作的输入都存在
        for op in &self.operations {
            for input_name in &op.inputs {
                if !self.tensors.contains_key(input_name)
                    && !self.inputs.iter().any(|i| i.name == *input_name)
                {
                    return Err(format!(
                        "Operation '{}' input '{}' not found",
                        op.name, input_name
                    ));
                }
            }
        }

        Ok(())
    }

    /// 序列化为 ONNX 格式
    pub fn to_onnx(&self) -> Result<Vec<u8>, String> {
        // TODO: 实现 ONNX 序列化
        // 可以使用 protobuf 库或 tract-onnx
        Err("ONNX serialization not implemented yet".to_string())
    }

    /// 生成图的字符串表示（用于调试）
    pub fn to_string_repr(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("Graph: {}\n", self.name));
        s.push_str("Inputs:\n");
        for input in &self.inputs {
            s.push_str(&format!("  {} : {:?} {:?}\n", input.name, input.dtype, input.shape));
        }
        s.push_str("Operations:\n");
        for op in &self.operations {
            s.push_str(&format!("  {} : {:?}\n", op.name, op.op_type));
        }
        s.push_str("Outputs:\n");
        for output in &self.outputs {
            s.push_str(&format!("  {} : {:?} {:?}\n", output.name, output.dtype, output.shape));
        }
        s
    }
}

impl NpuOpType {
    /// 获取操作名称
    pub fn name(&self) -> &str {
        match self {
            NpuOpType::MatMul => "MatMul",
            NpuOpType::BatchMatMul => "BatchMatMul",
            NpuOpType::MatVec => "MatVec",
            NpuOpType::Transpose { .. } => "Transpose",
            NpuOpType::Conv2D { .. } => "Conv",
            NpuOpType::DepthwiseConv2D { .. } => "DepthwiseConv2D",
            NpuOpType::ConvTranspose2D { .. } => "ConvTranspose",
            NpuOpType::ReLU => "Relu",
            NpuOpType::ReLU6 => "Relu6",
            NpuOpType::LeakyReLU { .. } => "LeakyRelu",
            NpuOpType::Sigmoid => "Sigmoid",
            NpuOpType::Tanh => "Tanh",
            NpuOpType::GELU => "Gelu",
            NpuOpType::Swish => "Swish",
            NpuOpType::SiLU => "Silu",
            NpuOpType::HardSwish => "HardSwish",
            NpuOpType::Softmax { .. } => "Softmax",
            NpuOpType::LogSoftmax { .. } => "LogSoftmax",
            NpuOpType::BatchNorm { .. } => "BatchNormalization",
            NpuOpType::LayerNorm { .. } => "LayerNormalization",
            NpuOpType::InstanceNorm { .. } => "InstanceNormalization",
            NpuOpType::GroupNorm { .. } => "GroupNormalization",
            NpuOpType::MaxPool2D { .. } => "MaxPool",
            NpuOpType::AvgPool2D { .. } => "AveragePool",
            NpuOpType::GlobalAvgPool => "GlobalAveragePool",
            NpuOpType::GlobalMaxPool => "GlobalMaxPool",
            NpuOpType::AdaptiveAvgPool { .. } => "AdaptiveAveragePool",
            NpuOpType::AdaptiveMaxPool { .. } => "AdaptiveMaxPool",
            NpuOpType::Add => "Add",
            NpuOpType::BiasAdd => "Add",  // BiasAdd 在 ONNX 中就是 Add
            NpuOpType::Sub => "Sub",
            NpuOpType::Mul => "Mul",
            NpuOpType::Div => "Div",
            NpuOpType::Exp => "Exp",
            NpuOpType::Log => "Log",
            NpuOpType::Sqrt => "Sqrt",
            NpuOpType::Pow => "Pow",
            NpuOpType::Neg => "Neg",
            NpuOpType::Abs => "Abs",
            NpuOpType::Min => "Min",
            NpuOpType::Max => "Max",
            NpuOpType::Clip { .. } => "Clip",
            NpuOpType::Sin => "Sin",
            NpuOpType::Cos => "Cos",
            NpuOpType::Tan => "Tan",
            NpuOpType::ReduceSum { .. } => "ReduceSum",
            NpuOpType::ReduceMean { .. } => "ReduceMean",
            NpuOpType::ReduceMax { .. } => "ReduceMax",
            NpuOpType::ReduceMin { .. } => "ReduceMin",
            NpuOpType::ReduceProd { .. } => "ReduceProd",
            NpuOpType::ReduceL2 { .. } => "ReduceL2",
            NpuOpType::Reshape => "Reshape",
            NpuOpType::Flatten { .. } => "Flatten",
            NpuOpType::Squeeze { .. } => "Squeeze",
            NpuOpType::Unsqueeze { .. } => "Unsqueeze",
            NpuOpType::Expand => "Expand",
            NpuOpType::Concat { .. } => "Concat",
            NpuOpType::Split { .. } => "Split",
            NpuOpType::Slice { .. } => "Slice",
            NpuOpType::Tile => "Tile",
            NpuOpType::Gather { .. } => "Gather",
            NpuOpType::ScatterND => "ScatterND",
            NpuOpType::NonZero => "NonZero",
            NpuOpType::TopK { .. } => "TopK",
            NpuOpType::FlashAttention { .. } => "FlashAttention",
            NpuOpType::MultiHeadAttention { .. } => "MultiHeadAttention",
            NpuOpType::ScaledDotProductAttention { .. } => "ScaledDotProductAttention",
            NpuOpType::Quantize { .. } => "QuantizeLinear",
            NpuOpType::Dequantize { .. } => "DequantizeLinear",
            NpuOpType::Requantize { .. } => "Requantize",
            NpuOpType::If { .. } => "If",
            NpuOpType::Loop { .. } => "Loop",
            NpuOpType::GatherEmbedding => "GatherEmbedding",
            NpuOpType::EmbeddingBag => "EmbeddingBag",
            NpuOpType::LSTM { .. } => "LSTM",
            NpuOpType::GRU { .. } => "GRU",
            NpuOpType::Dropout { .. } => "Dropout",
            NpuOpType::Identity => "Identity",
            NpuOpType::Cast => "Cast",
            NpuOpType::Constant { .. } => "Constant",
            NpuOpType::Custom { op_name } => op_name,
        }
    }

    /// 检查是否支持融合
    pub fn is_fusible(&self) -> bool {
        matches!(self,
            NpuOpType::ReLU | NpuOpType::ReLU6 | NpuOpType::Sigmoid |
            NpuOpType::Tanh | NpuOpType::GELU | NpuOpType::Swish |
            NpuOpType::Add | NpuOpType::Mul | NpuOpType::BiasAdd
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::npu::types::NpuType;

    #[test]
    fn test_graph_creation() {
        let mut graph = NpuGraph::new("test_graph");
        graph.add_input("x", NpuType::f32(), vec![1, 3, 224, 224]);
        graph.add_output("y", NpuType::f32(), vec![1, 1000]);

        assert_eq!(graph.inputs.len(), 1);
        assert_eq!(graph.outputs.len(), 1);
    }

    #[test]
    fn test_operation_creation() {
        let op = NpuOperation {
            index: 0,
            op_type: NpuOpType::MatMul,
            name: "matmul_0".to_string(),
            inputs: vec!["A".to_string(), "B".to_string()],
            outputs: vec!["C".to_string()],
            attributes: HashMap::new(),
            hints: OpHints::default(),
        };

        assert_eq!(op.op_type.name(), "MatMul");
    }
}
