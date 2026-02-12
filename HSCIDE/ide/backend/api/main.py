from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel
import uvicorn

from cpp_wrapper import get_cpp_lib

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
    
if __name__ == "__main__":
    uvicorn.run(
        "main:app",
        host="0.0.0.0",
        port=8000,
        reload=True,
        log_level="info"
    )