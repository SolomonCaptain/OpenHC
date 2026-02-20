import os

class Config:
    CLOUD_GRPC_ADDR = os.getenv("CLOUD_GRPC_ADDR", "154.37.219.104:50051")
    GO_DOWNLOADER_PATH = r""
    VULKAN_RENDERER_PATH = r""
    BASE_TASK_DIR = os.getenv("BASE_TASK_DIR", "./tasks")

settings = Config()