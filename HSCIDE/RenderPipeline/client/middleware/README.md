# Middleware - 渲染管线中间件

> HSCIDE 渲染管线的中间件组件，提供请求处理、数据转换和通信协调功能。

---

## 概述

Middleware 是 HSCIDE 渲染管线客户端的中间件组件，位于 VulkanRenderer、GoDownloader 和 gRPC 服务器之间，负责请求处理、数据格式转换、通信协议适配和错误处理。它作为渲染管线的协调层，确保数据在组件间高效、可靠地流动。

## 架构设计

```
渲染管线数据流:
     gRPC 服务器 (Go)
           ↓ (PNG 流)
     GoDownloader (Go) ←共享内存→ Middleware (Python) ←共享内存→ VulkanRenderer (C++)
           ↓                                               ↓
     网络下载                                     Vulkan 渲染与显示
```

Middleware 使用共享内存环形缓冲区与 GoDownloader 和 VulkanRenderer 通信，通过命名事件实现线程同步。

## 目录结构

```
middleware/
├── app/
│   ├── task_manager.py     # 任务管理，启动和监控子进程
│   ├── shared_memory.py    # 共享内存管理
│   ├── event_sync.py       # 事件同步
│   └── protocol.py         # 通信协议定义
├── config/
│   └── settings.py         # 配置管理
└── tests/                  # 单元测试
```

## 核心功能

### 1. 任务管理
- 启动和协调 GoDownloader 与 VulkanRenderer 子进程
- 监控任务状态，处理异常和超时
- 资源清理和任务终止

### 2. 共享内存管理
- 创建和管理跨进程共享内存缓冲区
- 实现环形缓冲区，支持生产者-消费者模式
- 内存映射和同步机制

### 3. 事件同步
- 使用命名事件实现进程间同步
- 协调数据生产和消费节奏
- 超时和错误处理

### 4. 协议适配
- 转换 gRPC 流式数据到渲染器可用的格式
- 处理不同的图像编码和压缩格式
- 适配不同的渲染器输入要求

## 使用示例

```python
# 启动渲染任务
from app.task_manager import start_render_task
from app.protocol import RenderRequest

request = RenderRequest(
    start_frame=1,
    end_frame=100,
    output_filename="output.mp4",
    width=1920,
    height=1080
)

task_id = await start_render_task(request)
print(f"任务已启动: {task_id}")
```

## 相关组件

- **VulkanRenderer**: Vulkan 渲染器 (`client/VulkanRenderer/`) - 高性能图形渲染
- **GoDownloader**: 下载组件 (`client/GoDownloader/`) - 从 gRPC 服务器下载 PNG 流
- **Server**: gRPC 渲染服务器 (`server/`) - 提供流式 PNG 帧服务

## 配置说明

编辑 `config/settings.py` 可配置：
- 共享内存大小和名称模式
- 事件超时时间
- 子进程路径和参数
- 日志级别和输出路径

## 故障排除

### 常见问题
1. **共享内存创建失败**: 检查权限和内存大小
2. **事件同步超时**: 增加 `EVENT_TIMEOUT_MS` 设置
3. **子进程崩溃**: 查看日志文件 `logs/middleware.log`

### 日志位置
- 应用日志: `logs/middleware.log`
- 任务日志: `tasks/task_<id>/` 目录

## 相关文档

- HSCIDE 主文档: `HSCIDE/README.md`
- VulkanRenderer 文档: `client/VulkanRenderer/README.md`
- GoDownloader 文档: `client/GoDownloader/README.md`
- Server 文档: `server/README.md`
