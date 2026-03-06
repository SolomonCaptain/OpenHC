# Triton 后端示例

本示例演示如何使用 HSCLang 的 Triton 后端生成 GPU 内核。

## 编译

```bash
# 使用 Triton 后端编译
hscc . --backend=triton

# 或者在 HSCC.toml 中设置 backend.kind = "triton"
hscc .
```

## 运行

```bash
# 安装依赖
pip install torch triton

# 运行生成的 Python 代码
python triton_demo.py
```

## 生成的代码说明

编译器会生成 `triton_demo.py` 文件，包含：

1. **Triton 内核定义**：使用 `@triton.jit` 装饰的内核函数
2. **启动函数**：配置 grid/block 并调用内核
3. **主函数**：创建数据并执行计算

## 预期输出

```python
# 向量加法内核
@triton.jit
def vector_add_kernel(
    a_ptr, b_ptr, c_ptr,
    n_elements,
    BLOCK_SIZE: tl.constexpr,
):
    pid = tl.program_id(axis=0)
    block_start = pid * BLOCK_SIZE
    offsets = block_start + tl.arange(0, BLOCK_SIZE)
    mask = offsets < n_elements
    
    a = tl.load(a_ptr + offsets, mask=mask)
    b = tl.load(b_ptr + offsets, mask=mask)
    c = a + b
    
    tl.store(c_ptr + offsets, c, mask=mask)
```

## 性能对比

Triton 后端的优势：

| 特性 | CUDA 手写 | Triton 生成 |
|-----|----------|------------|
| 内存合并 | 手动实现 | 自动优化 |
| 共享内存 | 手动管理 | 自动分配 |
| 张量核心 | 需要特殊指令 | 自动利用 |
| 跨平台 | 仅 NVIDIA | NVIDIA + AMD |
