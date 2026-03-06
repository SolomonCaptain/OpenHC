//! Intel NPU 后端实现
//!
//! 通过 OpenVINO 工具包支持 Intel NPU（Meteor Lake, Lunar Lake, Arrow Lake）。
//!
//! ## 架构概述
//!
//! Intel NPU 源自 Movidius（2016 年收购），首次集成于 Meteor Lake 处理器。
//!
//! ### 硬件特性 (Meteor Lake NPU 3720)
//! - 2 × NCE (Neural Compute Engine) tiles
//! - 512 MPEs/tile = 1024 MPEs total
//! - 4096 INT8 MACs/cycle = **9.5 TOPS**
//! - 2048 FP16 MACs/cycle = **4.7 TFLOPS**
//! - 4 MB 片上 SRAM
//! - LEON SPARC 控制器 (LeonRT + LeonNN)
//! - SHAVE DSP 核心
//!
//! ### 数据类型支持
//! - INT8: ✅ 全速
//! - FP16: ✅ 半速
//! - FP32: ⚠️ 有限（通过 SHAVE DSP）
//! - FP64: ❌ 不支持
//!
//! ## OpenVINO 集成
//!
//! 编译流程:
//! ```text
//! NpuGraph → ONNX → OpenVINO IR → NPU Blob
//! ```

use std::time::Duration;
use std::collections::HashMap;
use super::{
    NpuBackend, NpuDevice, NpuHardwareSpec, NpuCode, NpuError,
    RuntimeConfig, TensorDesc, ExecutionConfig, PerformanceHint,
    MemoryPoolConfig, MatrixUnitSpec, VectorUnitSpec, QuantSupport,
    VectorOp,
};
use crate::npu::types::{NpuType, NpuTypeKind, TensorLayout};
use crate::npu::graph::{NpuGraph, NpuOperation, NpuOpType, Padding};
use crate::npu::memory::MemoryPlan;

/// Intel NPU 代次
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntelNpuGeneration {
    /// Meteor Lake (Core Ultra Series 1)
    /// - NPU 3720
    /// - 9.5 TOPS INT8
    /// - 4 MB SRAM
    MeteorLake,
    /// Lunar Lake (Core Ultra Series 2)
    /// - 约 48 TOPS NPU
    /// - 更强的 AI 性能
    LunarLake,
    /// Arrow Lake (桌面版)
    ArrowLake,
}

impl Default for IntelNpuGeneration {
    fn default() -> Self {
        IntelNpuGeneration::MeteorLake
    }
}

/// Intel NPU 设备信息
#[derive(Debug, Clone)]
pub struct IntelNpuDevice {
    /// 代次
    pub generation: IntelNpuGeneration,
    /// NCE tiles 数量
    pub num_tiles: u32,
    /// 每 tile SRAM 大小 (KB)
    pub sram_per_tile_kb: u32,
    /// 峰值 INT8 TOPS
    pub peak_tops: f64,
    /// 峰值 FP16 TFLOPS
    pub peak_tflops: f64,
    /// 时钟频率 (MHz)
    pub clock_mhz: u32,
}

impl IntelNpuDevice {
    /// 获取 Meteor Lake 规格
    pub fn meteor_lake() -> Self {
        Self {
            generation: IntelNpuGeneration::MeteorLake,
            num_tiles: 2,
            sram_per_tile_kb: 2048, // 2 MB per tile
            peak_tops: 9.5,
            peak_tflops: 4.7,
            clock_mhz: 1160, // 1.16 GHz
        }
    }

    /// 获取 Lunar Lake 规格
    pub fn lunar_lake() -> Self {
        Self {
            generation: IntelNpuGeneration::LunarLake,
            num_tiles: 4,
            sram_per_tile_kb: 2048,
            peak_tops: 48.0, // 约 48 TOPS
            peak_tflops: 24.0,
            clock_mhz: 1500,
        }
    }

    /// 获取 Arrow Lake 规格
    pub fn arrow_lake() -> Self {
        Self {
            generation: IntelNpuGeneration::ArrowLake,
            num_tiles: 2,
            sram_per_tile_kb: 2048,
            peak_tops: 13.0,
            peak_tflops: 6.5,
            clock_mhz: 1300,
        }
    }
}

/// Intel NPU 后端 (通过 OpenVINO)
pub struct IntelNpuBackend {
    /// 设备配置
    config: IntelNpuConfig,
}

/// Intel NPU 配置
#[derive(Debug, Clone)]
pub struct IntelNpuConfig {
    /// 目标代次
    pub generation: IntelNpuGeneration,
    /// 性能模式
    pub performance_hint: PerformanceHint,
    /// 是否启用 Turbo 模式
    pub turbo: bool,
    /// 是否启用模型缓存
    pub enable_cache: bool,
    /// 缓存目录
    pub cache_dir: String,
    /// 量化策略
    pub quantization: QuantizationStrategy,
}

/// 量化策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantizationStrategy {
    /// 不量化
    None,
    /// INT8 静态量化
    Int8Static,
    /// INT8 动态量化
    Int8Dynamic,
}

impl Default for IntelNpuConfig {
    fn default() -> Self {
        Self {
            generation: IntelNpuGeneration::MeteorLake,
            performance_hint: PerformanceHint::Latency,
            turbo: false,
            enable_cache: true,
            cache_dir: "./model_cache".to_string(),
            quantization: QuantizationStrategy::None,
        }
    }
}

impl IntelNpuBackend {
    /// 创建新的 Intel NPU 后端
    pub fn new() -> Self {
        Self {
            config: IntelNpuConfig::default(),
        }
    }

    /// 使用配置创建后端
    pub fn with_config(config: IntelNpuConfig) -> Self {
        Self { config }
    }

    /// 获取设备规格
    fn get_device_spec(&self, generation: IntelNpuGeneration) -> IntelNpuDevice {
        match generation {
            IntelNpuGeneration::MeteorLake => IntelNpuDevice::meteor_lake(),
            IntelNpuGeneration::LunarLake => IntelNpuDevice::lunar_lake(),
            IntelNpuGeneration::ArrowLake => IntelNpuDevice::arrow_lake(),
        }
    }

    /// 检查静态形状（OpenVINO NPU 要求）
    fn ensure_static_shapes(&self, graph: &NpuGraph) -> Result<(), NpuError> {
        for tensor in graph.tensors.values() {
            if tensor.shape.iter().any(|d| *d < 0) {
                return Err(NpuError::UnsupportedDynamicShape {
                    tensor: tensor.name.clone(),
                    reason: "OpenVINO NPU Plugin only supports static shapes".to_string(),
                });
            }
        }
        Ok(())
    }

    /// 检查数据类型支持
    fn ensure_supported_dtypes(&self, graph: &NpuGraph) -> Result<(), NpuError> {
        for tensor in graph.tensors.values() {
            match &tensor.dtype.kind {
                NpuTypeKind::Float { width: 64 } => {
                    return Err(NpuError::UnsupportedDataType {
                        dtype: "FP64".to_string(),
                        reason: "Intel NPU does not support FP64 operations".to_string(),
                    });
                }
                NpuTypeKind::Integer { width: 128, .. } => {
                    return Err(NpuError::UnsupportedDataType {
                        dtype: "INT128".to_string(),
                        reason: "Intel NPU does not support 128-bit integers".to_string(),
                    });
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// 检查操作支持（OpenVINO NPU Plugin 支持的操作）
    fn is_op_supported_by_openvino(&self, op: &NpuOpType) -> bool {
        matches!(op,
            // 矩阵运算
            NpuOpType::MatMul |
            NpuOpType::BatchMatMul |
            NpuOpType::Transpose { .. } |

            // 卷积
            NpuOpType::Conv2D { .. } |
            NpuOpType::DepthwiseConv2D { .. } |
            NpuOpType::ConvTranspose2D { .. } |

            // 激活函数
            NpuOpType::ReLU |
            NpuOpType::ReLU6 |
            NpuOpType::LeakyReLU { .. } |
            NpuOpType::Sigmoid |
            NpuOpType::Tanh |
            NpuOpType::GELU |
            NpuOpType::Swish |
            NpuOpType::SiLU |
            NpuOpType::Softmax { .. } |

            // 归一化
            NpuOpType::BatchNorm { .. } |
            NpuOpType::LayerNorm { .. } |

            // 池化
            NpuOpType::MaxPool2D { .. } |
            NpuOpType::AvgPool2D { .. } |
            NpuOpType::GlobalAvgPool |
            NpuOpType::GlobalMaxPool |
            NpuOpType::AdaptiveAvgPool { .. } |
            NpuOpType::AdaptiveMaxPool { .. } |

            // 逐元素
            NpuOpType::Add |
            NpuOpType::Sub |
            NpuOpType::Mul |
            NpuOpType::Div |
            NpuOpType::Exp |
            NpuOpType::Log |
            NpuOpType::Sqrt |
            NpuOpType::Pow |
            NpuOpType::Min |
            NpuOpType::Max |
            NpuOpType::Clip { .. } |

            // 归约
            NpuOpType::ReduceSum { .. } |
            NpuOpType::ReduceMean { .. } |
            NpuOpType::ReduceMax { .. } |
            NpuOpType::ReduceMin { .. } |

            // 张量变换
            NpuOpType::Reshape |
            NpuOpType::Flatten { .. } |
            NpuOpType::Squeeze { .. } |
            NpuOpType::Unsqueeze { .. } |
            NpuOpType::Concat { .. } |
            NpuOpType::Split { .. } |
            NpuOpType::Slice { .. } |
            NpuOpType::Gather { .. } |

            // 量化
            NpuOpType::Quantize { .. } |
            NpuOpType::Dequantize { .. } |

            // 其他
            NpuOpType::Identity |
            NpuOpType::Cast |
            NpuOpType::Constant { .. }
        )
    }

    /// 生成 ONNX 模型
    fn generate_onnx(&self, graph: &NpuGraph) -> Result<Vec<u8>, NpuError> {
        // 构建 ONNX ModelProto
        let mut onnx_model = OnnxModelBuilder::new(&graph.name);

        // 添加输入
        for input in &graph.inputs {
            onnx_model.add_input(
                &input.name,
                input.dtype.to_onnx_type(),
                input.shape.clone(),
            );
        }

        // 添加输出
        for output in &graph.outputs {
            onnx_model.add_output(
                &output.name,
                output.dtype.to_onnx_type(),
                output.shape.clone(),
            );
        }

        // 添加节点
        for op in &graph.operations {
            onnx_model.add_node(
                op.op_type.name(),
                &op.name,
                op.inputs.clone(),
                op.outputs.clone(),
                self.op_attributes_to_onnx(&op.op_type, &op.attributes),
            );
        }

        onnx_model.build()
    }

    /// 转换操作属性为 ONNX 格式
    fn op_attributes_to_onnx(
        &self,
        op_type: &NpuOpType,
        attrs: &HashMap<String, super::super::graph::NpuAttribute>,
    ) -> HashMap<String, String> {
        let mut onnx_attrs = HashMap::new();

        match op_type {
            NpuOpType::Conv2D { padding, stride, dilation, groups } => {
                let pads = match padding {
                    Padding::Valid => vec![0, 0, 0, 0],
                    Padding::Same => vec![-1, -1, -1, -1], // AutoPad
                    Padding::Explicit(t, b, l, r) => vec![*t as i32, *b as i32, *l as i32, *r as i32],
                };
                onnx_attrs.insert("pads".to_string(), format!("{:?}", pads));
                onnx_attrs.insert("strides".to_string(), format!("[{}, {}]", stride.0, stride.1));
                onnx_attrs.insert("dilations".to_string(), format!("[{}, {}]", dilation.0, dilation.1));
                onnx_attrs.insert("group".to_string(), groups.to_string());
            }
            NpuOpType::Softmax { axis } => {
                onnx_attrs.insert("axis".to_string(), axis.to_string());
            }
            NpuOpType::ReduceMean { axes, keep_dims } => {
                onnx_attrs.insert("axes".to_string(), format!("{:?}", axes));
                onnx_attrs.insert("keepdims".to_string(), keep_dims.to_string());
            }
            NpuOpType::Concat { axis } => {
                onnx_attrs.insert("axis".to_string(), axis.to_string());
            }
            NpuOpType::Transpose { perm } => {
                onnx_attrs.insert("perm".to_string(), format!("{:?}", perm));
            }
            _ => {}
        }

        // 添加额外属性
        for (key, value) in attrs {
            let value_str = match value {
                super::super::graph::NpuAttribute::Int(v) => v.to_string(),
                super::super::graph::NpuAttribute::Float(v) => v.to_string(),
                super::super::graph::NpuAttribute::String(v) => v.clone(),
                super::super::graph::NpuAttribute::Ints(v) => format!("{:?}", v),
                super::super::graph::NpuAttribute::Floats(v) => format!("{:?}", v),
                _ => continue,
            };
            onnx_attrs.insert(key.clone(), value_str);
        }

        onnx_attrs
    }

    /// 生成 Python 运行时代码
    fn generate_python_runtime(&self, graph: &NpuGraph) -> String {
        let mut code = String::new();

        // 文件头
        code.push_str(&format!(r#"#!/usr/bin/env python3
# Auto-generated by HSCC Intel NPU Backend
# Model: {}
# Target: Intel NPU ({:?})

import numpy as np
from openvino.runtime import Core, Tensor, Type
from typing import Dict, List, Optional

"#, graph.name, self.config.generation));

        // 类定义
        code.push_str(&format!(r#"class {}Executor:
    """Intel NPU Executor (via OpenVINO)"""

    def __init__(self, model_path: str, cache_dir: str = "./cache"):
        self.core = Core()
        self.model = self.core.read_model(model_path)

        # Check NPU availability
        devices = self.core.available_devices
        if "NPU" not in devices:
            print(f"Warning: NPU not available. Available devices: {{devices}}")
            print("Falling back to AUTO device selection")
            target_device = "AUTO"
        else:
            target_device = "NPU"

        # Compile for NPU with optimizations
        self.compiled_model = self.core.compile_model(
            self.model,
            target_device,
            config={{
                "PERFORMANCE_HINT": "{}",
                "CACHE_DIR": cache_dir,
                "NPU_TURBO": "{}",
            }}
        )

        self.infer_request = self.compiled_model.create_infer_request()
        self._actual_device = self.compiled_model.get_property("EXECUTION_DEVICES")[0]

    @property
    def actual_device(self) -> str:
        """Get the actual execution device"""
        return self._actual_device

    def get_input_shapes(self) -> Dict[str, List[int]]:
        """Get input tensor shapes"""
        return {{
            port.get_any_name(): list(port.get_shape())
            for port in self.compiled_model.inputs
        }}

    def get_output_shapes(self) -> Dict[str, List[int]]:
        """Get output tensor shapes"""
        return {{
            port.get_any_name(): list(port.get_shape())
            for port in self.compiled_model.outputs
        }}

    def infer(self, inputs: Dict[str, np.ndarray]) -> Dict[str, np.ndarray]:
        """Execute inference

        Args:
            inputs: Dictionary of input name to numpy array

        Returns:
            Dictionary of output name to numpy array
        """
        # Prepare input tensors
        input_tensors = {{
            name: Tensor(array)
            for name, array in inputs.items()
        }}

        # Set inputs
        self.infer_request.set_input_tensors(input_tensors)

        # Execute inference
        self.infer_request.infer()

        # Get outputs
        outputs = {{}}
        for port in self.compiled_model.outputs:
            name = port.get_any_name()
            outputs[name] = self.infer_request.get_output_tensor(
                port.get_index()
            ).data.copy()

        return outputs

    def benchmark(self, inputs: Dict[str, np.ndarray], num_runs: int = 100) -> Dict[str, float]:
        """Benchmark inference performance

        Returns:
            Dictionary with 'latency_ms', 'throughput_fps', etc.
        """
        import time

        # Warmup
        for _ in range(10):
            self.infer(inputs)

        # Measure
        latencies = []
        for _ in range(num_runs):
            start = time.perf_counter()
            self.infer(inputs)
            end = time.perf_counter()
            latencies.append((end - start) * 1000)  # ms

        import statistics
        return {{
            "latency_ms_mean": statistics.mean(latencies),
            "latency_ms_std": statistics.stdev(latencies) if len(latencies) > 1 else 0,
            "latency_ms_p50": statistics.median(latencies),
            "latency_ms_p99": sorted(latencies)[int(len(latencies) * 0.99)],
            "throughput_fps": 1000.0 / statistics.mean(latencies),
        }}


"#,
            graph.name,
            match self.config.performance_hint {
                PerformanceHint::Latency => "LATENCY",
                PerformanceHint::Throughput => "THROUGHPUT",
                PerformanceHint::PowerEfficient => "CUMULATIVE_THROUGHPUT",
            },
            if self.config.turbo { "YES" } else { "NO" },
        ));

        // 添加输入输出描述
        code.push_str(&format!(r#"def main():
    """Example usage"""
    import argparse

    parser = argparse.ArgumentParser(description="Intel NPU Inference")
    parser.add_argument("--model", required=True, help="Path to ONNX model")
    parser.add_argument("--cache-dir", default="./cache", help="Model cache directory")
    args = parser.parse_args()

    executor = {}Executor(args.model, args.cache_dir)

    print(f"Model loaded successfully")
    print(f"Execution device: {{executor.actual_device}}")
    print(f"Input shapes: {{executor.get_input_shapes()}}")
    print(f"Output shapes: {{executor.get_output_shapes()}}")


if __name__ == "__main__":
    main()
"#, graph.name));

        code
    }

    /// 建议量化
    fn suggest_quantization(&self, graph: &NpuGraph) -> Vec<String> {
        let mut suggestions = Vec::new();

        for tensor in graph.tensors.values() {
            match &tensor.dtype.kind {
                NpuTypeKind::Float { width: 32 } => {
                    suggestions.push(format!(
                        "Tensor '{}' uses FP32. Consider quantizing to INT8 for better NPU performance.",
                        tensor.name
                    ));
                }
                NpuTypeKind::Float { width: 64 } => {
                    suggestions.push(format!(
                        "Tensor '{}' uses FP64 which is not supported. Must convert to FP32 or FP16.",
                        tensor.name
                    ));
                }
                _ => {}
            }
        }

        suggestions
    }
}

impl Default for IntelNpuBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl NpuBackend for IntelNpuBackend {
    fn name(&self) -> &str {
        "intel_npu"
    }

    fn supported_devices(&self) -> Vec<NpuDevice> {
        vec![
            NpuDevice::IntelNPU(IntelNpuGeneration::MeteorLake),
            NpuDevice::IntelNPU(IntelNpuGeneration::LunarLake),
            NpuDevice::IntelNPU(IntelNpuGeneration::ArrowLake),
        ]
    }

    fn hardware_spec(&self, device: NpuDevice) -> NpuHardwareSpec {
        let generation = match device {
            NpuDevice::IntelNPU(g) => g,
            _ => IntelNpuGeneration::MeteorLake,
        };

        let device_spec = self.get_device_spec(generation);

        NpuHardwareSpec {
            device,
            device_name: format!("Intel NPU ({:?})", generation),
            num_cores: device_spec.num_tiles,
            matrix_unit: MatrixUnitSpec {
                // NPU 3720: 512 MPEs per tile, each can do 4 INT8 MACs
                systolic_array: (64, 64, 1),
                supported_combinations: vec![
                    (NpuTypeKind::Integer { width: 8, signed: true },
                     NpuTypeKind::Integer { width: 8, signed: true },
                     NpuTypeKind::Integer { width: 32, signed: true }),
                ],
                peak_tops: device_spec.peak_tops,
            },
            vector_unit: VectorUnitSpec {
                width: 128, // SHAVE DSP 向量宽度
                supported_ops: vec![
                    VectorOp::Add, VectorOp::Sub, VectorOp::Mul, VectorOp::Div,
                    VectorOp::Exp, VectorOp::Log, VectorOp::Sqrt,
                    VectorOp::ReLU, VectorOp::Sigmoid, VectorOp::Tanh,
                    VectorOp::Max, VectorOp::Min,
                ],
            },
            local_memory_kb: device_spec.sram_per_tile_kb * device_spec.num_tiles,
            hbm_size_gb: 0, // 共享系统内存
            memory_bandwidth: 0.0, // 取决于 LPDDR5 配置
            supported_dtypes: vec![
                NpuTypeKind::Integer { width: 8, signed: true },
                NpuTypeKind::Integer { width: 8, signed: false },
                NpuTypeKind::Integer { width: 16, signed: true },
                NpuTypeKind::Integer { width: 32, signed: true },
                NpuTypeKind::Float { width: 16 },
                NpuTypeKind::Float { width: 32 },
            ],
            quant_support: QuantSupport::Int8,
            sparse_support: false,
            preferred_layout: TensorLayout::NCHW,
        }
    }

    fn is_op_supported(&self, op: &NpuOpType, _spec: &NpuHardwareSpec) -> bool {
        self.is_op_supported_by_openvino(op)
    }

    fn estimate_op_latency(&self, op: &NpuOperation, spec: &NpuHardwareSpec) -> Duration {
        // 简单的性能模型
        // 实际应该基于硬件特性和操作复杂度

        let base_ns = match &op.op_type {
            // 矩阵运算 - 主要受 MAC 数量影响
            NpuOpType::MatMul | NpuOpType::BatchMatMul => {
                // 假设 M×N×K 矩阵乘
                // 使用 TOPS 估算
                let mops = 1000.0; // 假设 1000 M ops
                (mops / spec.matrix_unit.peak_tops * 1_000_000.0) as u64
            }

            // 卷积 - 类似矩阵乘
            NpuOpType::Conv2D { .. } => 100_000, // 简化估算

            // 激活函数 - 内存受限
            NpuOpType::ReLU | NpuOpType::Sigmoid | NpuOpType::Tanh => 1_000,

            // 归约
            NpuOpType::ReduceSum { .. } | NpuOpType::ReduceMean { .. } => 5_000,

            // 逐元素
            NpuOpType::Add | NpuOpType::Mul => 500,

            // 其他
            _ => 10_000,
        };

        Duration::from_nanos(base_ns)
    }

    /// 优化计算图
    fn optimize_graph(&self, graph: &mut NpuGraph, _spec: &NpuHardwareSpec) -> Result<(), NpuError> {
        // 1. 检查静态形状
        self.ensure_static_shapes(graph)?;

        // 2. 检查数据类型
        self.ensure_supported_dtypes(graph)?;

        // 3. 计算张量生命周期
        graph.compute_lifetimes();

        // 4. 算子融合（OpenVINO 会自动做，但我们也可以预处理）
        // TODO: 实现融合优化

        // 5. 量化建议
        let suggestions = self.suggest_quantization(graph);
        if !suggestions.is_empty() {
            // 记录建议（实际实现可以写入日志）
            for suggestion in suggestions {
                eprintln!("INFO: {}", suggestion);
            }
        }

        // 6. 验证图
        graph.validate()
            .map_err(|e| NpuError::GraphValidationFailed {
                reason: e.to_string(),
            })?;

        Ok(())
    }

    fn plan_memory(&self, graph: &mut NpuGraph, spec: &NpuHardwareSpec) -> Result<MemoryPlan, NpuError> {
        // 使用通用内存规划器
        let planner = crate::npu::memory::MemoryPlanner::new(spec.clone());
        planner.plan(graph)
    }

    fn generate_code(&self, graph: &NpuGraph, _spec: &NpuHardwareSpec) -> Result<NpuCode, NpuError> {
        // 生成 ONNX 模型
        let onnx_bytes = self.generate_onnx(graph)?;

        // 同时生成 Python 运行时代码
        let python_code = self.generate_python_runtime(graph);

        // 返回 ONNX 模型（主要输出）
        // 用户可以同时获取 Python 运行时代码
        Ok(NpuCode::OnnxModel(onnx_bytes))
    }

    fn generate_runtime_config(&self, graph: &NpuGraph, spec: &NpuHardwareSpec) -> Result<RuntimeConfig, NpuError> {
        let inputs: Vec<TensorDesc> = graph.inputs.iter().map(|i| TensorDesc {
            name: i.name.clone(),
            dtype: i.dtype.clone(),
            shape: i.shape.clone(),
            layout: i.dtype.layout().unwrap_or(TensorLayout::NCHW),
        }).collect();

        let outputs: Vec<TensorDesc> = graph.outputs.iter().map(|o| TensorDesc {
            name: o.name.clone(),
            dtype: o.dtype.clone(),
            shape: o.shape.clone(),
            layout: o.dtype.layout().unwrap_or(TensorLayout::NCHW),
        }).collect();

        Ok(RuntimeConfig {
            inputs,
            outputs,
            execution: ExecutionConfig {
                performance_hint: self.config.performance_hint,
                num_requests: 4,
                turbo: self.config.turbo,
            },
            memory_pool: MemoryPoolConfig {
                pool_size_mb: spec.local_memory_kb / 1024,
                enable_reuse: true,
                defer_weights_load: false,
            },
        })
    }
}

// ============================================================================
// ONNX 模型构建器（简化实现）
// ============================================================================

/// ONNX 模型构建器
///
/// 简化的 ONNX 模型构建器，用于生成 ONNX 格式的模型。
/// 实际实现应该使用 protobuf 库。
struct OnnxModelBuilder {
    name: String,
    inputs: Vec<(String, String, Vec<i64>)>,
    outputs: Vec<(String, String, Vec<i64>)>,
    nodes: Vec<(String, String, Vec<String>, Vec<String>, HashMap<String, String>)>,
}

impl OnnxModelBuilder {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            nodes: Vec::new(),
        }
    }

    fn add_input(&mut self, name: &str, dtype: String, shape: Vec<i64>) {
        self.inputs.push((name.to_string(), dtype, shape));
    }

    fn add_output(&mut self, name: &str, dtype: String, shape: Vec<i64>) {
        self.outputs.push((name.to_string(), dtype, shape));
    }

    fn add_node(
        &mut self,
        op_type: &str,
        name: &str,
        inputs: Vec<String>,
        outputs: Vec<String>,
        attributes: HashMap<String, String>,
    ) {
        self.nodes.push((
            op_type.to_string(),
            name.to_string(),
            inputs,
            outputs,
            attributes,
        ));
    }

    fn build(self) -> Result<Vec<u8>, NpuError> {
        // 简化实现：生成 ONNX 文本格式
        // 实际实现应该使用 protobuf 生成二进制格式
        let mut onnx_text = String::new();

        onnx_text.push_str(&format!(
            "<ir_version: 7, producer: \"HSCC\", model_version: 1>\n"
        ));
        onnx_text.push_str(&format!("<opset_import: [ \"\" : 17 ]>\n\n"));

        // 输入
        for (name, dtype, shape) in &self.inputs {
            let shape_str: Vec<String> = shape.iter().map(|d| d.to_string()).collect();
            onnx_text.push_str(&format!(
                "{} [{}] = {}()\n",
                name,
                shape_str.join(", "),
                dtype
            ));
        }
        onnx_text.push_str("\n");

        // 节点
        for (op_type, name, inputs, outputs, attrs) in &self.nodes {
            let inputs_str = inputs.join(", ");
            let outputs_str = outputs.join(", ");
            let attrs_str: Vec<String> = attrs.iter()
                .map(|(k, v)| format!("{} = {}", k, v))
                .collect();

            onnx_text.push_str(&format!(
                "{} = {}<{}>({})  # {}\n",
                outputs_str,
                op_type,
                attrs_str.join(", "),
                inputs_str,
                name
            ));
        }
        onnx_text.push_str("\n");

        // 输出
        for (name, _, _) in &self.outputs {
            onnx_text.push_str(&format!("return {}\n", name));
        }

        // 返回文本格式（实际应该返回二进制 ONNX）
        Ok(onnx_text.into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intel_npu_backend_creation() {
        let backend = IntelNpuBackend::new();
        assert_eq!(backend.name(), "intel_npu");
    }

    #[test]
    fn test_hardware_spec() {
        let backend = IntelNpuBackend::new();
        let spec = backend.hardware_spec(NpuDevice::IntelNPU(IntelNpuGeneration::MeteorLake));

        assert_eq!(spec.num_cores, 2);
        assert!(spec.matrix_unit.peak_tops > 0.0);
        assert!(matches!(spec.quant_support, QuantSupport::Int8));
    }

    #[test]
    fn test_op_support() {
        let backend = IntelNpuBackend::new();
        let spec = backend.hardware_spec(NpuDevice::IntelNPU(IntelNpuGeneration::MeteorLake));

        assert!(backend.is_op_supported(&NpuOpType::MatMul, &spec));
        assert!(backend.is_op_supported(&NpuOpType::Conv2D {
            padding: crate::npu::graph::Padding::Valid,
            stride: (1, 1),
            dilation: (1, 1),
            groups: 1,
        }, &spec));
        assert!(backend.is_op_supported(&NpuOpType::ReLU, &spec));
    }

    #[test]
    fn test_device_parsing() {
        let device = IntelNpuDevice::meteor_lake();
        assert_eq!(device.generation, IntelNpuGeneration::MeteorLake);
        assert_eq!(device.num_tiles, 2);
        assert_eq!(device.peak_tops, 9.5);
    }
}
