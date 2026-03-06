//! 自动调优器框架
//!
//! 根据硬件特性自动选择最优内核配置：
//! - 块大小优化
//! - Warp 数量优化
//! - 共享内存使用策略
//! - 多精度计算选择

use super::kernel::TritonConfig;
use super::types::TritonType;
use std::collections::HashMap;

/// 硬件特性描述
#[derive(Debug, Clone)]
pub struct HardwareSpec {
    /// GPU 架构名称
    pub architecture: String,
    /// 计算能力 (如 8.0, 9.0)
    pub compute_capability: (u32, u32),
    /// SM 数量
    pub num_sms: u32,
    /// 最大线程数每 SM
    pub max_threads_per_sm: u32,
    /// 最大块大小
    pub max_block_size: u32,
    /// 共享内存大小 (KB)
    pub shared_memory_kb: u32,
    /// 是否支持 Tensor Core
    pub has_tensor_core: bool,
    /// 是否支持 BF16
    pub supports_bf16: bool,
    /// 是否支持 FP8
    pub supports_fp8: bool,
    /// 内存带宽 (GB/s)
    pub memory_bandwidth: f64,
}

impl HardwareSpec {
    /// NVIDIA A100 规格
    pub fn a100() -> Self {
        Self {
            architecture: "Ampere".to_string(),
            compute_capability: (8, 0),
            num_sms: 108,
            max_threads_per_sm: 2048,
            max_block_size: 1024,
            shared_memory_kb: 164,
            has_tensor_core: true,
            supports_bf16: true,
            supports_fp8: false,
            memory_bandwidth: 2039.0,
        }
    }
    
    /// NVIDIA H100 规格
    pub fn h100() -> Self {
        Self {
            architecture: "Hopper".to_string(),
            compute_capability: (9, 0),
            num_sms: 132,
            max_threads_per_sm: 2048,
            max_block_size: 1024,
            shared_memory_kb: 228,
            has_tensor_core: true,
            supports_bf16: true,
            supports_fp8: true,
            memory_bandwidth: 3352.0,
        }
    }
    
    /// NVIDIA RTX 4090 规格
    pub fn rtx4090() -> Self {
        Self {
            architecture: "Ada Lovelace".to_string(),
            compute_capability: (8, 9),
            num_sms: 128,
            max_threads_per_sm: 1536,
            max_block_size: 1024,
            shared_memory_kb: 100,
            has_tensor_core: true,
            supports_bf16: true,
            supports_fp8: true,
            memory_bandwidth: 1008.0,
        }
    }
    
    /// AMD MI250X 规格 (近似)
    pub fn mi250x() -> Self {
        Self {
            architecture: "CDNA2".to_string(),
            compute_capability: (0, 0), // AMD 使用不同的版本系统
            num_sms: 110,
            max_threads_per_sm: 2048,
            max_block_size: 1024,
            shared_memory_kb: 64,
            has_tensor_core: true,
            supports_bf16: true,
            supports_fp8: false,
            memory_bandwidth: 1600.0,
        }
    }
    
    /// 默认规格 (保守配置)
    pub fn default_gpu() -> Self {
        Self {
            architecture: "Unknown".to_string(),
            compute_capability: (7, 0),
            num_sms: 80,
            max_threads_per_sm: 2048,
            max_block_size: 1024,
            shared_memory_kb: 48,
            has_tensor_core: false,
            supports_bf16: false,
            supports_fp8: false,
            memory_bandwidth: 900.0,
        }
    }
}

/// 内核类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelType {
    /// 向量操作
    Vector,
    /// 矩阵乘法
    Matmul,
    /// 卷积
    Convolution,
    /// Reduce 操作
    Reduce,
    /// Softmax
    Softmax,
    /// LayerNorm
    LayerNorm,
    /// FlashAttention
    FlashAttention,
}

/// 调优参数
#[derive(Debug, Clone)]
pub struct TuningParams {
    /// 块大小 (各维度)
    pub block_sizes: HashMap<String, u32>,
    /// Warp 数量
    pub num_warps: u32,
    /// 流水线阶段数
    pub num_stages: u32,
    /// 是否使用共享内存
    pub use_shared_memory: bool,
    /// 数据类型 (FP32, FP16, BF16, FP8)
    pub dtype: DataType,
    /// 展开因子
    pub unroll_factor: u32,
}

/// 数据类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    FP32,
    FP16,
    BF16,
    FP8,
    INT8,
}

impl DataType {
    pub fn bytes(&self) -> u32 {
        match self {
            DataType::FP32 => 4,
            DataType::FP16 => 2,
            DataType::BF16 => 2,
            DataType::FP8 => 1,
            DataType::INT8 => 1,
        }
    }
}

/// 自动调优器
pub struct AutoTuner {
    /// 硬件规格
    hardware: HardwareSpec,
    /// 历史调优结果缓存
    tuning_cache: HashMap<String, TuningParams>,
    /// 是否启用缓存
    enable_cache: bool,
}

impl AutoTuner {
    pub fn new(hardware: HardwareSpec) -> Self {
        Self {
            hardware,
            tuning_cache: HashMap::new(),
            enable_cache: true,
        }
    }
    
    /// 使用默认硬件规格创建
    pub fn with_default_hardware() -> Self {
        Self::new(HardwareSpec::default_gpu())
    }
    
    /// 为特定内核类型生成最优配置
    pub fn tune(&mut self, kernel_type: KernelType, problem_size: &ProblemSize) -> TuningParams {
        // 检查缓存
        let cache_key = format!("{:?}_{:?}", kernel_type, problem_size);
        if self.enable_cache {
            if let Some(cached) = self.tuning_cache.get(&cache_key) {
                return cached.clone();
            }
        }
        
        // 根据内核类型选择调优策略
        let params = match kernel_type {
            KernelType::Vector => self.tune_vector(problem_size),
            KernelType::Matmul => self.tune_matmul(problem_size),
            KernelType::Convolution => self.tune_conv(problem_size),
            KernelType::Reduce => self.tune_reduce(problem_size),
            KernelType::Softmax => self.tune_softmax(problem_size),
            KernelType::LayerNorm => self.tune_layernorm(problem_size),
            KernelType::FlashAttention => self.tune_flash_attention(problem_size),
        };
        
        // 缓存结果
        if self.enable_cache {
            self.tuning_cache.insert(cache_key, params.clone());
        }
        
        params
    }

    /// 向量内核调优
    fn tune_vector(&self, problem_size: &ProblemSize) -> TuningParams {
        let n = problem_size.total_elements();

        // 选择最大的 2 的幂次作为块大小，不超过问题规模且不超过硬件限制
        let block_size = (n as u32).min(self.hardware.max_block_size);

        let mut block_sizes = HashMap::new();
        block_sizes.insert("BLOCK_SIZE".to_string(), block_size);

        TuningParams {
            block_sizes,
            num_warps: 4,
            num_stages: 1,
            use_shared_memory: true,
            dtype: DataType::FP32,
            unroll_factor: 1,
        }
    }
    
    /// 矩阵乘法调优
    fn tune_matmul(&self, problem_size: &ProblemSize) -> TuningParams {
        let m = problem_size.dim(0);
        let n = problem_size.dim(1);
        let k = problem_size.dim(2);
        
        // 根据矩阵大小选择分块策略
        let (block_m, block_n, block_k) = if m >= 1024 && n >= 1024 && k >= 1024 {
            // 大矩阵：大分块
            (128, 128, 32)
        } else if m >= 512 || n >= 512 {
            // 中等矩阵
            (64, 64, 32)
        } else {
            // 小矩阵：小分块
            (32, 32, 16)
        };
        
        // 流水线深度
        let num_stages = if self.hardware.shared_memory_kb >= 100 && k >= 512 {
            4
        } else if self.hardware.shared_memory_kb >= 64 {
            2
        } else {
            1
        };
        
        // Warp 数量
        let num_warps = if block_m * block_n >= 8192 { 8 } else { 4 };
        
        // 数据类型选择
        let dtype = if self.hardware.supports_bf16 && !self.hardware.has_tensor_core {
            DataType::FP16
        } else if self.hardware.supports_bf16 {
            DataType::BF16
        } else {
            DataType::FP32
        };
        
        let mut block_sizes = HashMap::new();
        block_sizes.insert("BLOCK_M".to_string(), block_m);
        block_sizes.insert("BLOCK_N".to_string(), block_n);
        block_sizes.insert("BLOCK_K".to_string(), block_k);
        
        TuningParams {
            block_sizes,
            num_warps,
            num_stages,
            use_shared_memory: true,
            dtype,
            unroll_factor: 1,
        }
    }
    
    /// 卷积操作调优
    fn tune_conv(&self, problem_size: &ProblemSize) -> TuningParams {
        let mut block_sizes = HashMap::new();
        block_sizes.insert("BLOCK_M".to_string(), 64);
        block_sizes.insert("BLOCK_N".to_string(), 64);
        block_sizes.insert("BLOCK_K".to_string(), 16);
        
        TuningParams {
            block_sizes,
            num_warps: 4,
            num_stages: 2,
            use_shared_memory: true,
            dtype: DataType::FP16,
            unroll_factor: 1,
        }
    }
    
    /// Reduce 操作调优
    fn tune_reduce(&self, problem_size: &ProblemSize) -> TuningParams {
        let n = problem_size.dim(0);
        
        let block_size = if n >= 1_000_000 {
            1024
        } else if n >= 100_000 {
            512
        } else {
            256
        };
        
        let mut block_sizes = HashMap::new();
        block_sizes.insert("BLOCK_SIZE".to_string(), block_size);
        
        TuningParams {
            block_sizes,
            num_warps: 4,
            num_stages: 1,
            use_shared_memory: true, // Reduce 需要共享内存
            dtype: DataType::FP32,
            unroll_factor: 4,
        }
    }
    
    /// Softmax 调优
    fn tune_softmax(&self, problem_size: &ProblemSize) -> TuningParams {
        let cols = problem_size.dim(1);
        
        let block_size = if cols >= 8192 {
            1024
        } else if cols >= 4096 {
            512
        } else {
            256
        };
        
        let mut block_sizes = HashMap::new();
        block_sizes.insert("BLOCK_SIZE".to_string(), block_size);
        
        TuningParams {
            block_sizes,
            num_warps: 4,
            num_stages: 1,
            use_shared_memory: false,
            dtype: DataType::FP32,
            unroll_factor: 1,
        }
    }

    /// LayerNorm 调优
    fn tune_layernorm(&self, problem_size: &ProblemSize) -> TuningParams {
        let hidden_size = problem_size.dim(1);

        let block_size: u32 = if hidden_size >= 4096 {
            1024
        } else if hidden_size >= 1024 {
            512
        } else {
            (hidden_size.max(64)) as u32
        };

        let mut block_sizes = HashMap::new();
        block_sizes.insert("BLOCK_SIZE".to_string(), block_size);

        TuningParams {
            block_sizes,
            num_warps: 4,
            num_stages: 1,
            use_shared_memory: true,
            dtype: DataType::FP32,
            unroll_factor: 1,
        }
    }
    
    /// FlashAttention 调优
    fn tune_flash_attention(&self, problem_size: &ProblemSize) -> TuningParams {
        let seq_len = problem_size.dim(0);
        let head_dim = problem_size.dim(2);
        
        let block_m = if seq_len >= 1024 { 128 } else { 64 };
        let block_n = if head_dim >= 128 { 64 } else { 32 };
        
        let num_stages = if self.hardware.shared_memory_kb >= 100 { 4 } else { 2 };
        
        let mut block_sizes = HashMap::new();
        block_sizes.insert("BLOCK_M".to_string(), block_m);
        block_sizes.insert("BLOCK_N".to_string(), block_n);
        
        TuningParams {
            block_sizes,
            num_warps: 4,
            num_stages,
            use_shared_memory: true,
            dtype: if self.hardware.supports_bf16 { DataType::BF16 } else { DataType::FP16 },
            unroll_factor: 1,
        }
    }
    
    /// 将调优参数转换为 TritonConfig
    pub fn params_to_config(params: &TuningParams) -> TritonConfig {
        let mut config = TritonConfig::default();
        
        for (name, size) in &params.block_sizes {
            config.block_sizes.insert(name.clone(), *size);
        }
        
        config.num_warps = params.num_warps;
        config.num_stages = params.num_stages;
        config.use_shared_memory = params.use_shared_memory;
        config.unroll_factor = params.unroll_factor;
        
        config
    }
    
    /// 清除缓存
    pub fn clear_cache(&mut self) {
        self.tuning_cache.clear();
    }
}

/// 问题规模描述
#[derive(Debug, Clone)]
pub struct ProblemSize {
    dimensions: Vec<u64>,
}

impl ProblemSize {
    pub fn new(dimensions: Vec<u64>) -> Self {
        Self { dimensions }
    }
    
    pub fn dim(&self, idx: usize) -> u64 {
        self.dimensions.get(idx).copied().unwrap_or(1)
    }
    
    pub fn total_elements(&self) -> u64 {
        self.dimensions.iter().product()
    }
}

impl std::fmt::Display for ProblemSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let dims: Vec<String> = self.dimensions.iter().map(|d| d.to_string()).collect();
        write!(f, "({})", dims.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hardware_specs() {
        let a100 = HardwareSpec::a100();
        assert_eq!(a100.compute_capability, (8, 0));
        assert!(a100.has_tensor_core);
        
        let h100 = HardwareSpec::h100();
        assert!(h100.supports_fp8);
    }
    
    #[test]
    fn test_vector_tuning() {
        let mut tuner = AutoTuner::new(HardwareSpec::a100());
        let problem = ProblemSize::new(vec![1_000_000]);
        
        let params = tuner.tune(KernelType::Vector, &problem);
        assert!(params.block_sizes.contains_key("BLOCK_SIZE"));
        assert_eq!(params.num_stages, 1);
    }
    
    #[test]
    fn test_matmul_tuning() {
        let mut tuner = AutoTuner::new(HardwareSpec::a100());
        let problem = ProblemSize::new(vec![1024, 1024, 1024]);
        
        let params = tuner.tune(KernelType::Matmul, &problem);
        
        assert!(params.block_sizes.contains_key("BLOCK_M"));
        assert!(params.block_sizes.contains_key("BLOCK_N"));
        assert!(params.block_sizes.contains_key("BLOCK_K"));
        assert!(params.use_shared_memory);
        assert!(params.num_stages >= 2);
    }
    
    #[test]
    fn test_tuning_cache() {
        let mut tuner = AutoTuner::new(HardwareSpec::default_gpu());
        tuner.enable_cache = true;
        
        let problem = ProblemSize::new(vec![100_000]);
        
        let params1 = tuner.tune(KernelType::Vector, &problem);
        let params2 = tuner.tune(KernelType::Vector, &problem);
        
        // 缓存的结果应该相同
        assert_eq!(params1.block_sizes, params2.block_sizes);
        
        // 检查缓存已存储
        assert!(!tuner.tuning_cache.is_empty());
    }
    
    #[test]
    fn test_params_to_config() {
        let mut params = TuningParams {
            block_sizes: HashMap::new(),
            num_warps: 8,
            num_stages: 4,
            use_shared_memory: true,
            dtype: DataType::BF16,
            unroll_factor: 2,
        };
        params.block_sizes.insert("BLOCK_M".to_string(), 128);
        
        let config = AutoTuner::params_to_config(&params);
        
        assert_eq!(config.num_warps, 8);
        assert_eq!(config.num_stages, 4);
        assert!(config.use_shared_memory);
    }
}
