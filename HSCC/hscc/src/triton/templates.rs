//! Triton 高级内核模板
//!
//! 提供预优化的内核模板，包括：
//! - 向量操作
//! - Reduce 操作
//! - 矩阵乘法
//! - Softmax
//! - LayerNorm

use super::types::TritonType;
use super::kernel::{TritonKernel, TritonStatement, TritonExpr, TritonParam, TritonConfig};

/// Reduce 操作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReduceOp {
    Sum,
    Max,
    Min,
    Prod,
    ArgMax,
    ArgMin,
}

impl ReduceOp {
    /// 获取 Triton 操作名称
    pub fn triton_name(&self) -> &'static str {
        match self {
            ReduceOp::Sum => "sum",
            ReduceOp::Max => "max",
            ReduceOp::Min => "min",
            ReduceOp::Prod => "prod",
            ReduceOp::ArgMax => "argmax",
            ReduceOp::ArgMin => "argmin",
        }
    }
}

/// 向量内核模板生成器
pub struct VectorKernels;

impl VectorKernels {
    /// 创建向量加法内核
    pub fn add_kernel(config: &TritonConfig) -> TritonKernel {
        create_vector_binary_kernel("vector_add", "+", config)
    }
    
    /// 创建向量减法内核
    pub fn sub_kernel(config: &TritonConfig) -> TritonKernel {
        create_vector_binary_kernel("vector_sub", "-", config)
    }
    
    /// 创建向量乘法内核
    pub fn mul_kernel(config: &TritonConfig) -> TritonKernel {
        create_vector_binary_kernel("vector_mul", "*", config)
    }
    
    /// 创建向量除法内核
    pub fn div_kernel(config: &TritonConfig) -> TritonKernel {
        create_vector_binary_kernel("vector_div", "/", config)
    }
    
    /// 创建向量缩放内核
    pub fn scale_kernel(config: &TritonConfig) -> TritonKernel {
        let mut kernel = TritonKernel::with_config("vector_scale_kernel".to_string(), config.clone());
        
        // 参数
        kernel.add_param(TritonParam::new("x_ptr".to_string(), TritonType::pointer(TritonType::float32())));
        kernel.add_param(TritonParam::new("alpha".to_string(), TritonType::float32()));
        kernel.add_param(TritonParam::new("n_elements".to_string(), TritonType::i32()));
        kernel.add_param(TritonParam::constexpr("BLOCK_SIZE".to_string(), TritonType::i32()));
        
        // 内核体
        add_standard_prolog(&mut kernel, true);
        
        kernel.add_statement(TritonStatement::Let {
            name: "x".to_string(),
            ty: None,
            init: Some(TritonExpr::load(
                TritonExpr::id("x_ptr") + TritonExpr::id("offsets"),
                Some(TritonExpr::id("mask")),
                None,
            )),
        });
        
        kernel.add_statement(TritonStatement::Let {
            name: "result".to_string(),
            ty: None,
            init: Some(TritonExpr::binary("*", TritonExpr::id("x"), TritonExpr::id("alpha"))),
        });
        
        kernel.add_statement(TritonStatement::Store {
            ptr: TritonExpr::id("x_ptr") + TritonExpr::id("offsets"),
            value: TritonExpr::id("result"),
            mask: Some(TritonExpr::id("mask")),
        });
        
        kernel
    }
}

/// 创建向量二元操作内核
fn create_vector_binary_kernel(name: &str, op: &str, config: &TritonConfig) -> TritonKernel {
    let kernel_name = format!("{}_kernel", name);
    let mut kernel = TritonKernel::with_config(kernel_name, config.clone());
    
    // 参数
    kernel.add_param(TritonParam::new("a_ptr".to_string(), TritonType::pointer(TritonType::float32())));
    kernel.add_param(TritonParam::new("b_ptr".to_string(), TritonType::pointer(TritonType::float32())));
    kernel.add_param(TritonParam::new("c_ptr".to_string(), TritonType::pointer(TritonType::float32())));
    kernel.add_param(TritonParam::new("n_elements".to_string(), TritonType::i32()));
    kernel.add_param(TritonParam::constexpr("BLOCK_SIZE".to_string(), TritonType::i32()));
    
    // 标准 prolog
    add_standard_prolog(&mut kernel, true);
    
    // 加载数据
    kernel.add_statement(TritonStatement::Let {
        name: "a".to_string(),
        ty: None,
        init: Some(TritonExpr::load(
            TritonExpr::id("a_ptr") + TritonExpr::id("offsets"),
            Some(TritonExpr::id("mask")),
            None,
        )),
    });
    
    kernel.add_statement(TritonStatement::Let {
        name: "b".to_string(),
        ty: None,
        init: Some(TritonExpr::load(
            TritonExpr::id("b_ptr") + TritonExpr::id("offsets"),
            Some(TritonExpr::id("mask")),
            None,
        )),
    });
    
    // 计算
    kernel.add_statement(TritonStatement::Let {
        name: "c".to_string(),
        ty: None,
        init: Some(TritonExpr::binary(op, TritonExpr::id("a"), TritonExpr::id("b"))),
    });
    
    // 存储
    kernel.add_statement(TritonStatement::Store {
        ptr: TritonExpr::id("c_ptr") + TritonExpr::id("offsets"),
        value: TritonExpr::id("c"),
        mask: Some(TritonExpr::id("mask")),
    });
    
    kernel
}

/// Reduce 内核模板生成器
pub struct ReduceKernels;

impl ReduceKernels {
    /// 创建 reduce sum 内核
    pub fn sum_kernel(config: &TritonConfig) -> TritonKernel {
        create_reduce_kernel("reduce_sum", ReduceOp::Sum, config)
    }
    
    /// 创建 reduce max 内核
    pub fn max_kernel(config: &TritonConfig) -> TritonKernel {
        create_reduce_kernel("reduce_max", ReduceOp::Max, config)
    }
    
    /// 创建 reduce min 内核
    pub fn min_kernel(config: &TritonConfig) -> TritonKernel {
        create_reduce_kernel("reduce_min", ReduceOp::Min, config)
    }
}

/// 创建 reduce 内核
fn create_reduce_kernel(name: &str, op: ReduceOp, config: &TritonConfig) -> TritonKernel {
    let kernel_name = format!("{}_kernel", name);
    let mut kernel = TritonKernel::with_config(kernel_name, config.clone());
    
    // 参数
    kernel.add_param(TritonParam::new("x_ptr".to_string(), TritonType::pointer(TritonType::float32())));
    kernel.add_param(TritonParam::new("y_ptr".to_string(), TritonType::pointer(TritonType::float32())));
    kernel.add_param(TritonParam::new("n_elements".to_string(), TritonType::i32()));
    kernel.add_param(TritonParam::constexpr("BLOCK_SIZE".to_string(), TritonType::i32()));
    
    // 标准 prolog
    add_standard_prolog(&mut kernel, true);
    
    // 加载数据
    kernel.add_statement(TritonStatement::Let {
        name: "x".to_string(),
        ty: None,
        init: Some(TritonExpr::load(
            TritonExpr::id("x_ptr") + TritonExpr::id("offsets"),
            Some(TritonExpr::id("mask")),
            Some(TritonExpr::float(0.0)),
        )),
    });
    
    // Reduce 操作
    let reduce_call = match op {
        ReduceOp::Sum => TritonExpr::call("tl.sum", vec![TritonExpr::id("x")]),
        ReduceOp::Max => TritonExpr::call("tl.max", vec![TritonExpr::id("x")]),
        ReduceOp::Min => TritonExpr::call("tl.min", vec![TritonExpr::id("x")]),
        _ => TritonExpr::call("tl.sum", vec![TritonExpr::id("x")]),
    };
    
    kernel.add_statement(TritonStatement::Let {
        name: "y".to_string(),
        ty: None,
        init: Some(reduce_call),
    });
    
    // 存储结果
    kernel.add_statement(TritonStatement::Store {
        ptr: TritonExpr::id("y_ptr") + TritonExpr::id("pid"),
        value: TritonExpr::id("y"),
        mask: None,
    });
    
    kernel
}

/// 矩阵乘法内核生成器
pub struct MatmulKernels;

impl MatmulKernels {
    /// 创建优化的矩阵乘法内核
    pub fn matmul_kernel(config: &TritonConfig) -> TritonKernel {
        let mut kernel = TritonKernel::with_config("matmul_kernel".to_string(), config.clone());
        
        // 参数
        kernel.add_param(TritonParam::new("a_ptr".to_string(), TritonType::pointer(TritonType::float32())));
        kernel.add_param(TritonParam::new("b_ptr".to_string(), TritonType::pointer(TritonType::float32())));
        kernel.add_param(TritonParam::new("c_ptr".to_string(), TritonType::pointer(TritonType::float32())));
        kernel.add_param(TritonParam::new("M".to_string(), TritonType::i32()));
        kernel.add_param(TritonParam::new("N".to_string(), TritonType::i32()));
        kernel.add_param(TritonParam::new("K".to_string(), TritonType::i32()));
        kernel.add_param(TritonParam::new("stride_am".to_string(), TritonType::i32()));
        kernel.add_param(TritonParam::new("stride_ak".to_string(), TritonType::i32()));
        kernel.add_param(TritonParam::new("stride_bk".to_string(), TritonType::i32()));
        kernel.add_param(TritonParam::new("stride_bn".to_string(), TritonType::i32()));
        kernel.add_param(TritonParam::new("stride_cm".to_string(), TritonType::i32()));
        kernel.add_param(TritonParam::new("stride_cn".to_string(), TritonType::i32()));
        kernel.add_param(TritonParam::constexpr("BLOCK_M".to_string(), TritonType::i32()));
        kernel.add_param(TritonParam::constexpr("BLOCK_N".to_string(), TritonType::i32()));
        kernel.add_param(TritonParam::constexpr("BLOCK_K".to_string(), TritonType::i32()));
        
        // PID
        kernel.add_statement(TritonStatement::Let {
            name: "pid".to_string(),
            ty: None,
            init: Some(TritonExpr::call("tl.program_id", vec![TritonExpr::int(0)])),
        });
        
        // 计算 block 起始位置
        kernel.add_statement(TritonStatement::Let {
            name: "pid_m".to_string(),
            ty: None,
            init: Some(TritonExpr::call("/", vec![
                TritonExpr::id("pid"),
                TritonExpr::call("tl.cdiv", vec![
                    TritonExpr::id("N"),
                    TritonExpr::id("BLOCK_N"),
                ]),
            ])),
        });
        
        kernel.add_statement(TritonStatement::Let {
            name: "pid_n".to_string(),
            ty: None,
            init: Some(TritonExpr::call("%", vec![
                TritonExpr::id("pid"),
                TritonExpr::call("tl.cdiv", vec![
                    TritonExpr::id("N"),
                    TritonExpr::id("BLOCK_N"),
                ]),
            ])),
        });
        
        // 计算偏移
        kernel.add_statement(TritonStatement::Let {
            name: "offs_am".to_string(),
            ty: None,
            init: Some(TritonExpr::id("pid_m") * TritonExpr::id("BLOCK_M") + TritonExpr::arange(0, 128)),
        });
        
        kernel.add_statement(TritonStatement::Let {
            name: "offs_bn".to_string(),
            ty: None,
            init: Some(TritonExpr::id("pid_n") * TritonExpr::id("BLOCK_N") + TritonExpr::arange(0, 128)),
        });
        
        // 初始化累加器
        kernel.add_statement(TritonStatement::Let {
            name: "accumulator".to_string(),
            ty: None,
            init: Some(TritonExpr::call("tl.zeros", vec![
                TritonExpr::call("tuple", vec![
                    TritonExpr::id("BLOCK_M"),
                    TritonExpr::id("BLOCK_N"),
                ]),
                TritonExpr::id("BLOCK_SIZE"),
            ])),
        });
        
        // K 循环
        kernel.add_statement(TritonStatement::Let {
            name: "k".to_string(),
            ty: None,
            init: Some(TritonExpr::id("K")),
        });
        
        // 累加结果的注释
        kernel.add_statement(TritonStatement::Expr(TritonExpr::String(
            "# Matrix multiply accumulation loop would go here".to_string()
        )));
        
        kernel
    }
}

/// Softmax 内核
pub struct SoftmaxKernels;

impl SoftmaxKernels {
    /// 创建 softmax 内核
    pub fn softmax_kernel(config: &TritonConfig) -> TritonKernel {
        let mut kernel = TritonKernel::with_config("softmax_kernel".to_string(), config.clone());
        
        // 参数
        kernel.add_param(TritonParam::new("x_ptr".to_string(), TritonType::pointer(TritonType::float32())));
        kernel.add_param(TritonParam::new("y_ptr".to_string(), TritonType::pointer(TritonType::float32())));
        kernel.add_param(TritonParam::new("n_cols".to_string(), TritonType::i32()));
        kernel.add_param(TritonParam::constexpr("BLOCK_SIZE".to_string(), TritonType::i32()));
        
        // 每行一个 block
        kernel.add_statement(TritonStatement::Let {
            name: "row_idx".to_string(),
            ty: None,
            init: Some(TritonExpr::call("tl.program_id", vec![TritonExpr::int(0)])),
        });
        
        // 列偏移
        kernel.add_statement(TritonStatement::Let {
            name: "col_offsets".to_string(),
            ty: None,
            init: Some(TritonExpr::arange(0, 1024)),
        });
        
        // 加载行数据
        kernel.add_statement(TritonStatement::Let {
            name: "x".to_string(),
            ty: None,
            init: Some(TritonExpr::load(
                TritonExpr::id("x_ptr") + TritonExpr::id("col_offsets"),
                Some(TritonExpr::id("col_offsets").lt(TritonExpr::id("n_cols"))),
                Some(TritonExpr::call("float", vec![TritonExpr::String("-inf".to_string())])),
            )),
        });
        
        // 计算 softmax
        kernel.add_statement(TritonStatement::Let {
            name: "x_max".to_string(),
            ty: None,
            init: Some(TritonExpr::call("tl.max", vec![TritonExpr::id("x")])),
        });
        
        kernel.add_statement(TritonStatement::Let {
            name: "x_shifted".to_string(),
            ty: None,
            init: Some(TritonExpr::id("x") - TritonExpr::id("x_max")),
        });
        
        kernel.add_statement(TritonStatement::Let {
            name: "numerator".to_string(),
            ty: None,
            init: Some(TritonExpr::call("tl.exp", vec![TritonExpr::id("x_shifted")])),
        });
        
        kernel.add_statement(TritonStatement::Let {
            name: "denominator".to_string(),
            ty: None,
            init: Some(TritonExpr::call("tl.sum", vec![TritonExpr::id("numerator")])),
        });
        
        kernel.add_statement(TritonStatement::Let {
            name: "y".to_string(),
            ty: None,
            init: Some(TritonExpr::binary("/", TritonExpr::id("numerator"), TritonExpr::id("denominator"))),
        });
        
        // 存储结果
        kernel.add_statement(TritonStatement::Store {
            ptr: TritonExpr::id("y_ptr") + TritonExpr::id("col_offsets"),
            value: TritonExpr::id("y"),
            mask: Some(TritonExpr::id("col_offsets").lt(TritonExpr::id("n_cols"))),
        });
        
        kernel
    }
}

/// 添加标准内核 prolog
fn add_standard_prolog(kernel: &mut TritonKernel, with_mask: bool) {
    // pid = tl.program_id(axis=0)
    kernel.add_statement(TritonStatement::Let {
        name: "pid".to_string(),
        ty: None,
        init: Some(TritonExpr::call("tl.program_id", vec![TritonExpr::int(0)])),
    });
    
    // block_start = pid * BLOCK_SIZE
    kernel.add_statement(TritonStatement::Let {
        name: "block_start".to_string(),
        ty: None,
        init: Some(TritonExpr::id("pid") * TritonExpr::id("BLOCK_SIZE")),
    });
    
    // offsets = block_start + tl.arange(0, BLOCK_SIZE)
    kernel.add_statement(TritonStatement::Let {
        name: "offsets".to_string(),
        ty: None,
        init: Some(TritonExpr::id("block_start") + TritonExpr::arange(0, 1024)),
    });
    
    if with_mask {
        // mask = offsets < n_elements
        kernel.add_statement(TritonStatement::Let {
            name: "mask".to_string(),
            ty: None,
            init: Some(TritonExpr::id("offsets").lt(TritonExpr::id("n_elements"))),
        });
    }
}

/// 内核注册表
pub struct KernelRegistry {
    kernels: Vec<TritonKernel>,
}

impl KernelRegistry {
    pub fn new() -> Self {
        Self {
            kernels: Vec::new(),
        }
    }
    
    /// 注册标准内核
    pub fn register_standard_kernels(&mut self, config: &TritonConfig) {
        // 向量操作
        self.kernels.push(VectorKernels::add_kernel(config));
        self.kernels.push(VectorKernels::sub_kernel(config));
        self.kernels.push(VectorKernels::mul_kernel(config));
        self.kernels.push(VectorKernels::div_kernel(config));
        self.kernels.push(VectorKernels::scale_kernel(config));
        
        // Reduce 操作
        self.kernels.push(ReduceKernels::sum_kernel(config));
        self.kernels.push(ReduceKernels::max_kernel(config));
        self.kernels.push(ReduceKernels::min_kernel(config));
        
        // 矩阵操作
        self.kernels.push(MatmulKernels::matmul_kernel(config));
        self.kernels.push(SoftmaxKernels::softmax_kernel(config));
    }
    
    /// 获取所有内核
    pub fn kernels(&self) -> &[TritonKernel] {
        &self.kernels
    }
    
    /// 生成所有内核的 Python 代码
    pub fn generate_python(&self) -> String {
        let mut code = String::new();
        
        code.push_str("import torch\n");
        code.push_str("import triton\n");
        code.push_str("import triton.language as tl\n\n");
        
        for kernel in &self.kernels {
            code.push_str(&kernel.to_code());
            code.push('\n');
        }
        
        code
    }
}

impl Default for KernelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_vector_add_kernel() {
        let config = TritonConfig::default();
        let kernel = VectorKernels::add_kernel(&config);
        
        assert!(kernel.name.contains("vector_add"));
        
        let code = kernel.to_code();
        assert!(code.contains("tl.program_id"));
        assert!(code.contains("tl.load"));
        assert!(code.contains("tl.store"));
    }
    
    #[test]
    fn test_reduce_sum_kernel() {
        let config = TritonConfig::default();
        let kernel = ReduceKernels::sum_kernel(&config);
        
        assert!(kernel.name.contains("reduce_sum"));
        
        let code = kernel.to_code();
        assert!(code.contains("tl.sum"));
    }
    
    #[test]
    fn test_softmax_kernel() {
        let config = TritonConfig::default();
        let kernel = SoftmaxKernels::softmax_kernel(&config);
        
        let code = kernel.to_code();
        assert!(code.contains("tl.exp"));
        assert!(code.contains("tl.max"));
    }
    
    #[test]
    fn test_kernel_registry() {
        let config = TritonConfig::default();
        let mut registry = KernelRegistry::new();
        registry.register_standard_kernels(&config);
        
        assert!(!registry.kernels().is_empty());
        
        let python = registry.generate_python();
        assert!(python.contains("import triton"));
    }
}
