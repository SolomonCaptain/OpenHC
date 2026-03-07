# HSCIDE - HSCLang 集成开发环境

> HSCLang IDE 与渲染管线，提供代码编辑、可视化和实时渲染功能。

---

## 概述

HSCIDE 是 OpenHC 项目的集成开发环境组件，包含主 IDE、后端服务和渲染管线。目前渲染管线（gRPC 流式传输 + Vulkan 实时渲染）已实现完整功能，可流式传输仿真结果并进行可视化。主 IDE（HSC Studio）提供基本的项目管理和代码编辑功能，后端服务（Python + Go）提供业务逻辑支持和网关服务。

## 目录结构

```
HSCIDE/
├── ide/
│   ├── HSC Studio/       # 主 IDE (.slnx 解决方案)
│   └── backend/
│       ├── BBF/          # Python 后端
│       └── gateway/      # Go 网关
└── RenderPipeline/
    ├── client/
    │   ├── VulkanRenderer/  # Vulkan 渲染器客户端
    │   ├── GoDownloader/    # Go 下载器
    │   └── middleware/      # 中间件
    └── server/              # gRPC 渲染服务器 (Go)
```

## 组件详解

### HSC Studio

主 IDE 应用程序，基于 .NET：
- 代码编辑器
- 项目管理
- 调试支持
- 可视化工具

### Backend 服务

#### BBF (Python 后端)

提供后端逻辑支持。

#### Gateway (Go 网关)

API 网关服务，处理前端请求。

### RenderPipeline 渲染管线

#### Server (Go gRPC 服务)

渲染服务器，提供流式 PNG 帧传输：

```go
// 端口: 50051
// 流式传输 PNG 帧
rpc GetPNGStream(PNGRequest) returns (stream PNGChunk)
```

#### VulkanRenderer

Vulkan 渲染器客户端，用于高性能图形渲染。

#### GoDownloader

文件下载组件。

## 渲染服务 API

### GetPNGStream

流式传输 PNG 帧序列：

**请求**:
```protobuf
message PNGRequest {
    int32 start_frame = 1;  // 起始帧
    int32 end_frame = 2;    // 结束帧
}
```

**响应**:
```protobuf
message PNGChunk {
    bytes data = 1;        // PNG 数据
    int32 frame_index = 2; // 帧索引
}
```

## 构建

### HSC Studio

使用 Visual Studio 打开 `ide/HSC Studio/HSC Studio.slnx`。

### 渲染服务器

```bash
cd HSCIDE/RenderPipeline/server
go build -o render-server
./render-server
```

## 运行

### 启动渲染服务

```bash
cd HSCIDE/RenderPipeline/server
go run main.go
# 服务监听端口: 50051
```

### 启动 IDE

通过 Visual Studio 启动 HSC Studio 项目。

## 技术栈

| 组件 | 技术 |
|------|------|
| HSC Studio | .NET, C# |
| Backend/BBF | Python |
| Backend/Gateway | Go |
| RenderPipeline/server | Go, gRPC |
| VulkanRenderer | Vulkan, C++ |

## 相关文档

- 编译器: `HSCC/README.md`
- 语言设计: `HSCLang/README.md`
