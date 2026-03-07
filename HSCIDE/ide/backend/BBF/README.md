# BBF - HSCIDE Python 后端

> HSCIDE 的 Python 后端服务，提供核心业务逻辑支持，包括项目管理、编译服务、仿真控制和结果分析。

---

## 概述

BBF (Backend Business Functions) 是 HSCIDE 集成开发环境的 Python 后端组件，负责处理 IDE 的核心业务逻辑。它作为 HSC Studio 前端和底层服务（编译器、仿真引擎、渲染管线）之间的桥梁，提供项目管理、编译服务、仿真控制、结果分析和用户配置管理等功能。

## 架构设计

```
HSC Studio (C#/.NET 前端)
         ↓ (HTTP/WebSocket)
   Gateway (Go 网关)
         ↓ (gRPC/HTTP)
     BBF (Python 后端)
         ↓
   ┌──────┴──────┐
   ↓             ↓
HSCC 编译器   渲染管线
   ↓             ↓
CUDA/NPU/FPGA  可视化结果
```

## 目录结构

```
BBF/
├── src/
│   ├── api/              # API 接口层
│   │   ├── project.py    # 项目管理 API
│   │   ├── compile.py    # 编译服务 API
│   │   ├── simulate.py   # 仿真控制 API
│   │   └── analyze.py    # 结果分析 API
│   ├── services/         # 业务服务层
│   │   ├── compiler.py   # 编译器服务
│   │   ├── simulator.py  # 仿真器服务
│   │   ├── render.py     # 渲染服务
│   │   └── analytics.py  # 分析服务
│   ├── models/           # 数据模型
│   │   ├── project.py    # 项目模型
│   │   ├── task.py       # 任务模型
│   │   └── result.py     # 结果模型
│   ├── utils/            # 工具函数
│   │   ├── config.py     # 配置管理
│   │   ├── logging.py    # 日志管理
│   │   └── validation.py # 数据验证
│   └── main.py           # 应用入口
├── config/
│   └── settings.py       # 配置文件
├── tests/                # 单元测试
└── requirements.txt      # Python 依赖
```

## 核心功能

### 1. 项目管理
- 创建、打开、保存和删除项目
- 项目配置管理（HSCC.toml、HSCMakeList.txt）
- 源代码管理和版本控制集成

### 2. 编译服务
- 调用 HSCC 编译器编译 HSCLang 项目
- 支持多后端编译（CUDA、HIP、Triton、NPU）
- 编译错误和警告解析与展示

### 3. 仿真控制
- 启动、暂停、停止仿真任务
- 实时监控仿真进度和状态
- 仿真参数调整和热重载

### 4. 结果分析
- 仿真结果数据解析和处理
- 性能分析和可视化
- 结果比较和报告生成

### 5. 用户配置
- 用户偏好设置管理
- 设备配置和资源分配
- 插件和扩展管理

## API 接口

### RESTful API (HTTP)
- `GET /api/projects` - 获取项目列表
- `POST /api/projects` - 创建新项目
- `GET /api/projects/{id}` - 获取项目详情
- `POST /api/projects/{id}/compile` - 编译项目
- `POST /api/projects/{id}/simulate` - 启动仿真
- `GET /api/projects/{id}/results` - 获取仿真结果

### WebSocket 接口
- 实时编译进度推送
- 仿真状态更新推送
- 日志和错误信息流式传输

## 配置说明

编辑 `config/settings.py` 可配置：

```python
# 编译器设置
COMPILER_PATH = "hscc"  # HSCC 编译器路径
DEFAULT_BACKEND = "cuda"  # 默认后端

# 仿真设置
SIMULATION_TIMEOUT = 3600  # 仿真超时时间（秒）
MAX_CONCURRENT_SIMULATIONS = 3  # 最大并发仿真数

# 渲染管线设置
RENDER_PIPELINE_HOST = "localhost"
RENDER_PIPELINE_PORT = 50051

# 网关设置
GATEWAY_HOST = "localhost"
GATEWAY_PORT = 8080
```

## 启动方式

```bash
# 安装依赖
pip install -r requirements.txt

# 启动服务
python src/main.py --host 0.0.0.0 --port 8000

# 或使用生产服务器
gunicorn src.main:app --bind 0.0.0.0:8000 --workers 4
```

## 相关组件

- **Gateway**: Go 网关服务 (`ide/backend/gateway/`) - HTTP/WebSocket 网关，协议转换
- **HSC Studio**: 主 IDE 应用 (`ide/HSC Studio/`) - 用户界面，基于 .NET/C#
- **HSCC**: 编译器 (`HSCC/`) - HSCLang 编译器
- **RenderPipeline**: 渲染管线 (`RenderPipeline/`) - 仿真结果可视化

## 开发指南

### 添加新的 API
1. 在 `src/api/` 创建新的 API 模块
2. 实现业务逻辑并添加到路由
3. 添加相应的测试用例

### 添加新的服务
1. 在 `src/services/` 创建新的服务类
2. 实现服务接口并在 API 中调用
3. 配置依赖注入（如需要）

## 相关文档

- HSCIDE 主文档: `HSCIDE/README.md`
- Gateway 文档: `ide/backend/gateway/README.md`
- HSC Studio 文档: `ide/HSC Studio/README.md`
- HSCC 编译器文档: `HSCC/README.md`
