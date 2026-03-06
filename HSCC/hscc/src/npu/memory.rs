//! NPU 内存规划
//!
//! 实现 NPU 内存复用和布局优化。

use std::collections::HashMap;
use super::types::TensorLayout;
use super::graph::NpuGraph;
use super::backends::{NpuHardwareSpec, NpuError};

/// 内存规划结果
#[derive(Debug, Clone)]
pub struct MemoryPlan {
    /// 总内存大小（字节）
    pub total_size: usize,
    /// 张量内存分配
    pub allocations: HashMap<String, TensorAllocation>,
    /// 内存池配置
    pub pools: Vec<MemoryPool>,
}

/// 张量内存分配
#[derive(Debug, Clone)]
pub struct TensorAllocation {
    /// 张量名称
    pub tensor_name: String,
    /// 内存偏移
    pub offset: usize,
    /// 大小
    pub size: usize,
    /// 所属内存池
    pub pool_id: u32,
}

/// 内存池
#[derive(Debug, Clone)]
pub struct MemoryPool {
    /// 池 ID
    pub id: u32,
    /// 池大小
    pub size: usize,
    /// 池类型
    pub pool_type: MemoryPoolType,
}

/// 内存池类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryPoolType {
    /// 片上 SRAM
    SRAM,
    /// 外部内存（HBM/DDR）
    External,
    /// 保留
    Reserved,
}

/// 内存规划器
pub struct MemoryPlanner {
    spec: NpuHardwareSpec,
}

impl MemoryPlanner {
    /// 创建内存规划器
    pub fn new(spec: NpuHardwareSpec) -> Self {
        Self { spec }
    }

    /// 执行内存规划
    pub fn plan(&self, graph: &mut NpuGraph) -> Result<MemoryPlan, NpuError> {
        // 使用贪心算法进行内存复用
        let mut allocator = MemoryAllocator::new(
            self.spec.local_memory_kb as usize * 1024,
            self.spec.hbm_size_gb as usize * 1024 * 1024 * 1024,
        );

        // 按生命周期分配内存
        for tensor in graph.tensors.values() {
            let size = tensor.dtype.size_in_bytes();
            let lifetime = (tensor.lifetime_start, tensor.lifetime_end);
            allocator.allocate(&tensor.name, size, lifetime);
        }

        allocator.into_memory_plan()
    }

    /// 优化内存布局
    pub fn optimize_layout(&self, graph: &mut NpuGraph) {
        // 根据设备选择最优布局
        let preferred_layout = self.spec.preferred_layout;

        for tensor in graph.tensors.values_mut() {
            if tensor.layout == TensorLayout::Unknown {
                tensor.layout = preferred_layout;
            }
        }
    }
}

/// 内存分配器
pub struct MemoryAllocator {
    /// SRAM 容量
    sram_capacity: usize,
    /// 外部内存容量
    external_capacity: usize,
    /// 分配记录
    allocations: Vec<AllocationRecord>,
    /// 当前偏移
    current_offset: usize,
}

/// 分配记录
#[derive(Debug, Clone)]
struct AllocationRecord {
    tensor_name: String,
    offset: usize,
    size: usize,
    lifetime: (usize, usize),
    pool_type: MemoryPoolType,
}

impl MemoryAllocator {
    /// 创建内存分配器
    pub fn new(sram_capacity: usize, external_capacity: usize) -> Self {
        Self {
            sram_capacity,
            external_capacity,
            allocations: Vec::new(),
            current_offset: 0,
        }
    }

    /// 分配内存
    pub fn allocate(&mut self, name: &str, size: usize, lifetime: (usize, usize)) {
        // 尝试在 SRAM 中分配
        let pool_type = if size <= self.sram_capacity && self.can_fit_in_sram(size, lifetime) {
            MemoryPoolType::SRAM
        } else {
            MemoryPoolType::External
        };

        let offset = self.current_offset;
        self.current_offset += size;

        self.allocations.push(AllocationRecord {
            tensor_name: name.to_string(),
            offset,
            size,
            lifetime,
            pool_type,
        });
    }

    /// 检查是否可以放入 SRAM
    fn can_fit_in_sram(&self, size: usize, _lifetime: (usize, usize)) -> bool {
        // 简化实现：检查当前使用量
        let used: usize = self.allocations.iter()
            .filter(|r| r.pool_type == MemoryPoolType::SRAM)
            .map(|r| r.size)
            .sum();
        used + size <= self.sram_capacity
    }

    /// 转换为内存规划
    pub fn into_memory_plan(self) -> Result<MemoryPlan, NpuError> {
        let total_size = self.current_offset;

        let allocations: HashMap<String, TensorAllocation> = self.allocations
            .iter()
            .enumerate()
            .map(|(i, r)| {
                (r.tensor_name.clone(), TensorAllocation {
                    tensor_name: r.tensor_name.clone(),
                    offset: r.offset,
                    size: r.size,
                    pool_id: i as u32,
                })
            })
            .collect();

        Ok(MemoryPlan {
            total_size,
            allocations,
            pools: vec![
                MemoryPool {
                    id: 0,
                    size: self.sram_capacity,
                    pool_type: MemoryPoolType::SRAM,
                },
                MemoryPool {
                    id: 1,
                    size: self.external_capacity,
                    pool_type: MemoryPoolType::External,
                },
            ],
        })
    }
}

impl MemoryPlan {
    /// 获取张量的内存偏移
    pub fn get_offset(&self, tensor_name: &str) -> Option<usize> {
        self.allocations.get(tensor_name).map(|a| a.offset)
    }

    /// 获取总内存使用量
    pub fn total_memory(&self) -> usize {
        self.total_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_allocator() {
        let mut allocator = MemoryAllocator::new(1024, 1024 * 1024);

        allocator.allocate("tensor1", 256, (0, 5));
        allocator.allocate("tensor2", 512, (3, 10));
        allocator.allocate("tensor3", 128, (6, 15));

        let plan = allocator.into_memory_plan().unwrap();
        assert_eq!(plan.total_size, 896);
    }
}
