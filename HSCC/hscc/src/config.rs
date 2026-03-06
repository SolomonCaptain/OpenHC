use serde::Deserialize;
use std::fs;
use anyhow::Result;

/// 后端类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// CUDA C++ 后端
    Cuda,
    /// Triton Python 后端
    Triton,
    /// HIP (AMD GPU) 后端
    Hip,
}

impl Default for Backend {
    fn default() -> Self {
        Backend::Cuda
    }
}

impl Backend {
    /// 从字符串解析后端类型
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "cuda" | "gpu" => Backend::Cuda,
            "triton" => Backend::Triton,
            "hip" | "rocm" | "amd" => Backend::Hip,
            _ => Backend::Cuda,
        }
    }
    
    /// 获取后端名称
    pub fn name(&self) -> &str {
        match self {
            Backend::Cuda => "cuda",
            Backend::Triton => "triton",
            Backend::Hip => "hip",
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub package: Package,
    pub target: Target,
    /// 可选的编译后端配置
    #[serde(default)]
    pub backend: Option<BackendConfig>,
}

#[derive(Debug, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub edition: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Target {
    pub device: String,
    pub arch: Option<String>,
}

/// 后端配置
#[derive(Debug, Deserialize, Default)]
pub struct BackendConfig {
    /// 后端类型: cuda, triton, hip
    #[serde(default)]
    pub kind: Option<String>,
    /// Triton 特定配置
    #[serde(default)]
    pub triton: Option<TritonConfig>,
}

/// Triton 后端配置
#[derive(Debug, Deserialize, Clone)]
pub struct TritonConfig {
    /// 默认块大小
    #[serde(default = "default_block_size")]
    pub block_size: u32,
    /// 使用的 warp 数量
    #[serde(default = "default_num_warps")]
    pub num_warps: u32,
    /// 流水线阶段数
    #[serde(default = "default_num_stages")]
    pub num_stages: u32,
}

fn default_block_size() -> u32 { 1024 }
fn default_num_warps() -> u32 { 4 }
fn default_num_stages() -> u32 { 2 }

impl Default for TritonConfig {
    fn default() -> Self {
        Self {
            block_size: default_block_size(),
            num_warps: default_num_warps(),
            num_stages: default_num_stages(),
        }
    }
}

impl Config {
    pub fn from_file(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
    
    /// 获取编译后端
    pub fn get_backend(&self) -> Backend {
        // 优先使用 backend.kind
        if let Some(ref backend_cfg) = self.backend {
            if let Some(ref kind) = backend_cfg.kind {
                return Backend::from_str(kind);
            }
        }
        
        // 兼容旧的 target.device 字段
        Backend::from_str(&self.target.device)
    }
    
    /// 获取 Triton 配置
    pub fn get_triton_config(&self) -> TritonConfig {
        self.backend
            .as_ref()
            .and_then(|b| b.triton.clone())
            .unwrap_or_default()
    }
    
    /// 检查是否使用 Triton 后端
    pub fn is_triton(&self) -> bool {
        self.get_backend() == Backend::Triton
    }
}