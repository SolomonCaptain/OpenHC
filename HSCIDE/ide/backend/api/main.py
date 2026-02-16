import ctypes
import os
from typing import List

from fastapi import FastAPI, HTTPException, UploadFile, File
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel
import uvicorn

from cpp_wrapper import get_cpp_lib, safe_path

app = FastAPI(
    title="现代IDE API",
    description="一个用于modern IDE的API",
    version="0.1.0",
    docs_url="/docs",
    redoc_url="/redoc",
)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# 数据模型
class HelloResponse(BaseModel):
    message: str
    source: str
    
# 初始化C++库
try:
    cpp_lib = get_cpp_lib()
    cpp_available = True
except Exception as e:
    print(f"Failed to load C++ library: {e}")
    cpp_available = False

# 文件操作的基本目录
BASE_DIR = "../files"

# 确保基础目录存在
if not os.path.exists(BASE_DIR):
    os.mkdir(BASE_DIR)

class FileContent(BaseModel):
    content: str
    
@app.get("/")
async def root():
    return {"message": "XX API is running!"}
    
@app.get("/api/hello", response_model=HelloResponse)
async def get_hello():
    """
    获取Hello World消息
    - 如果C++库可用，从C++动态库获取
    - 否则返回Python默认消息
    """
    try:
        if cpp_available:
            message = cpp_lib.get_hello_world()
            source = "C++ Dynamic Library"
        else:
            message = "Hello World from Python (C++ library not available)"
            source = "Python Fallback"
            
        return HelloResponse(message=message, source=source)
    except Exception as e:
        raise HTTPException(
            status_code=500,
            detail=f"Error calling C++ function: {str(e)}"
        )
        
@app.get("/api/health")
async def health_check():
    """健康检查端点"""
    return {
        "status": "healthy",
        "cpp_library_available": cpp_available,
        "service": "{API_TITLE}"
    }

@app.get("/api/files", response_model=List[str])
async def list_files():
    """列出所有文件"""
    try:
        files = cpp_lib.list_files(BASE_DIR)
        return files
    except Exception as e:
        raise HTTPException(
            status_code=500,
            detail=f"无法列出所有文件: {str(e)}"
        )

@app.get("/api/files/{filename}")
async def read_file(filename: str):
    """读取文件内容"""
    path = safe_path(filename)
    if not os.path.exists(path):
        raise HTTPException(status_code=404, detail="文件不存在")
    content = cpp_lib.read_file(path)
    if not content:
        raise HTTPException(status_code=500, detail="无法读取文件内容")
    return {"filename": filename, "content": content}

@app.post("/api/files/{filename}")
async def create_or_update_file(filename: str, file_content: FileContent):
    """创建或更新文件"""
    path = safe_path(filename)
    result = cpp_lib.write_file(path, file_content.content.encode('utf-8'))
    if result != 0:
        raise HTTPException(status_code=500, detail="无法创建或更新文件")
    return {"filename": filename, "message": "文件已创建或更新"}

@app.delete("/api/files/{filename}")
async def delete_file(filename: str):
    """删除文件"""
    path = safe_path(filename)
    if not os.path.exists(path):
        raise HTTPException(status_code=404, detail="文件不存在")
    result = cpp_lib.delete_file(path)
    if result != 0:
        raise HTTPException(status_code=500, detail="无法删除文件")
    return {"filename": filename, "message": "文件已删除"}

@app.post("/api/upload")
async def upload_file(file: UploadFile = File(...)):
    """上传文件"""
    filename = os.path.basename(file.filename)
    path = safe_path(filename)
    content = await file.read()
    result = cpp_lib.write_file(path, content.decode('utf-8'))
    if result != 0:
        raise HTTPException(status_code=500, detail="无法上传文件")
    return {"filename": filename, "message": "文件已上传"}
    
if __name__ == "__main__":
    uvicorn.run(
        "main:app",
        host="0.0.0.0",
        port=8000,
        reload=True,
        log_level="info"
    )