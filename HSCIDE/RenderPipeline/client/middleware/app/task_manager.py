import asyncio
import subprocess
import uuid
import os
import shlex
from pathlib import Path
from typing import Dict

from .config import settings
from .models import RenderRequest, TaskStatus

# 简单内存存储
tasks: Dict[str, dict] = {}

async def start_render_task(request: RenderRequest) -> str:
    """启动渲染任务，返回 task_id"""
    task_id = str(uuid.uuid4())[:8]
    task_dir = Path(settings.BASE_TASK_DIR) / f"task_{task_id}"
    task_dir.mkdir(parents=True, exist_ok=True)

    # 生成共享内存和事件名称（使用任务 ID 确保唯一性）
    shm_name = f"Global\\RenderTask_{task_id}"
    event_ready = f"Global\\EventReady_{task_id}"
    evnt_free = f"Global\\EventFree_{task_id}"

    # 构建子进程参数
    # Go 下载器参数：--shm <共享内存名称> --event-ready <事件名称> --event-free <事件名称> --cloud <云服务> --start <开始帧> --end <结束帧>
    go_args = [
        settings.GO_DOWNLOADER_PATH,
        "--shm", shm_name,
        "--event-ready", event_ready,
        "--event-free", evnt_free,
        "--cloud", settings.CLOUD_GRPC_ADDR,
        "--start", str(request.start_frame),
        "--end", str(request.end_frame),
    ]

    # Vulkan 渲染器参数：--shm <共享内存名称> --event-ready <事件名称> --event-free <事件名称> --out <输出文件> --width <宽度> --height <高度>
    render_args = [
        settings.VULKAN_RENDERER_PATH,
        "--shm", shm_name,
        "--event-ready", event_ready,
        "--event-free", evnt_free,
        "--out", str(task_dir / request.output_filename),
        "--width", str(request.width),
        "--height", str(request.height),
    ]

    # 启动子进程（不等待完成）
    go_proc = await asyncio.create_subprocess_exec(
        *go_args,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE
    )
    render_proc = await asyncio.create_subprocess_exec(
        *render_args,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE
    )

    # 存储任务信息
    tasks[task_id] = {
        "id": task_id,
        "status": "running",
        "go_proc": go_proc,
        "render_proc": render_proc,
        "task_dir": task_dir,
        "output_path": str(task_dir / request.output_filename),
        "shm_name": shm_name,
    }

    # 后台监控子进程状态
    asyncio.create_task(monitor_task(task_id))

    return task_id

async def monitor_task(task_id: str):
    """监控子进程，更新任务状态"""
    task_info = tasks.get(task_id)
    if not task_info:
        return

    go_proc = task_info["go_proc"]
    render_proc = task_info["render_proc"]

    # 等待子进程完成
    go_ret = await go_proc.wait()
    render_ret = await render_proc.wait()

    if go_ret == 0 and render_ret == 0:
        tasks[task_id]["status"] = "completed"
    else:
        tasks[task_id]["status"] = "failed"
        # 收集错误输出
        _, go_stderr = await go_proc.communicate()
        _, render_stderr = await render_proc.communicate()
        tasks[task_id]["error"] = f"Go: {go_stderr.decode()}, Renderer: {render_stderr.decode()}"

def get_task_status(task_id: str) -> TaskStatus:
    """获取任务状态"""
    task = tasks.get(task_id)
    if not task:
        return TaskStatus(task_id=task_id, status="not_found")
    return TaskStatus(
        task_id=task_id,
        status=task["status"],
        output_path=task.get("output_path") if task["status"] == "completed" else None,
        error=task.get("error"),
    )