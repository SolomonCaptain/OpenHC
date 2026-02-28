#pragma once

#define WIN32_LEAN_AND_MEAN             // 从 Windows 头文件中排除极少使用的内容
#define NOMINMAX                       // 防止 Windows.h 中的 min/max 宏冲突

#include <windows.h>
#include <wincodec.h>                  // Windows Imaging Component
#include <wrl/client.h>                // ComPtr
#include <wrl/implements.h>
#include <vector>
#include <string>
#include <memory>
#include <fstream>
#include <sstream>
#include <cstring>
