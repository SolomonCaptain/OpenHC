cmake_minimum_required(VERSION 4.1)
project(hscir
    LANGUAGES CXX C
)

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_library(hscir SHARED
    src/Types.cpp
        src/Operations.cpp
        src/Builder.cpp
        src/CAPI.cpp
        src/Module.cpp
)

target_include_directories(hscir PUBLIC
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
    $<INSTALL_INTERFACE:include>
)

install(TARGETS hscir
    LIBRARY DESTINATION lib
    ARCHIVE DESTINATION lib
    RUNTIME DESTINATION bin
)

install(DIRECTORY include/hscir DESTINATION include)