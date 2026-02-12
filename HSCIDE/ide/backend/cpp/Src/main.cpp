// INCLUDE BEGIN
#include <string>

#include "main.h"
// INCLUDE END

// MAIN BEGIN
extern "C" {
    const char* get_hello_world() {
        // 返回静态字符串，确保内存安全
        static std::string message = "Hello World!";
        return message.c_str();
    }
}
// MAIN END