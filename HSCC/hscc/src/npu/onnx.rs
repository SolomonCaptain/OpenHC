//! ONNX 模型生成器
//!
//! 将 NpuGraph 转换为标准的 ONNX 模型格式。
//! ONNX (Open Neural Network Exchange) 是一种开放的模型格式，
//! 支持多种深度学习框架和推理引擎（如 OpenVINO、TensorRT 等）。
//!
//! ## 使用方式
//!
//! ```ignore
//! use hscc::npu::onnx::OnnxBuilder;
//!
//! let graph = /* ... NpuGraph ... */;
//! let builder = OnnxBuilder::new(&graph);
//! let onnx_bytes = builder.build()?;
//! std::fs::write("model.onnx", &onnx_bytes)?;
//! ```
//!
//! ## ONNX 模型结构
//!
//! ```text
//! ModelProto
//! ├── ir_version: i64
//! ├── producer_name: String
//! ├── producer_version: String
//! ├── opset_import: [OperatorSetIdProto]
//! └── graph: GraphProto
//!     ├── name: String
//!     ├── input: [ValueInfoProto]
//!     ├── output: [ValueInfoProto]
//!     ├── initializer: [TensorProto]
//!     └── node: [NodeProto]
//! ```

use std::collections::HashMap;
use super::types::{NpuType, NpuTypeKind, TensorLayout};
use super::graph::{NpuGraph, NpuOperation, NpuOpType, NpuTensor, NpuAttribute, Padding};
use super::backends::NpuError;

// ============================================================================
// ONNX 类型定义（简化版，避免依赖大型 proto 文件）
// ============================================================================

/// ONNX 数据类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnnxDataType {
    Float = 1,
    Uint8 = 2,
    Int8 = 3,
    Uint16 = 4,
    Int16 = 5,
    Int32 = 6,
    Int64 = 7,
    String = 8,
    Bool = 9,
    Float16 = 10,
    Double = 11,
    Uint32 = 12,
    Uint64 = 13,
    Bfloat16 = 16,
}

impl OnnxDataType {
    /// 从 NpuTypeKind 转换
    pub fn from_npu_type(kind: &NpuTypeKind) -> Self {
        match kind {
            NpuTypeKind::Float { width } => match width {
                16 => OnnxDataType::Float16,
                32 => OnnxDataType::Float,
                64 => OnnxDataType::Double,
                _ => OnnxDataType::Float,
            },
            NpuTypeKind::Integer { width, signed } => {
                if *signed {
                    match width {
                        8 => OnnxDataType::Int8,
                        16 => OnnxDataType::Int16,
                        32 => OnnxDataType::Int32,
                        64 => OnnxDataType::Int64,
                        _ => OnnxDataType::Int32,
                    }
                } else {
                    match width {
                        8 => OnnxDataType::Uint8,
                        16 => OnnxDataType::Uint16,
                        32 => OnnxDataType::Uint32,
                        64 => OnnxDataType::Uint64,
                        _ => OnnxDataType::Uint32,
                    }
                }
            }
            NpuTypeKind::Quantized { base, .. } => match base {
                super::types::QuantBase::Int8 => OnnxDataType::Int8,
                super::types::QuantBase::UInt8 => OnnxDataType::Uint8,
                super::types::QuantBase::BF16 => OnnxDataType::Bfloat16,
                _ => OnnxDataType::Float,
            },
            _ => OnnxDataType::Float,
        }
    }
}

/// ONNX 属性类型
#[derive(Debug, Clone)]
pub enum OnnxAttribute {
    Float(f32),
    Int(i64),
    String(String),
    Tensor(OnnxTensor),
    Floats(Vec<f32>),
    Ints(Vec<i64>),
    Strings(Vec<String>),
}

/// ONNX 张量
#[derive(Debug, Clone)]
pub struct OnnxTensor {
    pub name: String,
    pub data_type: OnnxDataType,
    pub dims: Vec<i64>,
    pub float_data: Vec<f32>,
    pub int32_data: Vec<i32>,
    pub int64_data: Vec<i64>,
    pub raw_data: Vec<u8>,
}

impl OnnxTensor {
    pub fn new(name: String, data_type: OnnxDataType, dims: Vec<i64>) -> Self {
        Self {
            name,
            data_type,
            dims,
            float_data: Vec::new(),
            int32_data: Vec::new(),
            int64_data: Vec::new(),
            raw_data: Vec::new(),
        }
    }

    pub fn with_float_data(mut self, data: Vec<f32>) -> Self {
        self.float_data = data;
        self
    }

    pub fn with_int32_data(mut self, data: Vec<i32>) -> Self {
        self.int32_data = data;
        self
    }

    pub fn with_int64_data(mut self, data: Vec<i64>) -> Self {
        self.int64_data = data;
        self
    }

    pub fn with_raw_data(mut self, data: Vec<u8>) -> Self {
        self.raw_data = data;
        self
    }
}

/// ONNX 节点
#[derive(Debug, Clone)]
pub struct OnnxNode {
    pub name: String,
    pub op_type: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub attributes: HashMap<String, OnnxAttribute>,
    pub domain: Option<String>,
}

impl OnnxNode {
    pub fn new(name: String, op_type: String) -> Self {
        Self {
            name,
            op_type,
            inputs: Vec::new(),
            outputs: Vec::new(),
            attributes: HashMap::new(),
            domain: None,
        }
    }

    pub fn with_inputs(mut self, inputs: Vec<String>) -> Self {
        self.inputs = inputs;
        self
    }

    pub fn with_outputs(mut self, outputs: Vec<String>) -> Self {
        self.outputs = outputs;
        self
    }

    pub fn with_attribute(mut self, name: String, attr: OnnxAttribute) -> Self {
        self.attributes.insert(name, attr);
        self
    }

    pub fn with_domain(mut self, domain: String) -> Self {
        self.domain = Some(domain);
        self
    }
}

/// ONNX 值信息（输入/输出描述）
#[derive(Debug, Clone)]
pub struct OnnxValueInfo {
    pub name: String,
    pub data_type: OnnxDataType,
    pub shape: Vec<i64>,
}

impl OnnxValueInfo {
    pub fn new(name: String, data_type: OnnxDataType, shape: Vec<i64>) -> Self {
        Self { name, data_type, shape }
    }
}

/// ONNX 图
#[derive(Debug, Clone)]
pub struct OnnxGraph {
    pub name: String,
    pub inputs: Vec<OnnxValueInfo>,
    pub outputs: Vec<OnnxValueInfo>,
    pub nodes: Vec<OnnxNode>,
    pub initializers: Vec<OnnxTensor>,
}

impl OnnxGraph {
    pub fn new(name: String) -> Self {
        Self {
            name,
            inputs: Vec::new(),
            outputs: Vec::new(),
            nodes: Vec::new(),
            initializers: Vec::new(),
        }
    }

    pub fn add_input(&mut self, input: OnnxValueInfo) {
        self.inputs.push(input);
    }

    pub fn add_output(&mut self, output: OnnxValueInfo) {
        self.outputs.push(output);
    }

    pub fn add_node(&mut self, node: OnnxNode) {
        self.nodes.push(node);
    }

    pub fn add_initializer(&mut self, tensor: OnnxTensor) {
        self.initializers.push(tensor);
    }
}

/// ONNX 模型
#[derive(Debug, Clone)]
pub struct OnnxModel {
    pub ir_version: i64,
    pub producer_name: String,
    pub producer_version: String,
    pub model_version: i64,
    pub opset_version: i64,
    pub graph: OnnxGraph,
}

impl OnnxModel {
    pub fn new(graph: OnnxGraph) -> Self {
        Self {
            ir_version: 9,
            producer_name: "HSCC".to_string(),
            producer_version: env!("CARGO_PKG_VERSION").to_string(),
            model_version: 1,
            opset_version: 17,
            graph,
        }
    }

    pub fn with_opset_version(mut self, version: i64) -> Self {
        self.opset_version = version;
        self
    }

    /// 序列化为 ONNX 二进制格式（protobuf）
    pub fn to_bytes(&self) -> Result<Vec<u8>, NpuError> {
        // 使用 protobuf 序列化
        let mut encoder = ProtobufEncoder::new();
        self.encode(&mut encoder)?;
        Ok(encoder.into_bytes())
    }

    /// 序列化为 ONNX 文本格式（用于调试）
    pub fn to_text(&self) -> String {
        let mut text = String::new();

        text.push_str(&format!(
            "<ir_version: {}, producer: \"{}\", version: \"{}\">\n",
            self.ir_version, self.producer_name, self.producer_version
        ));
        text.push_str(&format!("<opset_import: [ \"\" : {} ]>\n\n", self.opset_version));

        // 图定义
        text.push_str(&format!("{} (\n", self.graph.name));

        // 输入
        for input in &self.graph.inputs {
            let shape_str: Vec<String> = input.shape.iter()
                .map(|d| if *d < 0 { "dim".to_string() } else { d.to_string() })
                .collect();
            text.push_str(&format!("  {} [{}]\n", input.name, shape_str.join(", ")));
        }
        text.push_str(") -> (\n");

        // 输出
        for output in &self.graph.outputs {
            let shape_str: Vec<String> = output.shape.iter()
                .map(|d| if *d < 0 { "dim".to_string() } else { d.to_string() })
                .collect();
            text.push_str(&format!("  {} [{}]\n", output.name, shape_str.join(", ")));
        }
        text.push_str(") {\n");

        // 节点
        for node in &self.graph.nodes {
            let inputs_str = node.inputs.join(", ");
            let outputs_str = node.outputs.join(", ");

            let attrs_str = if node.attributes.is_empty() {
                String::new()
            } else {
                let attrs: Vec<String> = node.attributes.iter()
                    .map(|(k, v)| format!("{}={}", k, format_attribute_value(v)))
                    .collect();
                format!("<{}>", attrs.join(", "))
            };

            text.push_str(&format!(
                "  {} = {}{}({})\n",
                outputs_str, node.op_type, attrs_str, inputs_str
            ));
        }

        text.push_str("}\n");
        text
    }

    fn encode(&self, encoder: &mut ProtobufEncoder) -> Result<(), NpuError> {
        // ModelProto 结构
        // 1: ir_version (int64)
        encoder.encode_varint(1, self.ir_version as u64);

        // 2: producer_name (string)
        encoder.encode_string(2, &self.producer_name);

        // 3: producer_version (string)
        encoder.encode_string(3, &self.producer_version);

        // 5: model_version (int64)
        encoder.encode_varint(5, self.model_version as u64);

        // 8: opset_import (repeated OperatorSetIdProto)
        encoder.encode_varint(8 << 3 | 2, 2); // length-delimited
        let opset_bytes = {
            let mut opset_enc = ProtobufEncoder::new();
            opset_enc.encode_string(1, ""); // domain
            opset_enc.encode_varint(2, self.opset_version as u64); // version
            opset_enc.into_bytes()
        };
        encoder.encode_bytes_raw(&opset_bytes);

        // 7: graph (GraphProto)
        self.encode_graph(encoder)?;

        Ok(())
    }

    fn encode_graph(&self, encoder: &mut ProtobufEncoder) -> Result<(), NpuError> {
        let graph_bytes = {
            let mut graph_enc = ProtobufEncoder::new();

            // 1: name (string)
            graph_enc.encode_string(1, &self.graph.name);

            // 2: input (repeated ValueInfoProto)
            for input in &self.graph.inputs {
                graph_enc.encode_varint(2 << 3 | 2, 0); // length-delimited tag
                let input_bytes = encode_value_info(input);
                graph_enc.encode_bytes_raw(&input_bytes);
            }

            // 3: output (repeated ValueInfoProto)
            for output in &self.graph.outputs {
                graph_enc.encode_varint(3 << 3 | 2, 0); // length-delimited tag
                let output_bytes = encode_value_info(output);
                graph_enc.encode_bytes_raw(&output_bytes);
            }

            // 4: initializer (repeated TensorProto)
            for init in &self.graph.initializers {
                graph_enc.encode_varint(4 << 3 | 2, 0);
                let init_bytes = encode_tensor(init);
                graph_enc.encode_bytes_raw(&init_bytes);
            }

            // 5: node (repeated NodeProto)
            for node in &self.graph.nodes {
                graph_enc.encode_varint(5 << 3 | 2, 0);
                let node_bytes = encode_node(node);
                graph_enc.encode_bytes_raw(&node_bytes);
            }

            graph_enc.into_bytes()
        };

        encoder.encode_varint(7 << 3 | 2, graph_bytes.len() as u64);
        encoder.encode_bytes_raw(&graph_bytes);

        Ok(())
    }
}

fn format_attribute_value(attr: &OnnxAttribute) -> String {
    match attr {
        OnnxAttribute::Float(f) => format!("{}", f),
        OnnxAttribute::Int(i) => format!("{}", i),
        OnnxAttribute::String(s) => format!("\"{}\"", s),
        OnnxAttribute::Tensor(t) => format!("Tensor({})", t.name),
        OnnxAttribute::Floats(v) => format!("{:?}", v),
        OnnxAttribute::Ints(v) => format!("{:?}", v),
        OnnxAttribute::Strings(v) => format!("{:?}", v),
    }
}

fn encode_value_info(info: &OnnxValueInfo) -> Vec<u8> {
    let mut enc = ProtobufEncoder::new();

    // 1: name (string)
    enc.encode_string(1, &info.name);

    // 2: type (TypeProto)
    enc.encode_varint(2 << 3 | 2, 0); // length-delimited
    let type_bytes = {
        let mut type_enc = ProtobufEncoder::new();

        // 1: tensor_type (TypeProto::Tensor)
        type_enc.encode_varint(1 << 3 | 2, 0); // length-delimited
        let tensor_type_bytes = {
            let mut tt_enc = ProtobufEncoder::new();

            // 1: elem_type (int32)
            tt_enc.encode_varint(1, info.data_type as u64);

            // 2: shape (TensorShapeProto)
            tt_enc.encode_varint(2 << 3 | 2, 0); // length-delimited
            let shape_bytes = {
                let mut shape_enc = ProtobufEncoder::new();

                // 1: dim (repeated TensorShapeProto::Dimension)
                for dim in &info.shape {
                    shape_enc.encode_varint(1 << 3 | 2, 0); // length-delimited
                    let dim_bytes = {
                        let mut dim_enc = ProtobufEncoder::new();
                        if *dim >= 0 {
                            dim_enc.encode_varint(1, *dim as u64); // dim_value
                        } else {
                            dim_enc.encode_string(2, "dim"); // dim_param
                        }
                        dim_enc.into_bytes()
                    };
                    shape_enc.encode_bytes_raw(&dim_bytes);
                }

                shape_enc.into_bytes()
            };
            tt_enc.encode_bytes_raw(&shape_bytes);

            tt_enc.into_bytes()
        };
        type_enc.encode_bytes_raw(&tensor_type_bytes);

        type_enc.into_bytes()
    };
    enc.encode_bytes_raw(&type_bytes);

    enc.into_bytes()
}

fn encode_tensor(tensor: &OnnxTensor) -> Vec<u8> {
    let mut enc = ProtobufEncoder::new();

    // 1: name (string)
    enc.encode_string(1, &tensor.name);

    // 2: data_type (int32)
    enc.encode_varint(2, tensor.data_type as u64);

    // 3: dims (repeated int64)
    for dim in &tensor.dims {
        enc.encode_varint(3 << 3 | 0, *dim as u64);
    }

    // 4: float_data (repeated float) - 标签 4, wire type 5 (32-bit)
    for &f in &tensor.float_data {
        enc.encode_float(4, f);
    }

    // 5: int32_data (repeated int32)
    for &i in &tensor.int32_data {
        enc.encode_varint(5 << 3 | 5, i as u64); // 32-bit
    }

    // 7: int64_data (repeated int64)
    for &i in &tensor.int64_data {
        enc.encode_varint(7, i as u64);
    }

    // 9: raw_data (bytes)
    if !tensor.raw_data.is_empty() {
        enc.encode_bytes(9, &tensor.raw_data);
    }

    enc.into_bytes()
}

fn encode_node(node: &OnnxNode) -> Vec<u8> {
    let mut enc = ProtobufEncoder::new();

    // 1: input (repeated string)
    for input in &node.inputs {
        enc.encode_string(1, input);
    }

    // 2: output (repeated string)
    for output in &node.outputs {
        enc.encode_string(2, output);
    }

    // 3: name (string)
    enc.encode_string(3, &node.name);

    // 4: op_type (string)
    enc.encode_string(4, &node.op_type);

    // 5: attribute (repeated AttributeProto)
    for (name, attr) in &node.attributes {
        enc.encode_varint(5 << 3 | 2, 0); // length-delimited
        let attr_bytes = encode_attribute(name, attr);
        enc.encode_bytes_raw(&attr_bytes);
    }

    // 6: domain (string)
    if let Some(domain) = &node.domain {
        enc.encode_string(6, domain);
    }

    enc.into_bytes()
}

fn encode_attribute(name: &str, attr: &OnnxAttribute) -> Vec<u8> {
    let mut enc = ProtobufEncoder::new();

    // 1: name (string)
    enc.encode_string(1, name);

    match attr {
        OnnxAttribute::Float(f) => {
            // 3: f (float)
            enc.encode_float(3, *f);
            // 13: type (AttributeType::FLOAT = 2)
            enc.encode_varint(13, 2);
        }
        OnnxAttribute::Int(i) => {
            // 2: i (int64)
            enc.encode_varint(2, *i as u64);
            // 13: type (AttributeType::INT = 1)
            enc.encode_varint(13, 1);
        }
        OnnxAttribute::String(s) => {
            // 4: s (bytes)
            enc.encode_bytes(4, s.as_bytes());
            // 13: type (AttributeType::STRING = 3)
            enc.encode_varint(13, 3);
        }
        OnnxAttribute::Tensor(t) => {
            // 5: t (TensorProto)
            enc.encode_varint(5 << 3 | 2, 0);
            let t_bytes = encode_tensor(t);
            enc.encode_bytes_raw(&t_bytes);
            // 13: type (AttributeType::TENSOR = 4)
            enc.encode_varint(13, 4);
        }
        OnnxAttribute::Floats(v) => {
            // 6: floats (repeated float)
            for &f in v {
                enc.encode_float(6, f);
            }
            // 13: type (AttributeType::FLOATS = 7)
            enc.encode_varint(13, 7);
        }
        OnnxAttribute::Ints(v) => {
            // 7: ints (repeated int64)
            for &i in v {
                enc.encode_varint(7, i as u64);
            }
            // 13: type (AttributeType::INTS = 6)
            enc.encode_varint(13, 6);
        }
        OnnxAttribute::Strings(v) => {
            // 8: strings (repeated bytes)
            for s in v {
                enc.encode_bytes(8, s.as_bytes());
            }
            // 13: type (AttributeType::STRINGS = 8)
            enc.encode_varint(13, 8);
        }
    }

    enc.into_bytes()
}

// ============================================================================
// Protobuf 编码器（简化实现）
// ============================================================================

/// 简化的 Protobuf 编码器
struct ProtobufEncoder {
    bytes: Vec<u8>,
}

impl ProtobufEncoder {
    fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    /// 编码 varint
    fn encode_varint(&mut self, field_number: u32, value: u64) {
        // field_number << 3 | wire_type
        let tag = (field_number << 3) | 0; // wire type 0 = varint
        self.encode_varint_raw(tag as u64);
        self.encode_varint_raw(value);
    }

    fn encode_varint_raw(&mut self, value: u64) {
        let mut v = value;
        loop {
            let mut byte = (v & 0x7F) as u8;
            v >>= 7;
            if v != 0 {
                byte |= 0x80;
            }
            self.bytes.push(byte);
            if v == 0 {
                break;
            }
        }
    }

    /// 编码 string
    fn encode_string(&mut self, field_number: u32, value: &str) {
        let tag = (field_number << 3) | 2; // wire type 2 = length-delimited
        self.encode_varint_raw(tag as u64);
        self.encode_varint_raw(value.len() as u64);
        self.bytes.extend_from_slice(value.as_bytes());
    }

    /// 编码 bytes
    fn encode_bytes(&mut self, field_number: u32, value: &[u8]) {
        let tag = (field_number << 3) | 2;
        self.encode_varint_raw(tag as u64);
        self.encode_varint_raw(value.len() as u64);
        self.bytes.extend_from_slice(value);
    }

    /// 编码 bytes (原始，不带 tag)
    fn encode_bytes_raw(&mut self, value: &[u8]) {
        self.bytes.extend_from_slice(value);
    }

    /// 编码 float (32-bit)
    fn encode_float(&mut self, field_number: u32, value: f32) {
        let tag = (field_number << 3) | 5; // wire type 5 = 32-bit
        self.encode_varint_raw(tag as u64);
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

// ============================================================================
// NpuGraph 到 ONNX 的转换
// ============================================================================

/// ONNX 模型构建器
pub struct OnnxBuilder<'a> {
    graph: &'a NpuGraph,
    opset_version: i64,
}

impl<'a> OnnxBuilder<'a> {
    /// 创建新的 ONNX 构建器
    pub fn new(graph: &'a NpuGraph) -> Self {
        Self {
            graph,
            opset_version: 17,
        }
    }

    /// 设置 opset 版本
    pub fn with_opset_version(mut self, version: i64) -> Self {
        self.opset_version = version;
        self
    }

    /// 构建 ONNX 模型
    pub fn build(&self) -> Result<OnnxModel, NpuError> {
        let mut onnx_graph = OnnxGraph::new(self.graph.name.clone());

        // 转换输入
        for input in &self.graph.inputs {
            let data_type = OnnxDataType::from_npu_type(&input.dtype.kind);
            onnx_graph.add_input(OnnxValueInfo::new(
                input.name.clone(),
                data_type,
                input.shape.clone(),
            ));
        }

        // 转换输出
        for output in &self.graph.outputs {
            let data_type = OnnxDataType::from_npu_type(&output.dtype.kind);
            onnx_graph.add_output(OnnxValueInfo::new(
                output.name.clone(),
                data_type,
                output.shape.clone(),
            ));
        }

        // 转换操作
        for op in &self.graph.operations {
            let node = self.convert_operation(op)?;
            onnx_graph.add_node(node);
        }

        // 创建模型
        let model = OnnxModel::new(onnx_graph)
            .with_opset_version(self.opset_version);

        Ok(model)
    }

    /// 转换 NpuOperation 到 OnnxNode
    fn convert_operation(&self, op: &NpuOperation) -> Result<OnnxNode, NpuError> {
        let op_type = self.get_onnx_op_type(&op.op_type);
        let mut node = OnnxNode::new(op.name.clone(), op_type);

        // 设置输入输出
        node = node.with_inputs(op.inputs.clone());
        node = node.with_outputs(op.outputs.clone());

        // 转换属性
        node = self.add_operation_attributes(node, &op.op_type, &op.attributes)?;

        Ok(node)
    }

    /// 获取 ONNX 操作类型名称
    fn get_onnx_op_type(&self, op_type: &NpuOpType) -> String {
        match op_type {
            NpuOpType::MatMul => "MatMul".to_string(),
            NpuOpType::BatchMatMul => "MatMul".to_string(),
            NpuOpType::Transpose { .. } => "Transpose".to_string(),
            NpuOpType::Conv2D { .. } => "Conv".to_string(),
            NpuOpType::DepthwiseConv2D { .. } => "Conv".to_string(),
            NpuOpType::ConvTranspose2D { .. } => "ConvTranspose".to_string(),
            NpuOpType::ReLU => "Relu".to_string(),
            NpuOpType::ReLU6 => "Clip".to_string(), // ReLU6 = Clip(0, 6)
            NpuOpType::LeakyReLU { .. } => "LeakyRelu".to_string(),
            NpuOpType::Sigmoid => "Sigmoid".to_string(),
            NpuOpType::Tanh => "Tanh".to_string(),
            NpuOpType::GELU => "Gelu".to_string(),
            NpuOpType::Swish | NpuOpType::SiLU => "Silu".to_string(),
            NpuOpType::HardSwish => "HardSwish".to_string(),
            NpuOpType::Softmax { .. } => "Softmax".to_string(),
            NpuOpType::LogSoftmax { .. } => "LogSoftmax".to_string(),
            NpuOpType::BatchNorm { .. } => "BatchNormalization".to_string(),
            NpuOpType::LayerNorm { .. } => "LayerNormalization".to_string(),
            NpuOpType::InstanceNorm { .. } => "InstanceNormalization".to_string(),
            NpuOpType::GroupNorm { .. } => "GroupNormalization".to_string(),
            NpuOpType::MaxPool2D { .. } => "MaxPool".to_string(),
            NpuOpType::AvgPool2D { .. } => "AveragePool".to_string(),
            NpuOpType::GlobalAvgPool => "GlobalAveragePool".to_string(),
            NpuOpType::GlobalMaxPool => "GlobalMaxPool".to_string(),
            NpuOpType::AdaptiveAvgPool { .. } => "AdaptiveAveragePool".to_string(),
            NpuOpType::AdaptiveMaxPool { .. } => "AdaptiveMaxPool".to_string(),
            NpuOpType::Add => "Add".to_string(),
            NpuOpType::Sub => "Sub".to_string(),
            NpuOpType::Mul => "Mul".to_string(),
            NpuOpType::Div => "Div".to_string(),
            NpuOpType::Exp => "Exp".to_string(),
            NpuOpType::Log => "Log".to_string(),
            NpuOpType::Sqrt => "Sqrt".to_string(),
            NpuOpType::Pow => "Pow".to_string(),
            NpuOpType::Neg => "Neg".to_string(),
            NpuOpType::Abs => "Abs".to_string(),
            NpuOpType::Min => "Min".to_string(),
            NpuOpType::Max => "Max".to_string(),
            NpuOpType::Clip { .. } => "Clip".to_string(),
            NpuOpType::Sin => "Sin".to_string(),
            NpuOpType::Cos => "Cos".to_string(),
            NpuOpType::Tan => "Tan".to_string(),
            NpuOpType::ReduceSum { .. } => "ReduceSum".to_string(),
            NpuOpType::ReduceMean { .. } => "ReduceMean".to_string(),
            NpuOpType::ReduceMax { .. } => "ReduceMax".to_string(),
            NpuOpType::ReduceMin { .. } => "ReduceMin".to_string(),
            NpuOpType::ReduceProd { .. } => "ReduceProd".to_string(),
            NpuOpType::ReduceL2 { .. } => "ReduceL2".to_string(),
            NpuOpType::Reshape => "Reshape".to_string(),
            NpuOpType::Flatten { .. } => "Flatten".to_string(),
            NpuOpType::Squeeze { .. } => "Squeeze".to_string(),
            NpuOpType::Unsqueeze { .. } => "Unsqueeze".to_string(),
            NpuOpType::Expand => "Expand".to_string(),
            NpuOpType::Concat { .. } => "Concat".to_string(),
            NpuOpType::Split { .. } => "Split".to_string(),
            NpuOpType::Slice { .. } => "Slice".to_string(),
            NpuOpType::Tile => "Tile".to_string(),
            NpuOpType::Gather { .. } => "Gather".to_string(),
            NpuOpType::ScatterND => "ScatterND".to_string(),
            NpuOpType::NonZero => "NonZero".to_string(),
            NpuOpType::TopK { .. } => "TopK".to_string(),
            NpuOpType::FlashAttention { .. } => "FlashAttention".to_string(),
            NpuOpType::MultiHeadAttention { .. } => "MultiHeadAttention".to_string(),
            NpuOpType::ScaledDotProductAttention { .. } => "ScaledDotProductAttention".to_string(),
            NpuOpType::Quantize { .. } => "QuantizeLinear".to_string(),
            NpuOpType::Dequantize { .. } => "DequantizeLinear".to_string(),
            NpuOpType::Requantize { .. } => "Requantize".to_string(),
            NpuOpType::If { .. } => "If".to_string(),
            NpuOpType::Loop { .. } => "Loop".to_string(),
            NpuOpType::LSTM { .. } => "LSTM".to_string(),
            NpuOpType::GRU { .. } => "GRU".to_string(),
            NpuOpType::Dropout { .. } => "Dropout".to_string(),
            NpuOpType::Identity => "Identity".to_string(),
            NpuOpType::Cast => "Cast".to_string(),
            NpuOpType::Constant { .. } => "Constant".to_string(),
            NpuOpType::Custom { op_name } => op_name.clone(),
            _ => "Unknown".to_string(),
        }
    }

    /// 添加操作属性
    fn add_operation_attributes(
        &self,
        mut node: OnnxNode,
        op_type: &NpuOpType,
        attrs: &HashMap<String, NpuAttribute>,
    ) -> Result<OnnxNode, NpuError> {
        match op_type {
            NpuOpType::Conv2D { padding, stride, dilation, groups } => {
                let pads = match padding {
                    Padding::Valid => vec![0i64, 0, 0, 0],
                    Padding::Same => vec![], // 使用 auto_pad
                    Padding::Explicit(t, b, l, r) => vec![*t as i64, *b as i64, *l as i64, *r as i64],
                };
                if !pads.is_empty() {
                    node = node.with_attribute("pads".to_string(), OnnxAttribute::Ints(pads));
                } else {
                    node = node.with_attribute("auto_pad".to_string(), OnnxAttribute::String("SAME_UPPER".to_string()));
                }
                node = node.with_attribute("strides".to_string(), OnnxAttribute::Ints(vec![stride.0 as i64, stride.1 as i64]));
                node = node.with_attribute("dilations".to_string(), OnnxAttribute::Ints(vec![dilation.0 as i64, dilation.1 as i64]));
                node = node.with_attribute("group".to_string(), OnnxAttribute::Int(*groups as i64));
            }
            NpuOpType::MaxPool2D { kernel, stride, padding } |
            NpuOpType::AvgPool2D { kernel, stride, padding } => {
                node = node.with_attribute("kernel_shape".to_string(), OnnxAttribute::Ints(vec![kernel.0 as i64, kernel.1 as i64]));
                node = node.with_attribute("strides".to_string(), OnnxAttribute::Ints(vec![stride.0 as i64, stride.1 as i64]));
                let pads = match padding {
                    Padding::Valid => vec![0i64, 0, 0, 0],
                    Padding::Same => vec![],
                    Padding::Explicit(t, b, l, r) => vec![*t as i64, *b as i64, *l as i64, *r as i64],
                };
                if !pads.is_empty() {
                    node = node.with_attribute("pads".to_string(), OnnxAttribute::Ints(pads));
                }
            }
            NpuOpType::Softmax { axis } |
            NpuOpType::LogSoftmax { axis } => {
                node = node.with_attribute("axis".to_string(), OnnxAttribute::Int(*axis as i64));
            }
            NpuOpType::LeakyReLU { alpha } => {
                node = node.with_attribute("alpha".to_string(), OnnxAttribute::Float(*alpha));
            }
            NpuOpType::BatchNorm { epsilon, .. } |
            NpuOpType::LayerNorm { epsilon, .. } |
            NpuOpType::InstanceNorm { epsilon } => {
                node = node.with_attribute("epsilon".to_string(), OnnxAttribute::Float(*epsilon));
            }
            NpuOpType::GroupNorm { epsilon, num_groups } => {
                node = node.with_attribute("epsilon".to_string(), OnnxAttribute::Float(*epsilon));
                node = node.with_attribute("num_groups".to_string(), OnnxAttribute::Int(*num_groups as i64));
            }
            NpuOpType::ReduceSum { axes, keep_dims } |
            NpuOpType::ReduceMean { axes, keep_dims } |
            NpuOpType::ReduceMax { axes, keep_dims } |
            NpuOpType::ReduceMin { axes, keep_dims } |
            NpuOpType::ReduceProd { axes, keep_dims } |
            NpuOpType::ReduceL2 { axes, keep_dims } => {
                node = node.with_attribute("axes".to_string(), OnnxAttribute::Ints(axes.iter().map(|a| *a as i64).collect()));
                node = node.with_attribute("keepdims".to_string(), OnnxAttribute::Int(if *keep_dims { 1 } else { 0 }));
            }
            NpuOpType::Concat { axis } => {
                node = node.with_attribute("axis".to_string(), OnnxAttribute::Int(*axis as i64));
            }
            NpuOpType::Transpose { perm } => {
                node = node.with_attribute("perm".to_string(), OnnxAttribute::Ints(perm.iter().map(|p| *p as i64).collect()));
            }
            NpuOpType::Flatten { axis } => {
                node = node.with_attribute("axis".to_string(), OnnxAttribute::Int(*axis as i64));
            }
            NpuOpType::Squeeze { axes } |
            NpuOpType::Unsqueeze { axes } => {
                node = node.with_attribute("axes".to_string(), OnnxAttribute::Ints(axes.iter().map(|a| *a as i64).collect()));
            }
            NpuOpType::Clip { min, max } => {
                node = node.with_attribute("min".to_string(), OnnxAttribute::Float(*min));
                node = node.with_attribute("max".to_string(), OnnxAttribute::Float(*max));
            }
            NpuOpType::Dropout { ratio } => {
                node = node.with_attribute("ratio".to_string(), OnnxAttribute::Float(*ratio));
            }
            NpuOpType::TopK { k, axis } => {
                node = node.with_attribute("k".to_string(), OnnxAttribute::Int(*k));
                node = node.with_attribute("axis".to_string(), OnnxAttribute::Int(*axis as i64));
            }
            NpuOpType::Gather { axis } => {
                node = node.with_attribute("axis".to_string(), OnnxAttribute::Int(*axis as i64));
            }
            _ => {}
        }

        // 添加额外属性
        for (key, value) in attrs {
            let onnx_attr = match value {
                NpuAttribute::Int(v) => OnnxAttribute::Int(*v),
                NpuAttribute::Float(v) => OnnxAttribute::Float(*v),
                NpuAttribute::String(v) => OnnxAttribute::String(v.clone()),
                NpuAttribute::Ints(v) => OnnxAttribute::Ints(v.clone()),
                NpuAttribute::Floats(v) => OnnxAttribute::Floats(v.clone()),
                _ => continue,
            };
            node = node.with_attribute(key.clone(), onnx_attr);
        }

        Ok(node)
    }

    /// 构建并序列化为 ONNX 二进制
    pub fn build_to_bytes(&self) -> Result<Vec<u8>, NpuError> {
        let model = self.build()?;
        model.to_bytes()
    }

    /// 构建并序列化为 ONNX 文本（调试用）
    pub fn build_to_text(&self) -> Result<String, NpuError> {
        let model = self.build()?;
        Ok(model.to_text())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_onnx_data_type_conversion() {
        assert_eq!(OnnxDataType::from_npu_type(&NpuTypeKind::Float { width: 32 }), OnnxDataType::Float);
        assert_eq!(OnnxDataType::from_npu_type(&NpuTypeKind::Float { width: 16 }), OnnxDataType::Float16);
        assert_eq!(OnnxDataType::from_npu_type(&NpuTypeKind::Integer { width: 32, signed: true }), OnnxDataType::Int32);
        assert_eq!(OnnxDataType::from_npu_type(&NpuTypeKind::Integer { width: 8, signed: true }), OnnxDataType::Int8);
    }

    #[test]
    fn test_onnx_node_creation() {
        let node = OnnxNode::new("matmul_0".to_string(), "MatMul".to_string())
            .with_inputs(vec!["A".to_string(), "B".to_string()])
            .with_outputs(vec!["C".to_string()]);

        assert_eq!(node.name, "matmul_0");
        assert_eq!(node.op_type, "MatMul");
        assert_eq!(node.inputs.len(), 2);
        assert_eq!(node.outputs.len(), 1);
    }

    #[test]
    fn test_protobuf_encoder_varint() {
        let mut enc = ProtobufEncoder::new();
        enc.encode_varint_raw(1);
        assert_eq!(enc.bytes, vec![1]);

        let mut enc = ProtobufEncoder::new();
        enc.encode_varint_raw(300);
        assert_eq!(enc.bytes, vec![0xAC, 0x02]);
    }

    #[test]
    fn test_onnx_builder_empty_graph() {
        let graph = NpuGraph::new("test");
        let builder = OnnxBuilder::new(&graph);
        let model = builder.build().unwrap();

        assert_eq!(model.graph.name, "test");
        assert!(model.graph.inputs.is_empty());
        assert!(model.graph.outputs.is_empty());
        assert!(model.graph.nodes.is_empty());
    }

    #[test]
    fn test_onnx_model_to_text() {
        let mut graph = NpuGraph::new("simple_matmul");
        graph.add_input("A", NpuType::f32(), vec![2, 3]);
        graph.add_input("B", NpuType::f32(), vec![3, 4]);
        graph.add_output("C", NpuType::f32(), vec![2, 4]);

        let builder = OnnxBuilder::new(&graph);
        let model = builder.build().unwrap();
        let text = model.to_text();

        assert!(text.contains("simple_matmul"));
        assert!(text.contains("A"));
        assert!(text.contains("B"));
        assert!(text.contains("C"));
    }
}
