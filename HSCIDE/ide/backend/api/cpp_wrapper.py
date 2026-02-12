import ctypes
import os
import platform
from typing import Optional

class CppLibrary:
    def __init__(self, lib_path: Optional[str] = None):
        """
        初始化C++动态库包装器
        
        Args:
            lib_path: 动态库路径，如果为None则自动检测平台
        """
        if lib_path is None:
            # 根据平台选择默认库名
            system = platform.system()
            if system == "Windows":
                lib_name = "libHSCIDE.dll"
            else:  # Linux
                lib_name = "libHSCIDE.so"
                
            # 在当前目录和上级目录查找库文件
            base_dir = os.path.dirname(os.path.abspath(__file__))
            lib_path = os.path.join(base_dir, "cpp_lib", lib_name)
            
        if not os.path.exists(lib_path):
            raise FileNotFoundError(f"C++ library not found at: {lib_path}")
        
        # 加载动态库
        self.lib = ctypes.CDLL(lib_path)
        
        # 定义函数原型
        self.lib.get.restype = ctypes.c_char_p
        self.lib.get.argtypes = []
        
    def get_hello_world(self) -> str:
        """
        调用C++函数获取Hello World字符串
        """
        result = self.lib.get()
        return result.decode('utf-8')
        
# 单例实例
_cpp_lib_instance = None

def get_cpp_lib() -> CppLibrary:
    """获取C++库实例（单例模式）"""
    global _cpp_lib_instance
    if _cpp_lib_instance is None:
        _cpp_lib_instance = CppLibrary()
    return _cpp_lib_instance