cmake_minimum_required(VERSION 3.15)
project(photonics_fdtd C)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)

# Find OpenHC plugin headers
if(NOT DEFINED HSC_PLUGINS_INCLUDE_DIR)
    get_filename_component(HSC_PLUGINS_INCLUDE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../.." ABSOLUTE)
endif()

# Build shared library
add_library(photonics_fdtd SHARED
    src/main.c
)

target_include_directories(photonics_fdtd PRIVATE
    ${HSC_PLUGINS_INCLUDE_DIR}/include
)

# Platform-specific settings
if(WIN32)
    set_target_properties(photonics_fdtd PROPERTIES
        PREFIX ""
        OUTPUT_NAME "photonics_fdtd"
        SUFFIX ".dll"
    )
elseif(APPLE)
    set_target_properties(photonics_fdtd PROPERTIES
        PREFIX "lib"
        OUTPUT_NAME "photonics_fdtd"
        SUFFIX ".dylib"
    )
else()
    set_target_properties(photonics_fdtd PROPERTIES
        PREFIX "lib"
        OUTPUT_NAME "photonics_fdtd"
        SUFFIX ".so"
    )
endif()

# Link math library on Unix
if(UNIX)
    target_link_libraries(photonics_fdtd m)
endif()

# Compiler warnings
if(MSVC)
    target_compile_options(photonics_fdtd PRIVATE /W4)
else()
    target_compile_options(photonics_fdtd PRIVATE -Wall -Wextra -pedantic)
endif()

# Install target
install(TARGETS photonics_fdtd
    LIBRARY DESTINATION lib
    RUNTIME DESTINATION bin
)

install(FILES plugin.toml
    DESTINATION lib/photonics_fdtd
)

# Copy manifest to build directory for testing
add_custom_command(TARGET photonics_fdtd POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_if_different
    ${CMAKE_CURRENT_SOURCE_DIR}/plugin.toml
    $<TARGET_FILE_DIR:photonics_fdtd>/plugin.toml
)
