//! NPU 代码生成
//!
//! 将 NPU 计算图生成为目标格式（ONNX、Python 等）。

use std::collections::HashMap;
use crate::npu::graph::{NpuGraph, NpuOperation, NpuOpType};
use crate::npu::types::NpuType;
use crate::npu::backends::NpuError;

/// NPU 代码生成器
pub struct NpuCodeGenerator {
    /// 生成的代码
    output: String,
    /// 缩进级别
    indent: usize,
}

impl NpuCodeGenerator {
    /// 创建代码生成器
    pub fn new() -> Self {
        Self {
            output: String::new(),
            indent: 0,
        }
    }

    /// 生成 ONNX 模型
    pub fn generate_onnx(&mut self, graph: &NpuGraph) -> Result<Vec<u8>, NpuError> {
        // ONNX 文本格式（用于调试）
        let mut onnx_text = String::new();

        // 头部
        onnx_text.push_str(&format!(
            "<ir_version: 7, producer: \"HSCC\">\n"
        ));
        onnx_text.push_str(&format!("<opset_import: [ \"\" : 17 ]>\n\n"));

        // 输入
        onnx_text.push_str("# Inputs\n");
        for input in &graph.inputs {
            onnx_text.push_str(&format!(
                "{} : {}\n",
                input.name,
                input.dtype.to_onnx_type()
            ));
        }
        onnx_text.push_str("\n");

        // 节点
        onnx_text.push_str("# Operations\n");
        for op in &graph.operations {
            let node_str = self.format_onnx_node(op);
            onnx_text.push_str(&node_str);
            onnx_text.push('\n');
        }
        onnx_text.push_str("\n");

        // 输出
        onnx_text.push_str("# Outputs\n");
        for output in &graph.outputs {
            onnx_text.push_str(&format!(
                "{} : {}\n",
                output.name,
                output.dtype.to_onnx_type()
            ));
        }

        Ok(onnx_text.into_bytes())
    }

    /// 格式化 ONNX 节点
    fn format_onnx_node(&self, op: &NpuOperation) -> String {
        let inputs = op.inputs.join(", ");
        let outputs = op.outputs.join(", ");
        let attrs = self.format_attributes(&op.op_type);

        format!(
            "{} = {}<{}>({})  # {}",
            outputs,
            op.op_type.name(),
            attrs,
            inputs,
            op.name
        )
    }

    /// 格式化属性
    fn format_attributes(&self, op_type: &NpuOpType) -> String {
        match op_type {
            NpuOpType::Conv2D { padding, stride, dilation, groups } => {
                format!(
                    "pads=[{:?}], strides=[{}, {}], dilations=[{}, {}], group={}",
                    padding,
                    stride.0, stride.1,
                    dilation.0, dilation.1,
                    groups
                )
            }
            NpuOpType::Softmax { axis } => {
                format!("axis={}", axis)
            }
            NpuOpType::ReduceMean { axes, keep_dims } => {
                format!("axes={:?}, keepdims={}", axes, keep_dims)
            }
            NpuOpType::Concat { axis } => {
                format!("axis={}", axis)
            }
            NpuOpType::Transpose { perm } => {
                format!("perm={:?}", perm)
            }
            _ => String::new(),
        }
    }

    /// 生成 JSON 图定义
    pub fn generate_json(&mut self, graph: &NpuGraph) -> Result<String, NpuError> {
        let mut json = String::new();
        json.push('{');
        json.push_str(&format!("\"name\": \"{}\", ", graph.name));

        // 输入
        json.push_str("\"inputs\": [");
        for (i, input) in graph.inputs.iter().enumerate() {
            if i > 0 { json.push_str(", "); }
            let entry = format!(
                r#"{{"name": "{}", "shape": {:?}, "dtype": "{}"}}"#,
                input.name,
                input.shape,
                input.dtype.to_onnx_type()
            );
            json.push_str(&entry);
        }
        json.push_str("], ");

        // 输出
        json.push_str("\"outputs\": [");
        for (i, output) in graph.outputs.iter().enumerate() {
            if i > 0 { json.push_str(", "); }
            let entry = format!(
                r#"{{"name": "{}", "shape": {:?}, "dtype": "{}"}}"#,
                output.name,
                output.shape,
                output.dtype.to_onnx_type()
            );
            json.push_str(&entry);
        }
        json.push_str("], ");

        // 节点
        json.push_str("\"nodes\": [");
        for (i, op) in graph.operations.iter().enumerate() {
            if i > 0 { json.push_str(", "); }
            let entry = format!(
                r#"{{"op": "{}", "name": "{}", "inputs": {:?}, "outputs": {:?}}}"#,
                op.op_type.name(),
                op.name,
                op.inputs,
                op.outputs
            );
            json.push_str(&entry);
        }
        json.push_str("]");

        json.push('}');
        Ok(json)
    }

    /// 生成可视化 DOT 格式
    pub fn generate_dot(&mut self, graph: &NpuGraph) -> String {
        let mut dot = String::new();
        dot.push_str("digraph NpuGraph {\n");
        dot.push_str("  rankdir=TB;\n");
        dot.push_str("  node [shape=box];\n\n");

        // 输入节点
        dot.push_str("  // Inputs\n");
        for input in &graph.inputs {
            dot.push_str(&format!(
                "  \"{}\" [shape=ellipse, label=\"{}\\n{:?}\"];\n",
                input.name,
                input.name,
                input.shape
            ));
        }
        dot.push_str("\n");

        // 操作节点
        dot.push_str("  // Operations\n");
        for op in &graph.operations {
            dot.push_str(&format!(
                "  \"{}\" [label=\"{}\\n{}\"];\n",
                op.name,
                op.op_type.name(),
                op.name
            ));

            // 输入边
            for input in &op.inputs {
                dot.push_str(&format!(
                    "  \"{}\" -> \"{}\";\n",
                    input,
                    op.name
                ));
            }

            // 输出边
            for output in &op.outputs {
                dot.push_str(&format!(
                    "  \"{}\" -> \"{}\";\n",
                    op.name,
                    output
                ));
            }
        }
        dot.push_str("\n");

        // 输出节点
        dot.push_str("  // Outputs\n");
        for output in &graph.outputs {
            dot.push_str(&format!(
                "  \"{}\" [shape=ellipse, label=\"{}\\n{:?}\"];\n",
                output.name,
                output.name,
                output.shape
            ));
        }

        dot.push_str("}\n");
        dot
    }
}

impl Default for NpuCodeGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_generation() {
        let mut generator = NpuCodeGenerator::new();
        let graph = NpuGraph::new("test");

        let json = generator.generate_json(&graph).unwrap();
        assert!(json.contains("\"name\": \"test\""));
    }

    #[test]
    fn test_dot_generation() {
        let mut generator = NpuCodeGenerator::new();
        let graph = NpuGraph::new("test");

        let dot = generator.generate_dot(&graph);
        assert!(dot.contains("digraph NpuGraph"));
    }
}
