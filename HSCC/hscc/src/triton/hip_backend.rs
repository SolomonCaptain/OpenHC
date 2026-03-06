//! AMD GPU (ROCm/HIP) 后端支持
//!
//! 为 AMD GPU 提供 Triton 后端支持：
//! - ROCm 兼容的内核生成
//! - HIP 运行时集成
//! - MI 系列 GPU 优化

use super::autotuner::{HardwareSpec, TuningParams, DataType, KernelType};
use super::kernel::{TritonKernel, TritonModule, TritonConfig};
use super::templates::KernelRegistry;
use std::collections::HashMap;

/// AMD GPU 架构枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AMDArchitecture {
    /// GCN 架构 (Vega 系列)
    GCN5,
    /// RDNA 1 (RX 5000 系列)
    RDNA1,
    /// RDNA 2 (RX 6000 系列)
    RDNA2,
    /// RDNA 3 (RX 7000 系列)
    RDNA3,
    /// CDNA 1 (MI100)
    CDNA1,
    /// CDNA 2 (MI200, MI250, MI250X)
    CDNA2,
    /// CDNA 3 (MI300)
    CDNA3,
}

impl AMDArchitecture {
    /// 获取架构代号
    pub fn gfx_arch(&self) -> &'static str {
        match self {
            AMDArchitecture::GCN5 => "gfx900",
            AMDArchitecture::RDNA1 => "gfx1010",
            AMDArchitecture::RDNA2 => "gfx1030",
            AMDArchitecture::RDNA3 => "gfx1100",
            AMDArchitecture::CDNA1 => "gfx908",
            AMDArchitecture::CDNA2 => "gfx90a",
            AMDArchitecture::CDNA3 => "gfx940",
        }
    }
    
    /// 是否支持 MFMA (Matrix Fused Multiply-Add)
    pub fn supports_mfma(&self) -> bool {
        matches!(self, 
            AMDArchitecture::CDNA1 | 
            AMDArchitecture::CDNA2 | 
            AMDArchitecture::CDNA3
        )
    }
    
    /// 是否支持 WMMA
    pub fn supports_wmma(&self) -> bool {
        matches!(self,
            AMDArchitecture::RDNA2 |
            AMDArchitecture::RDNA3 |
            AMDArchitecture::CDNA2 |
            AMDArchitecture::CDNA3
        )
    }
    
    /// 获取最大工作组大小
    pub fn max_workgroup_size(&self) -> u32 {
        match self {
            AMDArchitecture::GCN5 => 256,
            AMDArchitecture::RDNA1 => 256,
            AMDArchitecture::RDNA2 => 256,
            AMDArchitecture::RDNA3 => 256,
            AMDArchitecture::CDNA1 => 1024,
            AMDArchitecture::CDNA2 => 1024,
            AMDArchitecture::CDNA3 => 1024,
        }
    }
}

/// ROCm 后端配置
#[derive(Debug, Clone)]
pub struct ROCmConfig {
    /// 目标架构
    pub architecture: AMDArchitecture,
    /// 是否使用 HIP 源码模式
    pub hip_source_mode: bool,
    /// 是否启用 FP16 优化
    pub enable_fp16: bool,
    /// 是否启用 BF16 优化
    pub enable_bf16: bool,
    /// 是否启用 MFMA 矩阵操作
    pub enable_mfma: bool,
    /// 最大共享内存 (字节)
    pub max_shared_memory: u32,
}

impl Default for ROCmConfig {
    fn default() -> Self {
        Self {
            architecture: AMDArchitecture::CDNA2,
            hip_source_mode: false,
            enable_fp16: true,
            enable_bf16: true,
            enable_mfma: true,
            max_shared_memory: 64 * 1024, // 64KB
        }
    }
}

impl ROCmConfig {
    /// 创建 MI250X 配置
    pub fn mi250x() -> Self {
        Self {
            architecture: AMDArchitecture::CDNA2,
            hip_source_mode: false,
            enable_fp16: true,
            enable_bf16: true,
            enable_mfma: true,
            max_shared_memory: 64 * 1024,
        }
    }
    
    /// 创建 MI300 配置
    pub fn mi300() -> Self {
        Self {
            architecture: AMDArchitecture::CDNA3,
            hip_source_mode: false,
            enable_fp16: true,
            enable_bf16: true,
            enable_mfma: true,
            max_shared_memory: 128 * 1024,
        }
    }
}

/// ROCm 后端代码生成器
pub struct ROCmBackend {
    /// 配置
    config: ROCmConfig,
    /// 已生成的内核
    kernels: HashMap<String, String>,
}

impl ROCmBackend {
    pub fn new(config: ROCmConfig) -> Self {
        Self {
            config,
            kernels: HashMap::new(),
        }
    }
    
    /// 使用默认配置创建
    pub fn with_default_config() -> Self {
        Self::new(ROCmConfig::default())
    }
    
    /// 生成 ROCm 兼容的 Python 内核代码
    pub fn generate_kernel(&mut self, kernel: &TritonKernel) -> String {
        let mut code = String::new();
        
        // ROCm 特定导入
        code.push_str("# ROCm/AMD GPU Backend\n");
        code.push_str("# Architecture: ");
        code.push_str(self.config.architecture.gfx_arch());
        code.push_str("\n\n");
        
        code.push_str("import torch\n");
        code.push_str("import triton\n");
        code.push_str("import triton.language as tl\n\n");
        
        // ROCm 兼容性检查
        code.push_str("# ROCm compatibility check\n");
        code.push_str("def is_rocm_available():\n");
        code.push_str("    return torch.version.hip is not None\n\n");

        // 生成内核装饰器
        code.push_str("@triton.jit\n");
        code.push_str(&format!("def {}_kernel(\n", kernel.name));

        // 添加参数
        let params = kernel.params.iter()
            .map(|p| format!("    {}: {}", p.name, p.ty.to_triton_string()))
            .collect::<Vec<_>>()
            .join(",\n");
        code.push_str(&params);
        code.push_str("\n):\n");

        // 添加 ROCm 特定的优化提示
        code.push_str("    # ROCm optimized\n");
        
        // 生成的内核体
        for stmt in &kernel.body {
            code.push_str(&self.generate_statement(stmt));
        }
        
        code.push_str("\n");
        
        self.kernels.insert(kernel.name.clone(), code.clone());
        code
    }
    
    /// 生成语句代码
    fn generate_statement(&self, stmt: &super::kernel::TritonStatement) -> String {
        use super::kernel::TritonStatement;
        
        match stmt {
            TritonStatement::Let { name, init, .. } => {
                if let Some(expr) = init {
                    format!("    {} = {}\n", name, self.generate_expr(expr))
                } else {
                    format!("    {} = None\n", name)
                }
            }
            TritonStatement::Store { ptr, value, mask } => {
                if let Some(m) = mask {
                    format!("    tl.store({}, {}, mask={})\n", 
                        self.generate_expr(ptr), 
                        self.generate_expr(value),
                        self.generate_expr(m))
                } else {
                    format!("    tl.store({}, {})\n", 
                        self.generate_expr(ptr), 
                        self.generate_expr(value))
                }
            }
            TritonStatement::If { condition, then_body, else_body } => {
                let mut code = format!("    if {}:\n", self.generate_expr(condition));
                for s in then_body {
                    code.push_str(&self.generate_statement(s));
                }
                if let Some(else_stmts) = else_body {
                    code.push_str("    else:\n");
                    for s in else_stmts {
                        code.push_str(&self.generate_statement(s));
                    }
                }
                code
            }
            TritonStatement::For { var, start, end, body } => {
                let mut code = format!("    for {} in range({}, {}):\n", 
                    var, self.generate_expr(start), self.generate_expr(end));
                for s in body {
                    code.push_str(&self.generate_statement(s));
                }
                code
            }
            _ => String::new()
        }
    }
    
    /// 生成表达式代码
    fn generate_expr(&self, expr: &super::kernel::TritonExpr) -> String {
        use super::kernel::TritonExpr;

        match expr {
            TritonExpr::Identifier(name) => name.clone(),
            TritonExpr::Int(i) => i.to_string(),
            TritonExpr::Float(f) => f.to_string(),
            TritonExpr::String(s) => format!("\"{}\"", s),
            TritonExpr::Binary { op, lhs, rhs } => {
                format!("({} {} {})",
                        self.generate_expr(lhs),
                        op,
                        self.generate_expr(rhs))
            }
            TritonExpr::Call { func, args } => {
                let args_str = args.iter()
                    .map(|a| self.generate_expr(a))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}({})", func, args_str)
            }
            TritonExpr::Index { obj, indices } => {
                let indices_str = indices.iter()
                    .map(|i| self.generate_expr(i))
                    .collect::<Vec<_>>()
                    .join("][");
                format!("{}[{}]", self.generate_expr(obj), indices_str)
            }
            _ => String::from("/* complex expr */"),
        }
    }
    
    /// 生成 HIP 启动代码
    pub fn generate_launch_code(&self, kernel_name: &str, config: &TritonConfig) -> String {
        let mut code = String::new();
        
        code.push_str(&format!("# HIP Launch wrapper for {}\n", kernel_name));
        code.push_str("def launch_kernel(*args, **kwargs):\n");
        code.push_str("    # Check ROCm availability\n");
        code.push_str("    if not is_rocm_available():\n");
        code.push_str("        raise RuntimeError('ROCm not available')\n\n");
        
        code.push_str("    # Grid configuration\n");
        code.push_str(&format!("    grid = lambda META: (\n"));
        code.push_str(&format!("        triton.cdiv(M, META['BLOCK_M']),\n"));
        code.push_str(&format!("        triton.cdiv(N, META['BLOCK_N']),\n"));
        code.push_str("    )\n\n");
        
        code.push_str(&format!("    {}_kernel[grid](*args, **kwargs)\n", kernel_name));
        
        code
    }
    
    /// 生成完整的 ROCm 模块
    pub fn generate_module(&self, registry: &KernelRegistry) -> String {
        let mut code = String::new();
        
        // 文件头
        code.push_str("#!/usr/bin/env python3\n");
        code.push_str("# Auto-generated ROCm Triton kernels\n");
        code.push_str(&format!("# Target: {}\n\n", self.config.architecture.gfx_arch()));
        
        // 导入
        code.push_str("import torch\n");
        code.push_str("import triton\n");
        code.push_str("import triton.language as tl\n");
        code.push_str("from typing import *\n\n");
        
        // ROCm 工具函数
        code.push_str("def get_device_properties():\n");
        code.push_str("    '''Get AMD GPU device properties'''\n");
        code.push_str("    props = torch.cuda.get_device_properties(0)\n");
        code.push_str("    return {\n");
        code.push_str("        'name': props.name,\n");
        code.push_str("        'total_memory': props.total_memory,\n");
        code.push_str("        'multi_processor_count': props.multi_processor_count,\n");
        code.push_str("    }\n\n");
        
        // 所有内核
        for (name, kernel_code) in &self.kernels {
            code.push_str(&format!("# Kernel: {}\n", name));
            code.push_str(kernel_code);
            code.push_str("\n");
        }
        
        code
    }
    
    /// 获取架构信息
    pub fn architecture(&self) -> AMDArchitecture {
        self.config.architecture
    }
}

/// 为 AMD GPU 优化的内核配置
pub fn get_amd_optimized_config(
    arch: AMDArchitecture, 
    kernel_type: KernelType
) -> TuningParams {
    let mut block_sizes = HashMap::new();
    
    match kernel_type {
        KernelType::Matmul => {
            // AMD GPU matmul 优化配置
            block_sizes.insert("BLOCK_M".to_string(), 128);
            block_sizes.insert("BLOCK_N".to_string(), 128);
            block_sizes.insert("BLOCK_K".to_string(), 16);
            
            let num_stages = if arch.supports_mfma() { 4 } else { 2 };
            
            TuningParams {
                block_sizes,
                num_warps: 8,
                num_stages,
                use_shared_memory: true,
                dtype: if arch == AMDArchitecture::CDNA3 { 
                    DataType::FP8 
                } else { 
                    DataType::BF16 
                },
                unroll_factor: 1,
            }
        }
        KernelType::Vector => {
            block_sizes.insert("BLOCK_SIZE".to_string(), 1024);
            
            TuningParams {
                block_sizes,
                num_warps: 8,
                num_stages: 1,
                use_shared_memory: false,
                dtype: DataType::FP32,
                unroll_factor: 4,
            }
        }
        KernelType::FlashAttention => {
            block_sizes.insert("BLOCK_M".to_string(), 64);
            block_sizes.insert("BLOCK_N".to_string(), 64);
            
            TuningParams {
                block_sizes,
                num_warps: 4,
                num_stages: 1,
                use_shared_memory: true,
                dtype: DataType::BF16,
                unroll_factor: 1,
            }
        }
        _ => {
            block_sizes.insert("BLOCK_SIZE".to_string(), 256);
            
            TuningParams {
                block_sizes,
                num_warps: 4,
                num_stages: 1,
                use_shared_memory: false,
                dtype: DataType::FP32,
                unroll_factor: 1,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_amd_architecture() {
        let mi250 = AMDArchitecture::CDNA2;
        assert!(mi250.supports_mfma());
        assert!(mi250.supports_wmma());
        assert_eq!(mi250.gfx_arch(), "gfx90a");
    }
    
    #[test]
    fn test_rocm_config() {
        let config = ROCmConfig::mi250x();
        assert_eq!(config.architecture, AMDArchitecture::CDNA2);
        assert!(config.enable_mfma);
    }
    
    #[test]
    fn test_rocm_backend_creation() {
        let backend = ROCmBackend::with_default_config();
        assert_eq!(backend.architecture(), AMDArchitecture::CDNA2);
    }
    
    #[test]
    fn test_amd_optimized_config_matmul() {
        let config = get_amd_optimized_config(AMDArchitecture::CDNA2, KernelType::Matmul);
        
        assert!(config.block_sizes.contains_key("BLOCK_M"));
        assert!(config.use_shared_memory);
        assert!(config.num_stages >= 2);
    }
}
