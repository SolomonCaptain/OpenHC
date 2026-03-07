# OpenHC 插件开发指南

## 概述

OpenHC 插件系统提供了一个模块化、可扩展的架构，用于向 OpenHC 平台添加新功能。本指南解释了如何开发与核心框架无缝集成的插件。

## 架构

```
OpenHC 框架
├── 核心主干 (HSCIR, HSCC, HSCMake, HSCLang, HSCIDE)
└── 插件系统
    ├── 插件管理器 (Rust)
    ├── 插件 API (C ABI)
    └── 领域特定接口 (光子学, CFD 等)
```

## 插件类型

| 类别 | 描述 | 示例 |
|----------|-------------|----------|
| `domain` | 领域特定仿真模块 | 光子学, CFD, FEA |
| `solver` | 数值求解器 | FDTD, FEM, FVM |
| `material` | 材料库 | 光学材料, 流体材料 |
| `visualization` | 可视化模块 | 场图, 动画 |
| `backend` | 硬件加速器 | GPU, NPU, FPGA |
| `utility` | 工具插件 | 日志记录, 性能分析 |

## 快速开始

### 1. 创建插件目录

```bash
cd HSCPlugins/plugins
mkdir my_plugin
cd my_plugin
```

### 2. 创建清单 (`plugin.toml`)

```toml
[plugin]
name = "my.plugin"
version = "0.1.0"
description = "My awesome plugin"
author = "Your Name"
license = "Apache-2.0"
category = "utility"
capabilities = ["multithreaded"]

[dependencies]

[resources]
gpu_memory_mb = 256
max_threads = 4

[extensions]
operations = ["my.plugin.operation"]
types = []
```

### 3. 实现插件接口

Create `src/main.c`:

```c
#include <plugin_api.h>

static const HscPluginInfo* my_get_info(void) {
    static HscPluginInfo info = {
        .name = "my.plugin",
        .version = "0.1.0",
        .description = "My awesome plugin",
        .category = HSC_CATEGORY_UTILITY,
    };
    return &info;
}

static HscErrorCode my_initialize(HscPluginContext* ctx, const HscHostServices* services) {
    return HSC_SUCCESS;
}

static HscErrorCode my_create_instance(const char* config, HscPluginInstance** instance) {
    *instance = calloc(1, sizeof(MyState));
    return HSC_SUCCESS;
}

static HscErrorCode my_execute(
    HscPluginInstance* instance,
    const char* operation,
    const HscValue* const* inputs,
    uint32_t num_inputs,
    HscValue** outputs,
    uint32_t num_outputs
) {
    if (strcmp(operation, "my.plugin.operation") == 0) {
        // Implement your operation
        return HSC_SUCCESS;
    }
    return HSC_ERROR_OPERATION_NOT_SUPPORTED;
}

static void my_destroy_instance(HscPluginInstance* instance) {
    free(instance);
}

static HscErrorCode my_shutdown(void) {
    return HSC_SUCCESS;
}

HSC_EXPORT_PLUGIN(
    my_get_info,
    my_initialize,
    my_create_instance,
    my_execute,
    my_destroy_instance,
    NULL,  // configure
    NULL,  // query
    my_shutdown
)
```

### 4. 构建插件

Create `CMakeLists.txt`:

```cmake
cmake_minimum_required(VERSION 3.15)
project(my_plugin C)

add_library(my_plugin SHARED src/main.c)
target_include_directories(my_plugin PRIVATE ${HSC_PLUGINS_INCLUDE_DIR}/include)
```

Build:

```bash
cmake -B build -DHSC_PLUGINS_INCLUDE_DIR=../..
cmake --build build
```

## 插件清单参考

### `[plugin]` 部分

| 字段 | 类型 | 必需 | 描述 |
|-------|------|----------|-------------|
| `name` | string | 是 | 唯一插件标识符（小写，点/下划线） |
| `version` | string | 是 | 语义版本号（例如 "0.1.0"） |
| `description` | string | 否 | 人类可读的描述 |
| `author` | string | 否 | 作者名称或组织 |
| `license` | string | 否 | SPDX 许可证标识符 |
| `homepage` | string | 否 | 项目主页 URL |
| `category` | string | 否 | 插件类别 |
| `capabilities` | [string] | 否 | 能力标志 |

### `[[dependencies]]` 部分

| 字段 | 类型 | 必需 | 描述 |
|-------|------|----------|-------------|
| `name` | string | 是 | 依赖插件名称 |
| `version` | string | 否 | 版本要求（语义版本范围） |
| `optional` | bool | 否 | 依赖是否可选 |

### `[resources]` 部分

| 字段 | 类型 | 描述 |
|-------|------|-------------|
| `gpu_memory_mb` | int | 所需的 GPU 内存（MB） |
| `system_memory_mb` | int | 所需的系统内存（MB） |
| `max_threads` | int | 最大使用的线程数 |
| `compute_units` | int | 所需的计算单元 |

### `[extensions]` 部分

| 字段 | 类型 | 描述 |
|-------|------|-------------|
| `operations` | [string] | 此插件提供的操作 |
| `types` | [string] | 此插件提供的类型 |

## 插件 API 参考

### 错误代码

| 代码 | 名称 | 描述 |
|------|------|-------------|
| 0 | `HSC_SUCCESS` | 操作成功 |
| -1 | `HSC_ERROR_UNKNOWN` | 未知错误 |
| -2 | `HSC_ERROR_INVALID_ARGUMENT` | 无效参数 |
| -3 | `HSC_ERROR_OUT_OF_MEMORY` | 内存不足 |
| -4 | `HSC_ERROR_NOT_INITIALIZED` | 插件未初始化 |
| -5 | `HSC_ERROR_ALREADY_INITIALIZED` | 已初始化 |
| -6 | `HSC_ERROR_OPERATION_NOT_SUPPORTED` | 不支持的操作 |
| -7 | `HSC_ERROR_DEPENDENCY_MISSING` | 缺少依赖 |
| -8 | `HSC_ERROR_VERSION_MISMATCH` | 版本不匹配 |
| -9 | `HSC_ERROR_RESOURCE_EXHAUSTED` | 资源耗尽 |
| -10 | `HSC_ERROR_TIMEOUT` | 操作超时 |
| -11 | `HSC_ERROR_INTERNAL` | 内部错误 |

### 主机服务

插件在初始化期间接收主机服务：

```c
typedef struct HscHostServices {
    HscLogFunc log;           // 日志记录函数
    HscAllocFunc alloc;       // 内存分配
    HscDeallocFunc dealloc;   // 内存释放
    HscGetPluginFunc get_plugin; // 获取另一个插件
    HscGetTypeFunc get_type;  // 获取 HSCIR 类型
    HscCreateValueFunc create_value; // 创建 HSCIR 值
} HscHostServices;
```

## 光子学插件开发

### 领域接口

光子学插件应实现 `HscPhotonicsInterface`：

```c
typedef struct HscPhotonicsInterface {
    HscPluginEntry base;
    
    // Domain creation
    HscErrorCode (*create_domain)(...);
    
    // Material management
    HscErrorCode (*add_material)(...);
    HscErrorCode (*set_material_region)(...);
    
    // Source/monitor management
    HscErrorCode (*add_source)(...);
    HscErrorCode (*add_monitor)(...);
    
    // Simulation control
    HscErrorCode (*run_simulation)(...);
    HscErrorCode (*step_simulation)(...);
    
    // Results retrieval
    HscErrorCode (*get_field)(...);
    HscErrorCode (*get_spectrum)(...);
} HscPhotonicsInterface;
```

### FDTD 求解器示例

请参考 `plugins/photonics_fdtd/` 查看完整的 FDTD 求解器实现模板。

## 使用插件管理器

### Rust API

```rust
use hsc_plugin_manager::{PluginManager, LoadOptions};

fn main() -> anyhow::Result<()> {
    let manager = PluginManager::new();
    
    // 发现插件
    let options = LoadOptions {
        search_paths: vec!["./plugins".into()],
        ..Default::default()
    };
    
    // 加载并解决依赖关系
    let loaded = manager.load_with_dependencies(&options)?;
    
    // 初始化并使用
    for handle in &loaded {
        manager.initialize(handle.name())?;
        let instance = manager.create_instance(handle.name(), "{}")?;
        manager.execute(&instance, "my.operation", &[], 0)?;
        manager.destroy_instance(&instance)?;
    }
    
    Ok(())
}
```

### CLI 工具

```bash
# 列出插件
hsc-plugin list

# 加载插件
hsc-plugin load photonics.fdtd --init

# 显示插件信息
hsc-plugin info photonics.fdtd

# 创建新插件模板
hsc-plugin new my.solver --category solver

# 验证清单
hsc-plugin validate ./my_plugin/plugin.toml
```

## 最佳实践

### 1. 命名约定

- 使用反向域名表示法：`domain.solver.feature`
- 保持名称小写，使用点或下划线
- 要具体：使用 `photonics.fdtd.3d` 而不仅仅是 `fdtd`

### 2. 版本兼容性

- 遵循语义版本控制
- 声明最低 API 版本
- 优雅地处理版本不匹配

### 3. 资源管理

- 声明所有资源需求
- 关闭时清理所有资源
- 尽可能使用主机分配函数

### 4. 错误处理

- 返回适当的错误代码
- 返回前记录错误
- 提供有意义的错误消息

### 5. 线程安全

- 正确标记能力
- 使用适当的同步机制
- 尽可能避免全局状态

## 测试

### 单元测试

```bash
cd HSCPlugins/plugin-manager
cargo test
```

### 集成测试

```bash
# 构建测试插件
cd HSCPlugins/plugins/photonics_fdtd
cmake -B build && cmake --build build

# 使用 CLI 测试
hsc-plugin validate plugin.toml
hsc-plugin load . --init
```

## 故障排除

### 插件无法加载

1. 检查清单语法：`hsc-plugin validate plugin.toml`
2. 验证符号导出：`nm -D libmy_plugin.so | grep HSC_PLUGIN_ENTRY`
3. 检查依赖关系：`hsc-plugin deps my.plugin`

### 版本不匹配

```
Error: ApiVersionMismatch { plugin: "...", plugin_api: 10000, host_api: 10001 }
```

解决方案：更新插件以使用当前 API 或重新构建插件管理器。

### 缺少依赖

```
Error: MissingDependency { plugin: "a", dependency: "b" }
```

解决方案：先加载依赖项或使用 `load_with_dependencies()`。

## 目录结构

```
HSCPlugins/
├── include/
│   ├── plugin_api.h          # 核心 C ABI
│   └── photonics_interface.h # 光子学扩展
├── plugin-manager/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs            # 主库
│       ├── manager.rs        # 插件管理器
│       ├── registry.rs       # 插件注册表
│       └── bin/
│           └── cli.rs        # CLI 工具
└── plugins/
    └── photonics_fdtd/       # 示例插件
        ├── plugin.toml
        ├── CMakeLists.txt
        └── src/
            └── main.c
```

## 贡献指南

1. Fork 代码仓库
2. 创建功能分支
3. 按照本指南实现你的插件
4. 添加测试
5. 提交拉取请求

## 许可证

Apache License 2.0
