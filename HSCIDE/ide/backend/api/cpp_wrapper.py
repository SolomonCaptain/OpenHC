import ctypes
import os
import platform
from typing import Optional

def safe_path(filename: str, base_dir: str = "../files") -> str:
    """防止路径遍历攻击"""
    return os.path.normpath(os.path.join(base_dir, os.path.basename(filename)))

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
        self.lib.get_hello_world.restype = ctypes.c_char_p
        self.lib.get_hello_world.argtypes = []

        self.lib.read_file.restype = ctypes.c_char_p
        self.lib.read_file.argtypes = [ctypes.c_char_p]

        self.lib.write_file.restype = ctypes.c_int
        self.lib.write_file.argtypes = [ctypes.c_char_p, ctypes.c_char_p]

        self.lib.delete_file.restype = ctypes.c_int
        self.lib.delete_file.argtypes = [ctypes.c_char_p]

        self.lib.create_file.restype = ctypes.c_int
        self.lib.create_file.argtypes = [ctypes.c_char_p]

        self.lib.list_files.restype = ctypes.POINTER(ctypes.c_char_p)
        self.lib.list_files.argtypes = [ctypes.c_char_p, ctypes.POINTER(ctypes.c_int)]

        self.lib.free_string.argtypes = [ctypes.c_char_p]
        self.lib.free_string_array.argtypes = [ctypes.POINTER(ctypes.c_char_p), ctypes.c_int]

        
    def get_hello_world(self) -> str:
        """
        调用C++函数获取Hello World字符串
        """
        result = self.lib.get_hello_world()
        return result.decode('utf-8')

    def list_files(self, directory: str) -> list[str]:
        """
        调用C++函数获取全部文件
        """
        count = ctypes.c_int()
        files_ptr = self.lib.list_files(directory.encode('utf-8'), ctypes.byref(count))
        if not files_ptr:
            return []

        # 将C字符串数组转换为Python列表
        files = []
        for i in range(count.value):
            if files_ptr[i]:
                files.append(files_ptr[i].decode('utf-8'))

        # 释放C++分配的内存
        self.lib.free_string_array(files_ptr, count.value)
        return files

    def read_file(self, filename: str) -> str:
        """
        调用C++函数读取文件内容
        """
        safe_filename = safe_path(filename)
        result = self.lib.read_file(safe_filename.encode('utf-8'))
        if not result:
            return ""
        decoded_content = result.decode('utf-8')
        self.lib.free_string(result)
        return decoded_content

    def write_file(self, filename: str, content: str) -> int:
        """
        调用C++函数写入文件内容

        Args:
            filename: 文件名
            content: 文件内容

        Returns:
            int: 0表示成功，非0表示失败
        """
        safe_filename = safe_path(filename)
        result = self.lib.write_file(safe_filename.encode('utf-8'), content.encode('utf-8'))
        return result

    def delete_file(self, filename: str) -> int:
        """
        调用C++函数删除文件

        Args:
            filename: 文件名

        Returns:
            int: 0表示成功，非0表示失败
        """
        safe_filename = safe_path(filename)
        result = self.lib.delete_file(safe_filename.encode('utf-8'))
        return result

    def create_file(self, filename: str) -> int:
        """
        调用C++函数创建空文件

        Args:
            filename: 文件名

        Returns:
            int: 0表示成功，非0表示失败
        """
        safe_filename = safe_path(filename)
        result = self.lib.create_file(safe_filename.encode('utf-8'))
        return result

    def free_string(self, c_string: ctypes.c_char_p):
        """
        释放C++分配的字符串内存
        """
        if c_string:
            self.lib.free_string(c_string)

    def free_string_array(self, string_array: ctypes.POINTER(ctypes.c_char_p), count: int):
        """
        释放C++分配的字符串数组内存
        """
        if string_array and count > 0:
            self.lib.free_string_array(string_array, count)

# 单例实例
_cpp_lib_instance = None

def get_cpp_lib() -> CppLibrary:
    """获取C++库实例（单例模式）"""
    global _cpp_lib_instance
    if _cpp_lib_instance is None:
        _cpp_lib_instance = CppLibrary()
    return _cpp_lib_instance