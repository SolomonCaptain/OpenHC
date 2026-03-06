//! NPU 自动调优
//!
//! 根据 NPU 硬件特性自动选择最优执行配置。

use std::collections::HashMap;
use crate::npu::graph::{NpuGraph, NpuOpType};
use crate::npu::backends::NpuHardwareSpec;

/// NPU 自动调优器
pub struct NpuAutoTuner {
    /// 硬件规格
    spec: NpuHardwareSpec,
    /// 调优历史
    tuning_history: HashMap<String, TuningResult>,
}

/// 调优结果
#[derive(Debug, Clone)]
pub struct TuningResult {
    /// 配置签名
    pub signature: String,
    /// 最优参数
    pub params: NpuTuningParams,
    /// 预估性能
    pub estimated_latency_us: f64,
}

/// NPU 调优参数
#[derive(Debug, Clone)]
pub struct NpuTuningParams {
    /// 分块大小
    pub tile_sizes: Vec<Vec<i64>>,
    /// 流水线深度
    pub pipeline_depth: u32,
    /// 双缓冲
    pub double_buffer: bool,
    /// 量化策略
    pub quant_strategy: Option<QuantStrategy>,
    /// 并行流数量
    pub num_streams: u32,
}

/// 量化策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantStrategy {
    /// 不量化
    None,
    /// INT8 动态量化
    Int8Dynamic,
    /// INT8 静态量化
    Int8Static,
}

/// 问题规模
#[derive(Debug, Clone)]
pub struct ProblemSize {
    /// 批量大小
    pub batch: i64,
    /// 序列长度（用于 Transformer）
    pub seq_len: i64,
    /// 隐藏维度
    pub hidden_size: i64,
    /// 额外维度
    pub extra_dims: Vec<i64>,
}

impl Default for NpuTuningParams {
    fn default() -> Self {
        Self {
            tile_sizes: vec![vec![64, 64, 64]],
            pipeline_depth: 1,
            double_buffer: false,
            quant_strategy: None,
            num_streams: 1,
        }
    }
}

impl NpuAutoTuner {
    /// 创建自动调优器
    pub fn new(spec: NpuHardwareSpec) -> Self {
        Self {
            spec,
            tuning_history: HashMap::new(),
        }
    }

    /// 计算图签名（用于缓存调优结果）
    pub fn graph_signature(&self, graph: &NpuGraph) -> String {
        let mut sig = String::new();
        sig.push_str(&graph.name);

        for input in &graph.inputs {
            sig.push_str(&format!(":{}:{:?}", input.name, input.shape));
        }

        for op in &graph.operations {
            sig.push_str(&format!(":{}", op.op_type.name()));
        }

        // 简单哈希
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        sig.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// 调优
    pub fn tune(&mut self, graph: &NpuGraph) -> NpuTuningParams {
        let key = self.graph_signature(graph);

        // 检查缓存
        if let Some(result) = self.tuning_history.get(&key) {
            return result.params.clone();
        }

        // 分析图特征
        let features = self.analyze_graph_features(graph);

        // 选择最优配置
        let params = self.select_best_params(&features);

        // 缓存结果
        let result = TuningResult {
            signature: key.clone(),
            params: params.clone(),
            estimated_latency_us: self.estimate_latency(graph, &params),
        };
        self.tuning_history.insert(key, result);

        params
    }

    /// 分析图特征
    fn analyze_graph_features(&self, graph: &NpuGraph) -> GraphFeatures {
        let mut features = GraphFeatures::default();

        for op in &graph.operations {
            match &op.op_type {
                NpuOpType::MatMul => {
                    features.num_matmul += 1;
                }
                NpuOpType::Conv2D { .. } => {
                    features.num_conv += 1;
                }
                NpuOpType::FlashAttention { .. } | NpuOpType::MultiHeadAttention { .. } => {
                    features.num_attention += 1;
                }
                NpuOpType::LayerNorm { .. } | NpuOpType::BatchNorm { .. } => {
                    features.num_norm += 1;
                }
                _ => {}
            }
        }

        // 估算内存需求
        features.memory_estimate = graph.tensors.values()
            .map(|t| t.dtype.size_in_bytes())
            .sum();

        features
    }

    /// 选择最优参数
    fn select_best_params(&self, features: &GraphFeatures) -> NpuTuningParams {
        let mut params = NpuTuningParams::default();

        // 基于图特征调整参数
        if features.num_matmul > 0 {
            // MatMul 密集型
            params.tile_sizes = self.get_matmul_tile_sizes();
            params.pipeline_depth = 2;
            params.double_buffer = true;
        }

        if features.num_conv > 0 {
            // Conv 密集型
            params.tile_sizes = self.get_conv_tile_sizes();
            params.pipeline_depth = 4;
        }

        if features.num_attention > 0 {
            // Attention 密集型
            params.pipeline_depth = 1; // Attention 通常不适合深度流水线
            params.num_streams = 2;
        }

        // 内存优化
        if features.memory_estimate > self.spec.local_memory_kb as usize * 1024 {
            params.double_buffer = false; // 内存不足，禁用双缓冲
        }

        // 量化建议
        if features.num_matmul + features.num_conv > 5 {
            params.quant_strategy = Some(QuantStrategy::Int8Dynamic);
        }

        params
    }

    /// 获取 MatMul 分块大小
    fn get_matmul_tile_sizes(&self) -> Vec<Vec<i64>> {
        // 基于 systolic array 大小
        let (m, n, k) = self.spec.matrix_unit.systolic_array;
        vec![
            vec![m as i64, k as i64],  // A 分块
            vec![k as i64, n as i64],  // B 分块
        ]
    }

    /// 获取 Conv 分块大小
    fn get_conv_tile_sizes(&self) -> Vec<Vec<i64>> {
        vec![
            vec![1, 16, 16, 16],  // 输出分块
            vec![16, 16, 3, 3],   // 卷积核分块
        ]
    }

    /// 预估延迟
    fn estimate_latency(&self, graph: &NpuGraph, params: &NpuTuningParams) -> f64 {
        // 简单的性能模型
        let mut total_ops = 0.0;

        for op in &graph.operations {
            match &op.op_type {
                NpuOpType::MatMul => {
                    // 假设标准矩阵乘
                    total_ops += 1_000_000.0; // 1M ops
                }
                NpuOpType::Conv2D { .. } => {
                    total_ops += 5_000_000.0;
                }
                NpuOpType::FlashAttention { .. } => {
                    total_ops += 2_000_000.0;
                }
                _ => {
                    total_ops += 10_000.0;
                }
            }
        }

        // 基于 TOPS 估算
        let tops = self.spec.matrix_unit.peak_tops;
        let compute_time_us = total_ops / (tops * 1_000_000.0);

        // 流水线效果
        let pipeline_factor = if params.pipeline_depth > 1 {
            1.0 / (params.pipeline_depth as f64).sqrt()
        } else {
            1.0
        };

        // 双缓冲效果
        let buffer_factor = if params.double_buffer { 0.9 } else { 1.0 };

        compute_time_us * pipeline_factor * buffer_factor
    }
}

/// 图特征
#[derive(Debug, Clone, Default)]
struct GraphFeatures {
    /// MatMul 操作数量
    num_matmul: usize,
    /// Conv 操作数量
    num_conv: usize,
    /// Attention 操作数量
    num_attention: usize,
    /// 归一化操作数量
    num_norm: usize,
    /// 内存需求估算（字节）
    memory_estimate: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::npu::backends::intel_npu::IntelNpuBackend;
    use crate::npu::backends::{NpuBackend, NpuDevice};
    use crate::npu::backends::intel_npu::IntelNpuGeneration;

    #[test]
    fn test_autotuner_creation() {
        let backend = IntelNpuBackend::new();
        let spec = backend.hardware_spec(NpuDevice::IntelNPU(IntelNpuGeneration::MeteorLake));
        let tuner = NpuAutoTuner::new(spec);

        assert!(tuner.tuning_history.is_empty());
    }

    #[test]
    fn test_default_params() {
        let params = NpuTuningParams::default();
        assert_eq!(params.pipeline_depth, 1);
        assert!(!params.double_buffer);
    }
}
