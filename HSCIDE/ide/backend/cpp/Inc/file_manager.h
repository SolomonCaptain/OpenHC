#ifndef HSCIDE_FILE_MANAGER_H
#define HSCIDE_FILE_MANAGER_H

#include <string>
#include <vector>

extern "C" {
    // 使用extern "C"避免名字改编
    const char* read_file(const char* path);
    int write_file(const char* path, const char* content);
    int delete_file(const char* path);
    int create_file(const char* path);
    char** list_files(const char* dir, int* count);
    void free_string(char* str);
    void free_string_array(char** arr, int count);
}

#endif //HSCIDE_FILE_MANAGER_H