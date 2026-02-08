cmake_minimum_required(VERSION 4.1)
project(
        HSCIDE
        VERSION 1.0.0
        DESCRIPTION "HSCIDE backend"
        LANGUAGES CXX
)

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

if (NOT CMAKE_BUILD_TYPE)
    set(CMAKE_BUILD_TYPE Release)
endif ()

add_compile_options(
        -Wall
        -Wextra
        -Werror
        -Wpedantic
)

include_directories(./Inc)

add_executable(
        ${PROJECT_NAME}
        Src/main.cpp
)