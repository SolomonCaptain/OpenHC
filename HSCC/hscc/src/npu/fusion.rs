//! NPU 算子融合优化
//!
//! 实现常见的算子融合模式，提高 NPU 执行效率：
//! - MatMul + Bias + Activation → FusedMatMul
//! - Conv + BN + ReLU → FusedConv
//! - LayerNorm + Linear + Residual → TransformerBlock

use std::collections::{HashMap, HashSet};
use super::graph::{NpuGraph, NpuOperation, NpuOpType, NpuTensor, OpHints, Padding};
use super::types::{NpuType, NpuTypeKind, TensorLayout};

/// 融合模式类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FusionPattern {
    /// MatMul + BiasAdd + Activation
    MatMulBiasActivation,
    /// MatMul + BiasAdd
    MatMulBias,
    /// Conv + BiasAdd + Activation
    ConvBiasActivation,
    /// Conv + BatchNorm + Activation
    ConvBNActivation,
    /// LayerNorm + Linear + Residual
    TransformerBlock,
    /// Softmax 融合
    SoftmaxFusion,
    /// Element-wise 链式融合
    ElementWiseChain,
    /// Flash Attention
    FlashAttention,
    /// Add + LayerNorm
    AddLayerNorm,
    /// Linear + GELU
    LinearGELU,
}

/// 融合规则
#[derive(Debug, Clone)]
pub struct FusionRule {
    /// 融合模式
    pub pattern: FusionPattern,
    /// 匹配的算子序列
    pub op_sequence: Vec<&'static str>,
    /// 融合后的算子名称
    pub fused_op_name: &'static str,
    /// 性能收益估计 (1.0 = 100% 提升)
    pub estimated_benefit: f32,
}

impl FusionRule {
    /// 创建新的融合规则
    pub fn new(
        pattern: FusionPattern,
        op_sequence: Vec<&'static str>,
        fused_op_name: &'static str,
        estimated_benefit: f32,
    ) -> Self {
        Self {
            pattern,
            op_sequence,
            fused_op_name,
            estimated_benefit,
        }
    }
}

/// 融合优化器
pub struct FusionOptimizer {
    /// 可用的融合规则
    rules: Vec<FusionRule>,
    /// 已融合的操作索引
    fused_ops: HashSet<usize>,
}

impl FusionOptimizer {
    /// 创建融合优化器
    pub fn new() -> Self {
        Self {
            rules: Self::create_default_rules(),
            fused_ops: HashSet::new(),
        }
    }

    /// 创建默认融合规则
    fn create_default_rules() -> Vec<FusionRule> {
        vec![
            // MatMul + Add (Bias) + Activation 融合
            FusionRule::new(
                FusionPattern::MatMulBiasActivation,
                vec!["MatMul", "Add", "Relu"],
                "FusedMatMulRelu",
                0.3,
            ),
            FusionRule::new(
                FusionPattern::MatMulBiasActivation,
                vec!["MatMul", "Add", "Sigmoid"],
                "FusedMatMulSigmoid",
                0.25,
            ),
            FusionRule::new(
                FusionPattern::MatMulBiasActivation,
                vec!["MatMul", "Add", "Tanh"],
                "FusedMatMulTanh",
                0.25,
            ),
            FusionRule::new(
                FusionPattern::MatMulBiasActivation,
                vec!["MatMul", "Add", "Gelu"],
                "FusedMatMulGelu",
                0.35,
            ),
            FusionRule::new(
                FusionPattern::MatMulBias,
                vec!["MatMul", "Add"],
                "FusedMatMulBias",
                0.2,
            ),

            // Conv + Activation 融合
            FusionRule::new(
                FusionPattern::ConvBiasActivation,
                vec!["Conv", "Relu"],
                "FusedConvRelu",
                0.25,
            ),
            FusionRule::new(
                FusionPattern::ConvBiasActivation,
                vec!["Conv", "Sigmoid"],
                "FusedConvSigmoid",
                0.2,
            ),
            FusionRule::new(
                FusionPattern::ConvBiasActivation,
                vec!["Conv", "Add", "Relu"],
                "FusedConvBiasRelu",
                0.3,
            ),

            // BatchNorm 融合
            FusionRule::new(
                FusionPattern::ConvBNActivation,
                vec!["Conv", "BatchNormalization", "Relu"],
                "FusedConvBNRelu",
                0.4,
            ),
            FusionRule::new(
                FusionPattern::ConvBNActivation,
                vec!["Conv", "BatchNormalization"],
                "FusedConvBN",
                0.35,
            ),

            // Transformer 相关融合
            FusionRule::new(
                FusionPattern::AddLayerNorm,
                vec!["Add", "LayerNormalization"],
                "FusedAddLayerNorm",
                0.3,
            ),
            FusionRule::new(
                FusionPattern::LinearGELU,
                vec!["MatMul", "Gelu"],
                "FusedLinearGelu",
                0.3,
            ),

            // Softmax 融合
            FusionRule::new(
                FusionPattern::SoftmaxFusion,
                vec!["Exp", "ReduceSum", "Div"],
                "FusedSoftmax",
                0.2,
            ),
        ]
    }

    /// 执行融合优化
    pub fn optimize(&mut self, graph: &mut NpuGraph) -> Vec<FusionResult> {
        let mut results = Vec::new();
        self.fused_ops.clear();

        let mut changed = true;
        while changed {
            changed = false;

            // 尝试所有融合规则
            for rule in self.rules.clone() {
                if let Some(result) = self.try_apply_rule(graph, &rule) {
                    results.push(result);
                    changed = true;
                    break;
                }
            }
        }

        results
    }

    /// 尝试应用融合规则
    fn try_apply_rule(&mut self, graph: &mut NpuGraph, rule: &FusionRule) -> Option<FusionResult> {
        // 查找匹配的操作序列
        let matches = self.find_matching_sequence(graph, &rule.op_sequence)?;

        // 检查是否已被融合
        if matches.iter().any(|idx| self.fused_ops.contains(idx)) {
            return None;
        }

        // 执行融合
        let fused_op = self.create_fused_operation(graph, &matches, rule)?;

        // 记录已融合的操作
        for idx in &matches {
            self.fused_ops.insert(*idx);
        }

        // 替换操作
        self.replace_operations(graph, &matches, fused_op);

        Some(FusionResult {
            pattern: rule.pattern,
            original_ops: matches.len(),
            fused_op_name: rule.fused_op_name.to_string(),
            estimated_speedup: rule.estimated_benefit,
        })
    }

    /// 查找匹配的操作序列
    fn find_matching_sequence(&self, graph: &NpuGraph, sequence: &[&'static str]) -> Option<Vec<usize>> {
        if sequence.is_empty() || graph.operations.is_empty() {
            return None;
        }

        // 使用滑动窗口查找匹配序列
        for start in 0..graph.operations.len() {
            if self.fused_ops.contains(&start) {
                continue;
            }

            let mut matches = Vec::new();
            let mut current_idx = start;

            for expected_op in sequence {
                // 跳过已融合的操作
                while current_idx < graph.operations.len() && self.fused_ops.contains(&current_idx) {
                    current_idx += 1;
                }

                if current_idx >= graph.operations.len() {
                    break;
                }

                let op = &graph.operations[current_idx];
                if op.op_type.name() == *expected_op {
                    matches.push(current_idx);
                    current_idx += 1;
                } else {
                    break;
                }
            }

            if matches.len() == sequence.len() {
                // 验证数据流连接
                if self.verify_dataflow(graph, &matches) {
                    return Some(matches);
                }
            }
        }

        None
    }

    /// 验证操作之间的数据流连接
    fn verify_dataflow(&self, graph: &NpuGraph, indices: &[usize]) -> bool {
        if indices.len() < 2 {
            return true;
        }

        for i in 0..indices.len() - 1 {
            let current_op = &graph.operations[indices[i]];
            let next_op = &graph.operations[indices[i + 1]];

            // 检查当前操作的输出是否是下一个操作的输入
            let has_connection = current_op.outputs.iter()
                .any(|output| next_op.inputs.contains(output));

            if !has_connection {
                return false;
            }
        }

        true
    }

    /// 创建融合后的操作
    fn create_fused_operation(
        &self,
        graph: &NpuGraph,
        indices: &[usize],
        rule: &FusionRule,
    ) -> Option<NpuOperation> {
        let first_op = &graph.operations[indices[0]];
        let last_op = &graph.operations[indices[indices.len() - 1]];

        // 收集所有输入（去重）
        let mut all_inputs: Vec<String> = Vec::new();
        let mut seen_inputs: HashSet<String> = HashSet::new();

        for &idx in indices {
            for input in &graph.operations[idx].inputs {
                if !seen_inputs.contains(input) && !indices.iter().any(|&i| {
                    graph.operations[i].outputs.contains(input)
                }) {
                    all_inputs.push(input.clone());
                    seen_inputs.insert(input.clone());
                }
            }
        }

        // 创建融合操作类型
        let fused_op_type = self.create_fused_op_type(rule, graph, indices)?;

        Some(NpuOperation {
            index: first_op.index,
            op_type: fused_op_type,
            name: format!("fused_{}", rule.fused_op_name.to_lowercase()),
            inputs: all_inputs,
            outputs: last_op.outputs.clone(),
            attributes: HashMap::new(),
            hints: OpHints {
                fuse_with_upstream: false,
                fuse_with_downstream: false,
                ..Default::default()
            },
        })
    }

    /// 创建融合后的操作类型
    fn create_fused_op_type(
        &self,
        rule: &FusionRule,
        graph: &NpuGraph,
        indices: &[usize],
    ) -> Option<NpuOpType> {
        match rule.pattern {
            FusionPattern::MatMulBiasActivation |
            FusionPattern::MatMulBias => {
                // 检查是否有激活函数
                let has_activation = rule.op_sequence.iter()
                    .any(|&op| matches!(op, "Relu" | "Sigmoid" | "Tanh" | "Gelu" | "Swish"));

                let activation = if has_activation {
                    rule.op_sequence.last().map(|&s| s.to_string())
                } else {
                    None
                };

                // 创建融合 MatMul（暂时用自定义操作表示）
                Some(NpuOpType::Custom {
                    op_name: rule.fused_op_name.to_string(),
                })
            }

            FusionPattern::ConvBiasActivation |
            FusionPattern::ConvBNActivation => {
                // 从第一个 Conv 操作获取属性
                if let NpuOpType::Conv2D { padding, stride, dilation, groups } =
                    graph.operations[indices[0]].op_type.clone() {
                    // 创建融合 Conv（暂时用自定义操作表示）
                    Some(NpuOpType::Custom {
                        op_name: rule.fused_op_name.to_string(),
                    })
                } else {
                    None
                }
            }

            FusionPattern::AddLayerNorm => {
                Some(NpuOpType::Custom {
                    op_name: "FusedAddLayerNorm".to_string(),
                })
            }

            FusionPattern::LinearGELU => {
                Some(NpuOpType::Custom {
                    op_name: "FusedLinearGelu".to_string(),
                })
            }

            FusionPattern::SoftmaxFusion => {
                Some(NpuOpType::Softmax { axis: -1 })
            }

            _ => Some(NpuOpType::Custom {
                op_name: rule.fused_op_name.to_string(),
            }),
        }
    }

    /// 替换操作
    fn replace_operations(&mut self, graph: &mut NpuGraph, indices: &[usize], fused_op: NpuOperation) {
        // 移除原始操作（从后向前移除以保持索引有效）
        let mut sorted_indices = indices.to_vec();
        sorted_indices.sort_by(|a, b| b.cmp(a));

        // 收集要移除的操作名称
        let mut to_remove = HashSet::new();
        for &idx in &sorted_indices {
            to_remove.insert(idx);
        }

        // 插入融合操作
        let insert_idx = indices[0];
        
        // 重建操作列表
        let mut new_operations = Vec::new();
        for (idx, op) in graph.operations.drain(..).enumerate() {
            if idx == insert_idx {
                new_operations.push(fused_op.clone());
            }
            if !to_remove.contains(&idx) {
                new_operations.push(op);
            }
        }

        // 如果插入位置在末尾
        if insert_idx >= new_operations.len() {
            new_operations.push(fused_op);
        }

        // 更新索引
        for (idx, op) in new_operations.iter_mut().enumerate() {
            op.index = idx;
        }

        graph.operations = new_operations;
    }
}

impl Default for FusionOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

/// 融合结果
#[derive(Debug, Clone)]
pub struct FusionResult {
    /// 融合模式
    pub pattern: FusionPattern,
    /// 原始操作数量
    pub original_ops: usize,
    /// 融合后操作名称
    pub fused_op_name: String,
    /// 估计加速比
    pub estimated_speedup: f32,
}

/// 融合分析器
pub struct FusionAnalyzer {
    /// 潜在融合机会
    opportunities: Vec<FusionOpportunity>,
}

/// 融合机会
#[derive(Debug, Clone)]
pub struct FusionOpportunity {
    /// 融合模式
    pub pattern: FusionPattern,
    /// 操作索引
    pub op_indices: Vec<usize>,
    /// 估计收益
    pub estimated_benefit: f32,
}

impl FusionAnalyzer {
    /// 创建融合分析器
    pub fn new() -> Self {
        Self {
            opportunities: Vec::new(),
        }
    }

    /// 分析图中的融合机会
    pub fn analyze(&mut self, graph: &NpuGraph) -> &[FusionOpportunity] {
        self.opportunities.clear();

        // 分析各种融合模式
        self.find_matmul_fusion_opportunities(graph);
        self.find_conv_fusion_opportunities(graph);
        self.find_norm_fusion_opportunities(graph);

        &self.opportunities
    }

    /// 查找 MatMul 融合机会
    fn find_matmul_fusion_opportunities(&mut self, graph: &NpuGraph) {
        for (idx, op) in graph.operations.iter().enumerate() {
            if matches!(op.op_type, NpuOpType::MatMul) {
                // 查找后续的 Add 和激活函数
                let mut chain = vec![idx];
                let mut current_outputs = op.outputs.clone();

                // 查找 BiasAdd
                if let Some(add_idx) = self.find_consumer_with_op(graph, &current_outputs, "Add") {
                    chain.push(add_idx);
                    current_outputs = graph.operations[add_idx].outputs.clone();
                }

                // 查找激活函数
                for activation in &["Relu", "Sigmoid", "Tanh", "Gelu", "Swish"] {
                    if let Some(act_idx) = self.find_consumer_with_op(graph, &current_outputs, activation) {
                        chain.push(act_idx);
                        break;
                    }
                }

                if chain.len() > 1 {
                    let chain_len = chain.len();
                    self.opportunities.push(FusionOpportunity {
                        pattern: FusionPattern::MatMulBiasActivation,
                        op_indices: chain,
                        estimated_benefit: 0.3 * (chain_len - 1) as f32,
                    });
                }
            }
        }
    }

    /// 查找 Conv 融合机会
    fn find_conv_fusion_opportunities(&mut self, graph: &NpuGraph) {
        for (idx, op) in graph.operations.iter().enumerate() {
            if matches!(op.op_type, NpuOpType::Conv2D { .. }) {
                let mut chain = vec![idx];
                let mut current_outputs = op.outputs.clone();

                // 查找 BatchNorm
                if let Some(bn_idx) = self.find_consumer_with_op(graph, &current_outputs, "BatchNormalization") {
                    chain.push(bn_idx);
                    current_outputs = graph.operations[bn_idx].outputs.clone();
                }

                // 查找激活函数
                for activation in &["Relu", "Sigmoid", "Tanh"] {
                    if let Some(act_idx) = self.find_consumer_with_op(graph, &current_outputs, activation) {
                        chain.push(act_idx);
                        break;
                    }
                }

                if chain.len() > 1 {
                    let chain_len = chain.len();
                    self.opportunities.push(FusionOpportunity {
                        pattern: FusionPattern::ConvBNActivation,
                        op_indices: chain,
                        estimated_benefit: 0.35 * (chain_len - 1) as f32,
                    });
                }
            }
        }
    }

    /// 查找归一化融合机会
    fn find_norm_fusion_opportunities(&mut self, graph: &NpuGraph) {
        for (idx, op) in graph.operations.iter().enumerate() {
            if matches!(op.op_type, NpuOpType::Add) {
                // 查找后续的 LayerNorm
                if let Some(ln_idx) = self.find_consumer_with_op(graph, &op.outputs, "LayerNormalization") {
                    self.opportunities.push(FusionOpportunity {
                        pattern: FusionPattern::AddLayerNorm,
                        op_indices: vec![idx, ln_idx],
                        estimated_benefit: 0.3,
                    });
                }
            }
        }
    }

    /// 查找消费指定输出的特定操作类型
    fn find_consumer_with_op(&self, graph: &NpuGraph, outputs: &[String], op_name: &str) -> Option<usize> {
        for (idx, op) in graph.operations.iter().enumerate() {
            if op.op_type.name() == op_name {
                if op.inputs.iter().any(|input| outputs.contains(input)) {
                    return Some(idx);
                }
            }
        }
        None
    }
}

impl Default for FusionAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fusion_optimizer_creation() {
        let optimizer = FusionOptimizer::new();
        assert!(!optimizer.rules.is_empty());
    }

    #[test]
    fn test_fusion_analyzer() {
        let mut analyzer = FusionAnalyzer::new();
        let graph = NpuGraph::new("test");
        let opportunities = analyzer.analyze(&graph);
        // 空图应该没有融合机会
        assert!(opportunities.is_empty());
    }

    #[test]
    fn test_fusion_rule_creation() {
        let rule = FusionRule::new(
            FusionPattern::MatMulBiasActivation,
            vec!["MatMul", "Add", "Relu"],
            "FusedMatMulRelu",
            0.3,
        );

        assert_eq!(rule.pattern, FusionPattern::MatMulBiasActivation);
        assert_eq!(rule.op_sequence.len(), 3);
        assert_eq!(rule.fused_op_name, "FusedMatMulRelu");
    }
}
