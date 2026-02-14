# 异构仿真语言“HSCLang”设计：语法、语义与编程范式

## 一、设计哲学与核心思想

### 1.1 核心设计原则
- **显式与隐式的平衡**：关键异构特征显式表达，底层调度隐式优化
- **单一来源，多设备生成**：一套代码，自动映射到FPGA、NPU、GPU
- **数据流优先**：以数据移动和变换为中心思考问题
- **渐进式暴露**：初学者使用简单范式，专家可深入调优

### 1.2 借鉴现代语言精华

| 语言/框架 | 借鉴特性 | 在HSCLang中的应用 |
|----------|---------|--------------|
| **Rust** | 表达式导向、强类型、所有权 | 内存安全、数据竞争预防  |
| **Kokkos** | 执行模式+执行策略+计算体分离 | 异构任务的抽象模型    |
| **Unison** | 内容哈希标识、分布式透明 | 跨设备代码唯一标识    |
| **Vine** | 交互网并行模型 | 数据流图的自然表达    |
| **SYCL/OpenMP** | 单一源、目标注解 | 设备卸载的语法糖     |
| **ML家族** | 类型推断、模式匹配 | 简化异构数据处理     |


## 二、编程范式：顺序任务流+数据流

HSCLang的核心范式是**顺序任务流（Sequential Task Flow, STF）**与**显式数据流图**的结合 。

### 2.1 范式本质
- 开发者以**顺序方式**编写代码，描述任务及其数据依赖
- 运行时系统自动解析依赖，构建**有向无环图（DAG）**
- 任务在异构设备上**并行执行**，保证结果与顺序执行一致
- 数据在设备间**显式流动**，形成可观测的计算管道

### 2.2 开发者心智模型
开发者只需思考："我的数据从哪里来，经过什么变换，到哪里去"。无需关心：
- 哪个设备执行哪个任务（由运行时优化）
- 数据如何传输（由系统自动处理）
- 任务如何同步（由依赖图保证）


## 三、语法设计详解

### 3.1 基础语法风格
```rust
// HSCLang 示例 - 风格类似 Rust 与 Python 的融合
// 强类型但类型推断，表达式导向

// 导入异构计算库
use hetero::device::{GPU, NPU, FPGA};
use hetero::memory::Buffer;

// 主计算函数 - 自动异构并行
fn main() -> Result<()> {
    // 数据定义 - 类似 Python 的简洁
    let n = 1_000_000;
    let a = [f32; n] = load_input("data.bin")?;
    let b = [f32; n] = generate_signal(n);
    
    // 核心计算 - 任务式表达
    let result = hetero::compute! {
        // 任务1: 在GPU上进行大规模FFT
        task gpu::fft(input: a) -> temp1 {
            spec: [batch = 16, precision = "high"]
        }
        
        // 任务2: 在NPU上进行神经网络推理
        task npu::infer(model: "cnn_v2", input: b) -> temp2 {
            spec: [batch = 32, quantize = "int8"]
        }
        
        // 任务3: 在FPGA上进行实时融合 - 依赖前两个任务
        task fpga::fuse(a: temp1, b: temp2) -> output {
            spec: [pipeline = 4, latency = "critical"]
        }
    }.await?;  // 异步等待所有任务完成
    
    // 处理结果
    save_output("result.bin", output)?;
    
    Ok(())
}
```

### 3.2 设备与内存模型

**设备注解系统**
```rust
// 设备类型枚举
enum DeviceType {
    GPU(ComputeCapability),  // 如 8.0
    NPU(ArchVersion),        // 如 "ascend910b"
    FPGA(Family),            // 如 "ultrascale+"
    CPU,                     // 后备设备
}

// 内存空间抽象
enum MemorySpace {
    Host,           // 主机内存
    DeviceLocal,    // 设备本地内存（显存/BRAM）
    Unified,        // 统一内存（CXL支持）
    Pinned,         // 锁页内存（高速传输）
}

// 数据缓冲区定义
buffer![T] {
    space: MemorySpace,      // 存储空间
    device: Option<DeviceType>, // 当前所在设备
    shape: [usize],          // 多维形状
    layout: DataLayout,      // 行主序/列主序/自定义
}
```

### 3.3 任务定义语法

**任务三要素**
```rust
// 任务模板定义
task MyKernel {
    // 1. 执行模式 (Pattern)
    pattern: ParallelPattern {
        kind: For | Reduce | Scan | TaskGraph,  // 循环/规约/扫描/任务图
        independent: bool,                       // 是否可并行
        dynamic: bool,                           // 是否动态调度
    }
    
    // 2. 执行策略 (Execution Policy)
    policy: SchedulePolicy {
        device_hint: DeviceType,                  // 建议设备（可选）
        granularity: WorkGranularity,             // 细粒度/粗粒度
        priority: Priority,                        // 优先级
        recursive_split: bool,                     // 是否允许递归拆分 
    }
    
    // 3. 计算体 (Computational Body)
    body(inputs: (T...), outputs: (U...)) {
        // 实际计算代码
        // 支持FPGA风格的流水线语法
        pipeline {
            stage1: preprocess(input) -> middle,
            stage2: compute(middle) -> result,
            stage3: postprocess(result) -> output,
        }
        
        // 或GPU风格的并行循环
        parallel for i in 0..N {
            output[i] = f(input[i]);
        }
    }
}
```

### 3.4 依赖表达机制

**两种依赖表达方式**

1. **隐式数据依赖**（推荐）：
```rust
let x = task1();      // 产生x
let y = task2();      // 可并行执行
let z = task3(x);     // 依赖x，自动等待
```

2. **显式依赖图**（复杂场景）：
```rust
graph MyComputation {
    node A: preprocess(data);
    node B: extract_features(A);
    node C: run_npu_model(A);  // 与B并行
    node D: fuse(B, C);
    
    edge A -> B, A -> C, B -> D, C -> D;
    
    // 设备映射建议
    place B on GPU;
    place C on NPU;
    place A, D on FPGA;
}
```


## 四、异构计算模式库

HSCLang内置常用异构计算模式，开发者可直接调用或组合。

### 4.1 核心模式集合

```rust
// 1. 流水线并行模式 (适用于FPGA+GPU) 
pipeline ImageProcessing {
    stage FPGA::preprocess(raw) -> filtered;      // FPGA预处理
    stage GPU::detect_objects(filtered) -> boxes; // GPU检测
    stage NPU::classify(boxes) -> labels;         // NPU分类
    
    // 流水线控制
    depth = 4;  // 四级流水
    bubble = false;  // 不允许断流
}

// 2. 分治并行模式
div_conquer Simulation {
    // 自适应网格划分
    partition domain into subdomains by load;
    
    // 各子域在不同设备计算
    for subdomain in subdomains {
        spawn on best_device(subdomain) {
            solve(subdomain);
        }
    }
    
    // 边界同步
    sync boundaries every 10 steps;
}

// 3. 降阶模型模式 (GPU物理 + NPU代理)
rom_hybrid Simulation {
    // GPU每100步运行完整物理
    step gpu::full_physics() -> state every 100;
    
    // NPU在中间步运行代理模型
    step npu::surrogate(state) -> next_state for 99;
    
    // 误差校正
    correct when error > threshold using gpu;
}
```


## 五、完整示例：CFD+AI仿真

以下是一个完整的计算流体力学与AI增强仿真示例，展示HSCLang的实际应用。

```rust
// cfd_ai_sim.hl - 异构CFD仿真程序

import hetero::*;
import devices::{A100, Ascend910, VU9P};  // 具体设备型号

// 物理场定义
struct FlowField {
    @GPU   // 主要驻留在GPU内存
    velocity: Buffer<f32, 3>,  // 三维速度场
    
    @NPU   // 主要驻留在NPU内存
    turbulence: Buffer<f32, 3>, // 湍流参数
    
    @FPGA  // 主要驻留在FPGA BRAM
    boundary: Buffer<f32, 2>,   // 边界条件
}

// 仿真参数
struct SimParams {
    dt: f32,           // 时间步长
    max_steps: i32,    // 最大步数
    convergence: f32,  // 收敛阈值
}

// 主仿真任务
task CfdAiSimulation {
    pattern: TaskGraph {
        independent: false,  // 任务间有依赖
    }
    
    policy: Adaptive {  // 自适应调度 
        initial_device: "gpu",
        recursive_split: true,  // 允许动态拆分任务
        load_balancing: true,   // 动态负载均衡
    }
    
    body(params: SimParams, field: FlowField) -> FlowField {
        // 初始化
        let t = 0;
        let error = INFINITY;
        
        // 任务图构建
        while t < params.max_steps && error > params.convergence {
            // 步骤1: FPGA处理边界条件（低延迟实时处理）
            let boundary_updated = task FPGA::update_boundary {
                input: field.boundary,
                output: new_boundary,
                
                // FPGA特定优化
                pipeline_depth: 8,
                dsp_usage: "balanced",
            };
            
            // 步骤2: GPU求解NS方程（计算密集）
            let flow_updated = task GPU::solve_navier_stokes {
                input: (field.velocity, boundary_updated),
                output: new_velocity,
                
                // GPU特定优化
                block_size: (256, 1, 1),
                shared_memory: 48.kb,
                stream_priority: "high",
            };
            
            // 步骤3: NPU预测湍流（AI加速）
            let turb_predicted = task NPU::predict_turbulence {
                input: (field.turbulence, flow_updated),
                output: new_turbulence,
                
                // NPU特定优化
                model: "turbulence_cnn_v3",
                batch_size: 16,
                precision: "mixed",
            };
            
            // 步骤4: 误差估计（可在CPU或任意设备）
            error = task estimate_error {
                input: (flow_updated, turb_predicted),
                output: error_value,
                
                // 设备无关，运行时决定
                device_hint: Any,
            };
            
            // 更新场数据（自动处理依赖）
            field.velocity = flow_updated;
            field.turbulence = turb_predicted;
            
            t += 1;
            
            // 每100步输出状态
            if t % 100 == 0 {
                log!("Step {}: error = {}", t, error);
            }
        }
        
        // 返回最终结果
        field
    }
}

// 主函数
fn main() -> Result<()> {
    // 检测可用设备
    let devices = hetero::probe_devices();
    println!("Detected devices: {}", devices);
    
    // 配置仿真
    let params = SimParams {
        dt: 0.01,
        max_steps: 10000,
        convergence: 1e-6,
    };
    
    // 初始化流场
    let field = FlowField {
        velocity: Buffer::zeros([1024, 1024, 1024])?.place_on(GPU),
        turbulence: Buffer::zeros([1024, 1024, 1024])?.place_on(NPU),
        boundary: Buffer::zeros([1024, 1024])?.place_on(FPGA),
    };
    
    // 加载边界条件
    load_boundary_data("wing_profile.bin", field.boundary)?;
    
    // 运行仿真 - 自动异构并行
    println!("Starting CFD-AI simulation...");
    let result = CfdAiSimulation::run(params, field).await?;
    
    // 保存结果
    save_vtk("result.vtk", result)?;
    println!("Simulation completed!");
    
    Ok(())
}
```


## 六、高级特性设计

### 6.1 内容寻址与版本管理

借鉴Unison的设计，每个任务定义由内容哈希唯一标识：

```rust
// 任务定义自动生成哈希
task fft::forward = hash: "a1b2c3d4..."

// 部署时自动解析依赖
deploy {
    include hash: "a1b2c3d4...";  // 精确指定版本
    include fft::forward;          // 解析到当前环境版本
}
```

### 6.2 递归任务与动态图调整

支持任务在运行时动态拆分，适应异构设备：

```rust
task AdaptiveSolver {
    recursive: true,  // 允许递归
    
    body(problem: Problem, depth: i32) -> Solution {
        if problem.size() < THRESHOLD || depth > MAX_DEPTH {
            // 基础情况：直接求解
            solve_direct(problem)
        } else {
            // 递归情况：拆分问题
            let subproblems = partition(problem);
            
            // 动态决定每个子问题的设备
            parallel for sp in subproblems {
                // 根据子问题特征选择设备
                let device = select_device(sp);
                
                // 递归调用（可能在不同设备）
                spawn on device {
                    AdaptiveSolver::run(sp, depth + 1)
                }
            }.collect()
        }
    }
}
```

### 6.3 编译期元编程

支持在编译时生成异构代码：

```rust
// 编译时循环展开
meta for unroll_factor in [2, 4, 8] {
    // 为每个展开因子生成专门版本
    export kernel matmul_${unroll_factor}(a, b) {
        #pragma unroll unroll_factor
        for i in 0..N {
            // 循环体
        }
    }
}

// 编译时根据目标设备选择最优版本
let kernel = select_optimal_kernel!(matmul, target_device);
```


## 七、编译与构建体验

### 7.1 编译器架构

```
HSCLang源文件
       ↓
  前端解析器
       ↓
  HSC IR (基于MLIR)
       ↓
  设备无关优化
       ↓
  ┌──────┬──────┬──────┐
  ↓      ↓      ↓      ↓
GPU后端 NPU后端 FPGA后端 CPU后端
  ↓      ↓      ↓      ↓
PTX   CANN   HLS C++  LLVM IR
  ↓      ↓      ↓      ↓
  └──────┴──────┴──────┘
       ↓
  异构可执行文件
  (包含多设备二进制)
```

### 7.2 开发者工具链

```bash
# 编译命令
hscc build --target=hetero source.hl

# 设备探测
hscc probe devices

# 性能分析
hscc profile --visualize execution.json

# 交互式调试
hscc debug --device=gpu source.hl
```


## 八、总结：为什么HSCLang易于学习？

### 8.1 渐进式学习路径
1. **初级**：使用`task`注解现有函数，自动异构并行
2. **中级**：定义数据依赖，构建复杂任务图
3. **高级**：编写设备特定优化，定制调度策略

### 8.2 直观的概念映射
| 异构概念 | HSCLang表达                       |
|---------|---------------------------------|
| 设备内存 | `Buffer<T> with space`          |
| 内核启动 | `task device::kernel`           |
| 数据传输 | 自动推导，或`move_to(device)`         |
| 同步 | `await`或图依赖自动保证                 |
| 流水线 | `pipeline { stage1 -> stage2 }` |

### 8.3 可读性优先
- 关键字贴近自然语言（`task`, `pipeline`, `graph`）
- 类型注解清晰但不冗长
- 错误信息定位到具体设备和任务

通过这套设计，HSCLang能够让开发者专注于**仿真算法的逻辑本身**，而不是被底层硬件细节困扰，同时保留了充分的优化空间，满足从教学到科研、从原型到产品的全场景需求。