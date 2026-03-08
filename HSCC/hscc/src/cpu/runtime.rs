//! CPU 运行时支持
//!
//! 提供 CPU 后端的运行时功能

use std::collections::HashMap;

/// 运行时配置
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// 线程池大小
    pub thread_pool_size: usize,
    /// 是否启用 SIMD
    pub enable_simd: bool,
    /// 是否启用缓存优化
    pub enable_cache_opt: bool,
    /// 缓存行大小（字节）
    pub cache_line_size: usize,
    /// L1 缓存大小（字节）
    pub l1_cache_size: usize,
    /// L2 缓存大小（字节）
    pub l2_cache_size: usize,
    /// L3 缓存大小（字节）
    pub l3_cache_size: usize,
    /// 内存对齐（字节）
    pub memory_alignment: usize,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        RuntimeConfig {
            thread_pool_size: 0,  // 0 表示自动检测
            enable_simd: true,
            enable_cache_opt: true,
            cache_line_size: 64,
            l1_cache_size: 32 * 1024,      // 32 KB
            l2_cache_size: 256 * 1024,     // 256 KB
            l3_cache_size: 8 * 1024 * 1024, // 8 MB
            memory_alignment: 64,
        }
    }
}

impl RuntimeConfig {
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置线程池大小
    pub fn with_thread_pool(mut self, size: usize) -> Self {
        self.thread_pool_size = size;
        self
    }

    /// 禁用 SIMD
    pub fn without_simd(mut self) -> Self {
        self.enable_simd = false;
        self
    }

    /// 设置缓存大小
    pub fn with_cache_sizes(mut self, l1: usize, l2: usize, l3: usize) -> Self {
        self.l1_cache_size = l1;
        self.l2_cache_size = l2;
        self.l3_cache_size = l3;
        self
    }
}

/// CPU 运行时
pub struct CpuRuntime {
    /// 配置
    config: RuntimeConfig,
    /// 环境信息
    env_info: EnvironmentInfo,
}

/// 环境信息
#[derive(Debug, Clone)]
pub struct EnvironmentInfo {
    /// CPU 核心数
    pub num_cores: usize,
    /// 支持的 SIMD 指令集
    pub simd_features: Vec<String>,
    /// 总内存（字节）
    pub total_memory: u64,
    /// CPU 型号
    pub cpu_model: String,
}

impl Default for EnvironmentInfo {
    fn default() -> Self {
        EnvironmentInfo {
            num_cores: num_cpus::get(),
            simd_features: vec![],
            total_memory: 0,
            cpu_model: "Unknown".to_string(),
        }
    }
}

impl CpuRuntime {
    /// 创建新的运行时
    pub fn new(config: RuntimeConfig) -> Self {
        let env_info = Self::detect_environment();
        CpuRuntime { config, env_info }
    }

    /// 使用默认配置创建
    pub fn with_default_config() -> Self {
        CpuRuntime::new(RuntimeConfig::default())
    }

    /// 检测环境信息
    fn detect_environment() -> EnvironmentInfo {
        let mut info = EnvironmentInfo::default();

        // 检测 SIMD 支持
        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx512f") {
                info.simd_features.push("AVX512".to_string());
            }
            if is_x86_feature_detected!("avx2") {
                info.simd_features.push("AVX2".to_string());
            }
            if is_x86_feature_detected!("avx") {
                info.simd_features.push("AVX".to_string());
            }
            if is_x86_feature_detected!("sse4.2") {
                info.simd_features.push("SSE4.2".to_string());
            }
            if is_x86_feature_detected!("sse4.1") {
                info.simd_features.push("SSE4.1".to_string());
            }
        }

        #[cfg(target_arch = "aarch64")]
        {
            info.simd_features.push("NEON".to_string());
        }

        info
    }

    /// 获取配置
    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }

    /// 获取环境信息
    pub fn env_info(&self) -> &EnvironmentInfo {
        &self.env_info
    }

    /// 获取最优线程数
    pub fn optimal_thread_count(&self) -> usize {
        if self.config.thread_pool_size > 0 {
            self.config.thread_pool_size
        } else {
            self.env_info.num_cores
        }
    }

    /// 检查是否支持 SIMD
    pub fn supports_simd(&self, feature: &str) -> bool {
        self.env_info.simd_features.iter().any(|f| f == feature)
    }

    /// 获取最优的 SIMD 宽度
    pub fn optimal_simd_width(&self) -> usize {
        if self.supports_simd("AVX512") {
            16
        } else if self.supports_simd("AVX2") || self.supports_simd("AVX") {
            8
        } else if self.supports_simd("SSE4.1") || self.supports_simd("SSE4.2") {
            4
        } else {
            1
        }
    }

    /// 计算最优的分块大小
    pub fn optimal_tile_size(&self, element_size: usize, num_dims: usize) -> Vec<usize> {
        // 简化的分块大小计算
        // 目标：让每个块适合 L1 缓存
        let cache_size = self.config.l1_cache_size;
        let elements_per_tile = cache_size / element_size;

        match num_dims {
            1 => vec![elements_per_tile],
            2 => {
                let side = (elements_per_tile as f64).sqrt() as usize;
                vec![side, side]
            }
            3 => {
                let side = (elements_per_tile as f64).powf(1.0 / 3.0) as usize;
                vec![side, side, side]
            }
            _ => {
                let side = (elements_per_tile as f64).powf(1.0 / num_dims as f64) as usize;
                vec![side; num_dims]
            }
        }
    }
}

/// 生成运行时初始化代码
pub fn generate_runtime_init(config: &RuntimeConfig) -> String {
    let mut code = String::new();

    code.push_str("// CPU Runtime Initialization\n");
    code.push_str("namespace hsc_runtime {\n\n");

    // 线程数
    code.push_str(&format!(
        "constexpr int DEFAULT_NUM_THREADS = {};\n",
        if config.thread_pool_size > 0 {
            config.thread_pool_size
        } else {
            0  // 使用硬件线程数
        }
    ));

    // 缓存配置
    code.push_str(&format!("constexpr size_t CACHE_LINE_SIZE = {};\n", config.cache_line_size));
    code.push_str(&format!("constexpr size_t L1_CACHE_SIZE = {};\n", config.l1_cache_size));
    code.push_str(&format!("constexpr size_t L2_CACHE_SIZE = {};\n", config.l2_cache_size));
    code.push_str(&format!("constexpr size_t L3_CACHE_SIZE = {};\n", config.l3_cache_size));
    code.push_str(&format!("constexpr size_t MEMORY_ALIGNMENT = {};\n", config.memory_alignment));

    code.push_str("\n// Memory allocation with alignment (cross-platform)\n");
    code.push_str("template<typename T>\n");
    code.push_str("T* aligned_alloc(size_t count) {\n");
    code.push_str("#ifdef _WIN32\n");
    code.push_str(&format!(
        "    return static_cast<T*>(_aligned_malloc(count * sizeof(T), MEMORY_ALIGNMENT));\n"
    ));
    code.push_str("#else\n");
    code.push_str("    void* ptr = nullptr;\n");
    code.push_str(&format!(
        "    posix_memalign(&ptr, MEMORY_ALIGNMENT, count * sizeof(T));\n"
    ));
    code.push_str("    return static_cast<T*>(ptr);\n");
    code.push_str("#endif\n");
    code.push_str("}\n\n");

    code.push_str("template<typename T>\n");
    code.push_str("void aligned_free(T* ptr) {\n");
    code.push_str("#ifdef _WIN32\n");
    code.push_str("    _aligned_free(ptr);\n");
    code.push_str("#else\n");
    code.push_str("    free(ptr);\n");
    code.push_str("#endif\n");
    code.push_str("}\n\n");

    // 计时器
    code.push_str("// Timer utilities\n");
    code.push_str("class Timer {\n");
    code.push_str("    std::chrono::high_resolution_clock::time_point start_;\n");
    code.push_str("public:\n");
    code.push_str("    Timer() : start_(std::chrono::high_resolution_clock::now()) {}\n");
    code.push_str("    double elapsed_ms() const {\n");
    code.push_str("        auto end = std::chrono::high_resolution_clock::now();\n");
    code.push_str("        return std::chrono::duration<double, std::milli>(end - start_).count();\n");
    code.push_str("    }\n");
    code.push_str("    void reset() { start_ = std::chrono::high_resolution_clock::now(); }\n");
    code.push_str("};\n\n");

    code.push_str("} // namespace hsc_runtime\n");

    code
}

/// 生成设备查询代码
pub fn generate_device_query() -> String {
    let mut code = String::new();

    code.push_str("// Device Query\n");
    code.push_str("int get_device_count() { return 1; }  // CPU only\n");
    code.push_str("int get_current_device() { return Device::CPU; }\n");
    code.push_str("std::string get_device_name(int device) {\n");
    code.push_str("    return \"CPU\";\n");
    code.push_str("}\n\n");

    code.push_str("// Thread utilities\n");
    code.push_str("int get_num_threads() {\n");
    code.push_str("#ifdef _OPENMP\n");
    code.push_str("    return omp_get_max_threads();\n");
    code.push_str("#else\n");
    code.push_str("    return std::thread::hardware_concurrency();\n");
    code.push_str("#endif\n");
    code.push_str("}\n\n");

    code.push_str("void set_num_threads(int n) {\n");
    code.push_str("#ifdef _OPENMP\n");
    code.push_str("    omp_set_num_threads(n);\n");
    code.push_str("#endif\n");
    code.push_str("}\n");

    code
}

/// 生成内存管理代码
pub fn generate_memory_management() -> String {
    let mut code = String::new();

    code.push_str("// Memory Management\n");
    code.push_str("template<typename T>\n");
    code.push_str("class AlignedBuffer {\n");
    code.push_str("    T* data_;\n");
    code.push_str("    size_t size_;\n");
    code.push_str("public:\n");
    code.push_str("    AlignedBuffer(size_t size) : size_(size) {\n");
    code.push_str("        data_ = hsc_runtime::aligned_alloc<T>(size);\n");
    code.push_str("    }\n");
    code.push_str("    ~AlignedBuffer() {\n");
    code.push_str("        if (data_) hsc_runtime::aligned_free(data_);\n");
    code.push_str("    }\n");
    code.push_str("    T* data() { return data_; }\n");
    code.push_str("    const T* data() const { return data_; }\n");
    code.push_str("    size_t size() const { return size_; }\n");
    code.push_str("    // Non-copyable\n");
    code.push_str("    AlignedBuffer(const AlignedBuffer&) = delete;\n");
    code.push_str("    AlignedBuffer& operator=(const AlignedBuffer&) = delete;\n");
    code.push_str("    // Movable\n");
    code.push_str("    AlignedBuffer(AlignedBuffer&& other) noexcept\n");
    code.push_str("        : data_(other.data_), size_(other.size_) {\n");
    code.push_str("        other.data_ = nullptr;\n");
    code.push_str("    }\n");
    code.push_str("};\n\n");

    code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_config() {
        let config = RuntimeConfig::default()
            .with_thread_pool(4);

        assert_eq!(config.thread_pool_size, 4);
        assert!(config.enable_simd);
    }

    #[test]
    fn test_cpu_runtime() {
        let runtime = CpuRuntime::with_default_config();
        let thread_count = runtime.optimal_thread_count();

        assert!(thread_count >= 1);
    }

    #[test]
    fn test_simd_detection() {
        let runtime = CpuRuntime::with_default_config();
        let width = runtime.optimal_simd_width();

        assert!(width >= 1);
    }

    #[test]
    fn test_tile_size() {
        let runtime = CpuRuntime::with_default_config();
        let tile = runtime.optimal_tile_size(4, 2);  // float, 2D

        assert_eq!(tile.len(), 2);
    }
}
