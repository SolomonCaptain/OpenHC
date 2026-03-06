//! 算子融合优化器
//!
//! 实现以下融合模式：
//! - Element-wise 链式融合
//! - Matmul + Bias + Activation 融合
//! - Conv + BatchNorm + ReLU 融合
//! - Reduce + Element-wise 融合

use crate::ast::{Expression, Statement, Type, BinaryOp};
use std::collections::{HashMap, HashSet};

/// 融合模式类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FusionPattern {
    /// Element-wise 操作链 (如 add + mul + relu)
    ElementWiseChain,
    /// Matmul + Bias + Activation
    MatmulBiasActivation,
    /// Conv + BatchNorm + ReLU
    ConvBNReLU,
    /// Reduce + 后续 Element-wise
    ReduceElementWise,
    /// Softmax 融合
    SoftmaxFusion,
}

/// 可融合的操作类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FusibleOp {
    /// 元素级二元操作
    ElementWiseBinary { op: String },
    /// 元素级一元操作
    ElementWiseUnary { op: String },
    /// 矩阵乘法
    Matmul,
    /// 卷积
    Conv,
    /// 归约操作
    Reduce { kind: String, axis: Option<i32> },
    /// 激活函数
    Activation { kind: String },
    /// 批归一化
    BatchNorm,
    /// Softmax
    Softmax,
    /// Bias Add
    BiasAdd,
}

impl FusibleOp {
    /// 从 AST 表达式推断可融合操作
    pub fn from_expression(expr: &Expression) -> Option<Self> {
        match expr {
            Expression::Binary { op, left, right } => {
                // 检查是否是 element-wise 操作
                match op {
                    BinaryOp::Add => {
                        // 检查是否是 bias add
                        if matches!(right.as_ref(), Expression::Identifier(_)) {
                            Some(FusibleOp::BiasAdd)
                        } else {
                            Some(FusibleOp::ElementWiseBinary { op: "add".to_string() })
                        }
                    }
                    BinaryOp::Mul => Some(FusibleOp::ElementWiseBinary { op: "mul".to_string() }),
                    BinaryOp::Sub => Some(FusibleOp::ElementWiseBinary { op: "sub".to_string() }),
                    BinaryOp::Div => Some(FusibleOp::ElementWiseBinary { op: "div".to_string() }),
                    _ => None,
                }
            }
            Expression::Call { func, args } => {
                let func_name = match func.as_ref() {
                    Expression::Path(path) => path.segments.last().map(|s| s.ident.as_str()),
                    Expression::Identifier(name) => Some(name.as_str()),
                    _ => None,
                };
                
                match func_name {
                    Some("matmul") | Some("linear") => Some(FusibleOp::Matmul),
                    Some("conv") | Some("conv2d") => Some(FusibleOp::Conv),
                    Some("relu") | Some("gelu") | Some("sigmoid") | Some("tanh") => {
                        Some(FusibleOp::Activation { kind: func_name.unwrap().to_string() })
                    }
                    Some("softmax") => Some(FusibleOp::Softmax),
                    Some("batch_norm") | Some("batchnorm") => Some(FusibleOp::BatchNorm),
                    Some("sum") | Some("max") | Some("min") | Some("mean") => {
                        Some(FusibleOp::Reduce { kind: func_name.unwrap().to_string(), axis: None })
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }
    
    /// 检查是否可以与另一个操作融合
    pub fn can_fuse_with(&self, other: &FusibleOp) -> bool {
        match (self, other) {
            // Element-wise 链式融合
            (FusibleOp::ElementWiseBinary { .. }, FusibleOp::ElementWiseBinary { .. }) => true,
            (FusibleOp::ElementWiseBinary { .. }, FusibleOp::Activation { .. }) => true,
            (FusibleOp::ElementWiseUnary { .. }, FusibleOp::ElementWiseBinary { .. }) => true,
            
            // Matmul + Bias + Activation
            (FusibleOp::Matmul, FusibleOp::BiasAdd) => true,
            (FusibleOp::BiasAdd, FusibleOp::Activation { .. }) => true,
            
            // Conv + BN + ReLU
            (FusibleOp::Conv, FusibleOp::BatchNorm) => true,
            (FusibleOp::BatchNorm, FusibleOp::Activation { kind }) => {
                matches!(kind.as_str(), "relu" | "leaky_relu")
            }
            
            // Reduce + Element-wise (某些情况)
            (FusibleOp::Reduce { .. }, FusibleOp::ElementWiseBinary { .. }) => true,
            
            _ => false,
        }
    }
}

/// 融合组
#[derive(Debug, Clone)]
pub struct FusionGroup {
    /// 组内的操作序列
    pub ops: Vec<FusibleOp>,
    /// 输入变量
    pub inputs: HashSet<String>,
    /// 输出变量
    pub outputs: HashSet<String>,
    /// 融合模式
    pub pattern: FusionPattern,
}

impl FusionGroup {
    pub fn new() -> Self {
        Self {
            ops: Vec::new(),
            inputs: HashSet::new(),
            outputs: HashSet::new(),
            pattern: FusionPattern::ElementWiseChain,
        }
    }
    
    /// 添加操作到融合组
    pub fn add_op(&mut self, op: FusibleOp) {
        self.ops.push(op);
        self.update_pattern();
    }
    
    /// 根据操作序列更新融合模式
    fn update_pattern(&mut self) {
        self.pattern = if self.ops.len() >= 2 {
            match (&self.ops[0], &self.ops[1]) {
                (FusibleOp::Matmul, _) => FusionPattern::MatmulBiasActivation,
                (FusibleOp::Conv, _) => FusionPattern::ConvBNReLU,
                (FusibleOp::Reduce { .. }, _) => FusionPattern::ReduceElementWise,
                (FusibleOp::Softmax, _) => FusionPattern::SoftmaxFusion,
                _ => FusionPattern::ElementWiseChain,
            }
        } else {
            FusionPattern::ElementWiseChain
        }
    }
    
    /// 检查是否可以添加操作
    pub fn can_add(&self, op: &FusibleOp) -> bool {
        if self.ops.is_empty() {
            return true;
        }
        
        let last_op = self.ops.last().unwrap();
        last_op.can_fuse_with(op)
    }
    
    /// 生成融合后的 Triton 内核代码
    pub fn generate_fused_kernel(&self, kernel_name: &str) -> String {
        let mut code = String::new();
        
        code.push_str(&format!("# Fused kernel: {}\n", kernel_name));
        code.push_str(&format!("# Pattern: {:?}\n", self.pattern));
        code.push_str("@triton.jit\n");
        code.push_str(&format!("def {}_kernel(\n", kernel_name));
        code.push_str("    # Parameters would be generated here\n");
        code.push_str("):\n");
        
        // 根据融合模式生成代码
        match self.pattern {
            FusionPattern::MatmulBiasActivation => {
                code.push_str("    # Fused matmul + bias + activation\n");
                code.push_str("    # acc = tl.dot(a, b)\n");
                code.push_str("    # acc += bias\n");
                code.push_str("    # output = activation(acc)\n");
            }
            FusionPattern::ConvBNReLU => {
                code.push_str("    # Fused conv + batch_norm + relu\n");
            }
            FusionPattern::ElementWiseChain => {
                code.push_str("    # Fused element-wise operations\n");
                for (i, op) in self.ops.iter().enumerate() {
                    code.push_str(&format!("    # Step {}: {:?}\n", i, op));
                }
            }
            FusionPattern::ReduceElementWise => {
                code.push_str("    # Fused reduce + element-wise\n");
            }
            FusionPattern::SoftmaxFusion => {
                code.push_str("    # Fused softmax\n");
            }
        }
        
        code
    }
}

/// 算子融合优化器
pub struct FusionOptimizer {
    /// 已识别的融合组
    fusion_groups: Vec<FusionGroup>,
    /// 变量依赖图
    dependency_graph: HashMap<String, HashSet<String>>,
    /// 变量定义位置
    var_def_sites: HashMap<String, usize>,
}

impl FusionOptimizer {
    pub fn new() -> Self {
        Self {
            fusion_groups: Vec::new(),
            dependency_graph: HashMap::new(),
            var_def_sites: HashMap::new(),
        }
    }
    
    /// 分析语句块，识别融合机会
    pub fn analyze(&mut self, statements: &[Statement]) -> Vec<FusionGroup> {
        self.fusion_groups.clear();
        self.dependency_graph.clear();
        self.var_def_sites.clear();
        
        // 第一遍：构建依赖图
        for (idx, stmt) in statements.iter().enumerate() {
            self.analyze_statement_deps(stmt, idx);
        }
        
        // 第二遍：识别融合模式
        let mut current_group = FusionGroup::new();
        
        for (idx, stmt) in statements.iter().enumerate() {
            if let Some(op) = self.extract_fusible_op(stmt) {
                if current_group.can_add(&op) {
                    current_group.add_op(op);
                } else {
                    if !current_group.ops.is_empty() {
                        self.fusion_groups.push(current_group.clone());
                    }
                    current_group = FusionGroup::new();
                    current_group.add_op(op);
                }
            }
            
            // 更新变量定义位置
            if let Statement::Let { name, .. } = stmt {
                self.var_def_sites.insert(name.clone(), idx);
            }
        }
        
        // 保存最后一个组
        if !current_group.ops.is_empty() {
            self.fusion_groups.push(current_group);
        }
        
        self.fusion_groups.clone()
    }
    
    /// 分析语句的依赖关系
    fn analyze_statement_deps(&mut self, stmt: &Statement, idx: usize) {
        match stmt {
            Statement::Let { name, init, .. } => {
                self.var_def_sites.insert(name.clone(), idx);
                if let Some(expr) = init {
                    let deps = self.collect_expression_deps(expr);
                    self.dependency_graph.insert(name.clone(), deps);
                }
            }
            Statement::Expr(expr) => {
                let _deps = self.collect_expression_deps(expr);
            }
            Statement::Return(Some(expr)) => {
                let _deps = self.collect_expression_deps(expr);
            }
            _ => {}
        }
    }
    
    /// 收集表达式中的变量依赖
    fn collect_expression_deps(&self, expr: &Expression) -> HashSet<String> {
        let mut deps = HashSet::new();
        
        match expr {
            Expression::Identifier(name) => {
                deps.insert(name.clone());
            }
            Expression::Binary { left, right, .. } => {
                deps.extend(self.collect_expression_deps(left));
                deps.extend(self.collect_expression_deps(right));
            }
            Expression::Call { args, .. } => {
                for arg in args {
                    deps.extend(self.collect_expression_deps(arg));
                }
            }
            Expression::Index { obj, index } => {
                deps.extend(self.collect_expression_deps(obj));
                deps.extend(self.collect_expression_deps(index));
            }
            Expression::MethodCall { obj, args, .. } => {
                deps.extend(self.collect_expression_deps(obj));
                for arg in args {
                    deps.extend(self.collect_expression_deps(arg));
                }
            }
            Expression::Array(elems) => {
                for elem in elems {
                    deps.extend(self.collect_expression_deps(elem));
                }
            }
            _ => {}
        }
        
        deps
    }
    
    /// 从语句中提取可融合操作
    fn extract_fusible_op(&self, stmt: &Statement) -> Option<FusibleOp> {
        match stmt {
            Statement::Let { init, .. } => {
                init.as_ref().and_then(FusibleOp::from_expression)
            }
            Statement::Expr(expr) => FusibleOp::from_expression(expr),
            _ => None,
        }
    }
    
    /// 检查两个操作是否可以通过中间变量融合
    pub fn can_fuse_via_var(&self, var: &str) -> bool {
        // 如果变量只被使用一次，可以融合
        if let Some(deps) = self.dependency_graph.get(var) {
            deps.len() <= 1
        } else {
            false
        }
    }
    
    /// 应用融合优化
    pub fn apply_fusion(&self, groups: &[FusionGroup]) -> FusionResult {
        let mut result = FusionResult::new();
        
        for group in groups {
            if group.ops.len() > 1 {
                let kernel_name = format!("fused_{}", result.fused_kernels.len());
                let kernel_code = group.generate_fused_kernel(&kernel_name);
                
                result.fused_kernels.push(FusedKernel {
                    name: kernel_name,
                    code: kernel_code,
                    pattern: group.pattern.clone(),
                    original_ops: group.ops.len(),
                });
                
                result.total_fused_ops += group.ops.len();
            }
        }
        
        result
    }
    
    /// 获取融合组
    pub fn fusion_groups(&self) -> &[FusionGroup] {
        &self.fusion_groups
    }
}

impl Default for FusionOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

/// 融合后的内核
#[derive(Debug, Clone)]
pub struct FusedKernel {
    /// 内核名称
    pub name: String,
    /// 内核代码
    pub code: String,
    /// 融合模式
    pub pattern: FusionPattern,
    /// 原始操作数量
    pub original_ops: usize,
}

/// 融合优化结果
#[derive(Debug, Clone)]
pub struct FusionResult {
    /// 融合后的内核列表
    pub fused_kernels: Vec<FusedKernel>,
    /// 总共融合的操作数
    pub total_fused_ops: usize,
}

impl FusionResult {
    pub fn new() -> Self {
        Self {
            fused_kernels: Vec::new(),
            total_fused_ops: 0,
        }
    }
    
    /// 计算优化收益估计
    pub fn estimated_speedup(&self) -> f64 {
        // 每个融合操作减少一次内存访问
        // 假设内存访问占 GPU 执行时间的 50%
        let memory_savings = self.total_fused_ops as f64 * 0.5;
        let kernel_launch_savings = (self.fused_kernels.len() as f64) * 0.1;
        
        1.0 + memory_savings + kernel_launch_savings
    }
}

impl Default for FusionResult {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    
    fn parse_statements(source: &str) -> Vec<Statement> {
        let full_source = format!("fn test() {{ {} }}", source);
        let mut lexer = Lexer::new(&full_source);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().expect("Failed to parse");
        program.functions[0].body.statements.clone()
    }
    
    #[test]
    fn test_element_wise_detection() {
        let stmts = parse_statements(r#"
            let x = a + b;
            let y = x * c;
        "#);
        
        let mut optimizer = FusionOptimizer::new();
        let groups = optimizer.analyze(&stmts);
        
        assert!(!groups.is_empty());
    }
    
    #[test]
    fn test_fusible_op_detection() {
        let expr = Expression::Binary {
            left: Box::new(Expression::Identifier("a".to_string())),
            op: BinaryOp::Add,
            right: Box::new(Expression::Identifier("b".to_string())),
        };
        
        let op = FusibleOp::from_expression(&expr);
        assert!(matches!(op, Some(FusibleOp::ElementWiseBinary { .. })));
    }
    
    #[test]
    fn test_fusion_compatibility() {
        let matmul = FusibleOp::Matmul;
        let bias = FusibleOp::BiasAdd;
        let relu = FusibleOp::Activation { kind: "relu".to_string() };
        
        assert!(matmul.can_fuse_with(&bias));
        assert!(bias.can_fuse_with(&relu));
    }
    
    #[test]
    fn test_fusion_result() {
        let mut result = FusionResult::new();
        result.fused_kernels.push(FusedKernel {
            name: "test".to_string(),
            code: "test".to_string(),
            pattern: FusionPattern::ElementWiseChain,
            original_ops: 3,
        });
        result.total_fused_ops = 3;
        
        let speedup = result.estimated_speedup();
        assert!(speedup > 1.0);
    }
}
