# HSCLang 语言详解

HSCLang（Heterogeneous Simulation Computing Language）是一门面向**异构仿真计算**的编译型语言，目标平台涵盖 **FPGA、NPU、GPU** 等加速设备。它旨在让开发者能够以高级抽象描述并行计算任务，同时精细控制任务在异构设备上的调度、数据流和内存布局，从而高效实现 CFD（计算流体动力学）、AI 推理、图像处理等领域的仿真应用。

## 一、设计目标与核心理念

- **异构一体**：将 CPU、GPU、NPU、FPGA 视为统一的计算资源，语言层面提供设备抽象和任务放置机制。
- **数据流驱动**：支持显式的流水线（pipeline）和数据流图（graph）描述，便于表达多阶段、多设备的协作计算。
- **模式化并行**：内置常见的并行模式（parallel for、reduce、scan），并允许用户自定义任务的调度策略和粒度。
- **内存与设备亲和**：通过 `Buffer` 类型和 `place_on` / `move_to` 操作，明确数据在异构内存空间（Host、DeviceLocal、Unified、Pinned）中的位置。

## 二、词法元素

### 关键字
语言包含三类关键字：

- **基础控制**：`fn`、`let`、`mut`、`if`、`else`、`while`、`loop`、`break`、`continue`、`return`、`match`、`type`、`struct`、`enum`、`trait`、`impl`、`true`、`false`、`nil`、`async`、`await`、`move`、`copy`、`clone`、`ref`、`deref`。
- **异构专用**：`task`、`pipeline`、`graph`、`spawn`、`buffer`、`device`、`pattern`、`policy`、`body`、`parallel`、`for`、`reduce`、`scan`、`stage`、`edge`、`node`、`place`、`on`。
- **设备与策略**：`GPU`、`NPU`、`FPGA`、`CPU`、`Host`、`DeviceLocal`、`Unified`、`Pinned`、`ParallelPattern`、`SchedulePolicy`、`WorkGranularity`、`Priority`、`DataLayout`。

### 操作符与分隔符
支持丰富的操作符，包括算术、位运算、比较、逻辑、赋值组合、范围（`..` `...`）、箭头（`->` `=>`）、管道（`|>` `<|`）等。分隔符采用类 C 风格：`{} [] () , ; : | &`。

### 字面量
- 整数：支持后缀（如 `u32`、`i64`），数字间可用下划线分隔。
- 浮点数：支持科学计数法和精度后缀（`f32`、`f64`）。
- 字符串、字符、布尔、`nil`。

### 注释
行注释 `//` 和块注释 `/* */`。

## 三、语法结构

### 1. 程序组织
一个 HSCLang 源文件由导入语句（`import` / `use`）和一系列声明组成。声明包括函数、任务、结构体、枚举、trait、实现、类型别名、常量。

```
import module::submodule::Item as Alias;
use other::thing;

fn main() { ... }
task MyTask { ... }
struct Point { x: f32, y: f32 }
```

### 2. 函数
函数定义与 Rust 类似，包含参数列表、返回类型和块。

```
fn add(a: i32, b: i32) -> i32 { a + b }
```

### 3. 任务（Task）
任务是 HSCLang 的核心抽象，代表一个可在异构设备上执行的**计算单元**。任务定义包含可选的模式（`pattern`）、策略（`policy`）和主体（`body`）。

```
task MyTask : TaskType {
    pattern: ParallelPattern { kind: For, ... }
    policy: SchedulePolicy { granularity: Coarse, priority: High }
    body(params: Params) -> Result {
        // 计算逻辑
    }
}
```

- **pattern**：描述任务的并行模式，如 `For`、`Reduce`、`Scan`、`TaskGraph`，可附带参数（如是否独立迭代）。
- **policy**：调度策略，指定设备提示、工作粒度（Fine/Coarse/Adaptive）、优先级等。
- **body**：实际执行的函数体，可调用其他任务或函数。

### 4. 语句与表达式
语句包括变量声明、赋值、返回、控制流（if/while/loop/break/continue）、`match`、表达式语句、以及异构特有的 **spawn**、**pipeline**、**graph**。

**spawn 语句**：启动一个任务，可指定设备，并可选等待（`.await`）。

```
spawn on GPU::device0 my_task(args).await;
```

**pipeline 语句**：定义多阶段流水线，每个阶段可指定设备、输入输出。

```
pipeline ImageProc {
    stage FPGA::preprocess(raw) -> filtered;
    stage GPU::detect_objects(filtered) -> boxes;
    stage NPU::classify(boxes) -> labels;
}
```

**graph 语句**：定义显式的数据流图，包含节点（计算）和边（数据依赖）。

```
graph MyGraph {
    node A: preprocess(data);
    node B: extract_features(A);
    node C: run_npu_model(A);
    node D: fuse(B, C);
    edge A -> B, A -> C, B -> D, C -> D;
}
```

### 5. 表达式
表达式涵盖字面量、变量、二元/一元运算、函数调用、字段访问、索引、if/match/block 表达式，以及异构相关的 `place_on`、`move_to`、`await` 方法调用。

```
let result = compute(data).place_on(GPU).await;
```

## 四、类型系统

### 1. 基本类型
- 有符号整数：`i8` `i16` `i32` `i64` `i128`
- 无符号整数：`u8` `u16` `u32` `u64` `u128`
- 浮点数：`f32` `f64`
- 布尔：`bool`
- 字符：`char`
- 单元类型：`nil`（类似 Rust 的 `()`）

### 2. 复合类型
- 数组：`[T; N]`，定长
- 切片：`[T]`，动态视图
- 元组：`(T1, T2, ...)`
- 结构体（`struct`）、枚举（`enum`）
- 指针（`*T`）、引用（`&T` / `&mut T`）
- 函数类型：`fn(T1, T2) -> R`
- 任务类型：`task<T>`（表示一个返回 T 类型的任务）

### 3. 异构专用类型

#### `Buffer<T, Dims?>`
多维缓冲区，携带数据类型和可选的维度信息。它是数据在设备间传递的主要载体。

**方法**（内置）：
- `zeros(shape: [usize]) -> Buffer<T>`：创建全零缓冲区。
- `place_on(self, device: DeviceType) -> Buffer<T>`：将缓冲区关联到指定设备（数据可能尚未迁移）。
- `move_to(self, device: DeviceType) -> Buffer<T>`：将数据实际迁移到目标设备。
- `copy_to_host(self) -> HostBuffer<T>`：拷贝回主机内存。
- `shape() -> [usize]`、`len() -> usize`。

#### `DeviceType`
枚举，标识计算设备，包含带参数的变体：
- `GPU(ComputeCapability)`
- `NPU(ArchVersion)`
- `FPGA(Family)`
- `CPU`

#### `MemorySpace`
内存空间类型：`Host`、`DeviceLocal`（设备本地）、`Unified`（统一内存）、`Pinned`（页锁定内存）。

#### `ParallelPattern`、`SchedulePolicy`、`WorkGranularity`、`Priority`、`DataLayout`
用于配置任务的行为和调度。

## 五、内置功能与标准库

### 1. 设备管理
- `probe_devices() -> Vec<DeviceType>`：探测当前系统可用设备。
- `select_device(problem: Problem) -> DeviceType`：根据问题特征（如数据规模、计算模式）在运行时自动选择合适设备。

### 2. 数据 I/O
- `load_input<T>(path: String) -> Result<Buffer<T>>`
- `save_output<T>(path: String, data: Buffer<T>) -> Result<()>`

### 3. 日志宏
- `log!(format: String, ...)`：格式化日志输出。

### 4. 设备专用方法
每种设备预定义了一些典型操作：
- **GPU**：`fft`、`solve_navier_stokes`、`full_physics`
- **NPU**：`infer`、`predict_turbulence`、`surrogate`
- **FPGA**：`update_boundary`、`preprocess`、`fuse`

这些方法可直接在 `task` 中调用，例如 `task GPU::fft(data)`。

### 5. 模式语法糖
- **parallel for**：`parallel for i in 0..N { ... }`
- **reduce**：`reduce (acc, i) => acc + i over 0..N`
- **scan**：类似

这些模式会被编译器转换为底层的任务调用和调度策略。

## 六、编程模型详解

### 任务（Task）与 Spawn
任务可以像函数一样定义和调用，但通过 `spawn` 实现**异步启动**。`spawn on device` 指定设备，返回值是一个 future，可通过 `.await` 同步等待结果。

```rust
let handle = spawn on GPU::my_task(data);
let result = handle.await;
```

### 流水线（Pipeline）
流水线将计算分解为多个阶段，每个阶段可独立调度到不同设备，数据自动在阶段间流动。编译器会分析阶段间的依赖，尽可能实现重叠执行。

### 数据流图（Graph）
显式构建有向无环图（DAG），节点为任务，边为数据依赖。运行时根据依赖关系并发调度节点，适合复杂的工作流。

### 数据放置与迁移
通过 `Buffer` 的方法控制数据位置：
- `place_on` 仅设置关联设备，不立即迁移（延迟迁移优化）。
- `move_to` 强制迁移数据。
- `copy_to_host` 将设备数据拷贝回主机。

结合设备类型和内存空间，可以精细管理异构内存层次。

## 七、示例解读

### 1. 简单向量加法
```rust
fn main() {
    let a = Buffer::<f32>::zeros([1000]).place_on(GPU);
    let b = Buffer::<f32>::zeros([1000]).place_on(GPU);
    let c = task GPU::add(a, b).await;
    save_output("out.bin", c);
}
```
创建两个 GPU 上的缓冲区，启动 GPU 上的 `add` 任务（假设已实现），等待结果后保存。

### 2. CFD-AI 混合仿真
```rust
task CfdAiSimulation {
    pattern: TaskGraph { independent: false }
    policy: Adaptive { recursive_split: true }
    body(params: SimParams, field: FlowField) -> FlowField {
        while t < params.max_steps && error > params.convergence {
            let boundary_updated = task FPGA::update_boundary(field.boundary);
            let flow_updated = task GPU::solve_navier_stokes(field.velocity, boundary_updated);
            let turb_predicted = task NPU::predict_turbulence(field.turbulence, flow_updated);
            error = task estimate_error(flow_updated, turb_predicted);
            field.velocity = flow_updated;
            field.turbulence = turb_predicted;
            t += 1;
        }
        field
    }
}
```
定义一个任务，其主体是一个迭代循环，每次迭代并发执行三个子任务（FPGA 边界更新、GPU 流场求解、NPU 湍流预测），然后计算误差。任务的 pattern 指定为任务图（可能允许子任务间的依赖优化），policy 使用自适应拆分。

### 3. 流水线图像处理
```rust
pipeline ImageProc {
    stage FPGA::preprocess(raw) -> filtered;
    stage GPU::detect_objects(filtered) -> boxes;
    stage NPU::classify(boxes) -> labels;
}
```
定义三级流水线，各阶段在不同设备上执行，数据依次流过。

### 4. 显式数据流图
```rust
graph MyGraph {
    node A: preprocess(data);
    node B: extract_features(A);
    node C: run_npu_model(A);
    node D: fuse(B, C);
    edge A -> B, A -> C, B -> D, C -> D;
}
```
节点 A 的结果同时流向 B 和 C，B 和 C 的结果汇聚到 D。运行时 B 和 C 可并行执行。

## 八、总结

HSCLang 通过将**异构设备抽象**、**任务并行模式**、**显式数据流**和**内存放置**融入语言设计，为开发面向 FPGA/NPU/GPU 的仿真应用提供了高层次的抽象，同时保留了底层调优的灵活性。其语法借鉴了 Rust 和现代函数式语言的特点，但核心亮点在于对异构计算的一等支持。随着版本演进，预计会加入更丰富的设备库、自动调度优化以及形式化验证支持。