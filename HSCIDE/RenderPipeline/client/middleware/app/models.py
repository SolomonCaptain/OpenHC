from pydantic import BaseModel
from typing import Optional

class RenderRequest(BaseModel):
    start_frame: int = 1
    end_frame: int = 100
    output_filename: str = "output.mp4"
    width: int = 1920
    height: int = 1080

class TaskStatus(BaseModel):
    task_id: str
    status: str
    output_path: Optional[str] = None
    error: Optional[str] = None