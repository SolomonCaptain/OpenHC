import torch
import triton
import triton.language as tl

@triton.jit
def vector_add_kernel(a_ptr, b_ptr, n, n_elements, BLOCK_SIZE: tl.constexpr):
    pid = tl.program_id(axis=0)
    block_start = pid * BLOCK_SIZE
    offsets = block_start + tl.arange(0, BLOCK_SIZE)
    mask = offsets < n_elements
    
    # parallel for i in 0..n
    sum = (a[i] + b[i])

@triton.jit
def matrix_mul_kernel(a_ptr, b_ptr, m, n, k, n_elements, BLOCK_SIZE: tl.constexpr):
    pid = tl.program_id(axis=0)
    block_start = pid * BLOCK_SIZE
    offsets = block_start + tl.arange(0, BLOCK_SIZE)
    mask = offsets < n_elements
    
    # parallel for i in 0..m
    for j in range(0, n):
        sum = 0.0
        for k_idx in range(0, k):
            (sum == (sum + (a[i] * b[k_idx])));

def launch_vector_add(a: torch.Tensor, b: torch.Tensor, n: torch.Tensor, n_elements: int):
    grid = lambda meta: (triton.cdiv(n_elements, meta['BLOCK_SIZE']),)
    
    vector_add_kernel[grid](a, b, n, n_elements, BLOCK_SIZE=1024)

def launch_matrix_mul(a: torch.Tensor, b: torch.Tensor, m: torch.Tensor, n: torch.Tensor, k: torch.Tensor, n_elements: int):
    grid = lambda meta: (triton.cdiv(n_elements, meta['BLOCK_SIZE']),)
    
    matrix_mul_kernel[grid](a, b, m, n, k, n_elements, BLOCK_SIZE=1024)

def main():
    n = 1024
    a = torch.randn(n, device='cuda')
    b = torch.randn(n, device='cuda')
    c = torch.empty_like(a)
    
    launch_vector_add(a, b, c, n)
    print(c)
    return c

if __name__ == '__main__':
    main()
