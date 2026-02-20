from fastapi import FastAPI, HTTPException
from .models import RenderRequest, TaskStatus
from . import task_manager

app = FastAPI(title="渲染控制层 API")

@app.post("/render", response_model=dict)
async def create_render_task(request: RenderRequest):
    """提交渲染任务"""
    task_id = await task_manager.start_render_task(request)
    return {"task_id": task_id, "message": "任务已启动"}

@app.get("/task/{task_id}", response_model=TaskStatus)
async def get_task_status(task_id: str):
    """查询任务状态"""
    status = task_manager.get_task_status(task_id)
    if status.status == "not_found":
        raise HTTPException(status_code=404, detail="任务不存在")
    return status

@app.get("/")
async def root():
    return {"message": "渲染控制层运行中"}