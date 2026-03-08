cmake_minimum_required(VERSION 3.25)
project(hscir_tests
    LANGUAGES CXX C
)

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# 下载 Catch2
include(FetchContent)
FetchContent_Declare(
    Catch2
    GIT_REPOSITORY https://github.com/catchorg/Catch2.git
    GIT_TAG v3.5.0
)
FetchContent_MakeAvailable(Catch2)

# 测试源文件
add_executable(hscir_tests
    test_types.cpp
    test_operations.cpp
    test_builder.cpp
    test_verifier.cpp
)

target_link_libraries(hscir_tests
    PRIVATE
    hscir
    Catch2::Catch2WithMain
)

target_include_directories(hscir_tests PRIVATE
    ${CMAKE_SOURCE_DIR}/include
)

enable_testing()
include(Catch)
catch_discover_tests(hscir_tests)
