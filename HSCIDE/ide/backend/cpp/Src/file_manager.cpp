#include "file_manager.h"

#include <fstream>
#include <filesystem>
#include <iostream>
#include <cstring>
#include <vector>

namespace fs = std::filesystem;

// 读取文件内容，返回动态分配的C字符串（调用者需free）
const char* read_file(const char* path)
{
    std::ifstream file(path, std::ios::binary | std::ios::ate);
    if (!file.is_open()) return nullptr;
    std::streamsize size = file.tellg();
    file.seekg(0, std::ios::beg);
    char* buffer = new char[size + 1];
    if (file.read(buffer, size))
    {
        buffer[size] = '\0';
        return buffer;
    }
    delete[] buffer;
    return nullptr;
}

// 写入文件，成功返回0，失败返回-1
int write_file(const char* path, const char* content)
{
    std::ofstream file(path, std::ios::binary);
    if (!file.is_open()) return -1;
    file.write(content, std::strlen(content));
    return file.good() ? 0 : -1;
}

// 删除文件，成功返回0，失败返回-1
int delete_file(const char* path)
{
    try
    {
        if (fs::remove(path))
            return 0;
        else
            return -1;
    } catch (...)
    {
        return -1;
    }
}

// 创建空文件，成功返回0，失败返回-1
int create_file(const char* path)
{
    std::ofstream file(path);
    if (!file.is_open()) return -1;
    file.close();
    return 0;
}

// 列出目录下所有.txt文件，返回动态分配的字符串数组，并通过count返回数量
char** list_files(const char* dir, int* count)
{
    std::vector<std::string> files;
    try
    {
        for (const auto& entry : fs::directory_iterator(dir))
        {
            if (entry.is_regular_file() && entry.path().extension() == ".txt")
            {
                files.push_back(entry.path().filename().string());
            }
        }
    } catch (...)
    {
        *count = 0;
        return nullptr;
    }
    *count = files.size();
    char** arr = new char*[files.size()];
    for (size_t i = 0; i < files.size(); ++i)
    {
        arr[i] = new char[files[i].size() + 1];
        std::strcpy(arr[i], files[i].c_str());
    }
    return arr;
}

void free_string(char* str)
{
    delete[] str;
}

void free_string_array(char** arr, int count)
{
    for (int i = 0; i < count; ++i)
    {
        delete[] arr[i];
    }
    delete[] arr;
}