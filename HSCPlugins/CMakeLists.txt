cmake_minimum_required(VERSION 3.15)
project(HSCPlugins)

set(HSC_PLUGINS_INCLUDE_DIR ${CMAKE_CURRENT_SOURCE_DIR} CACHE PATH
    "Path to HSCPlugins include directory")

# Add plugin subdirectories
add_subdirectory(plugins/photonics_fdtd)

# Install headers
install(DIRECTORY include/
    DESTINATION include
    FILES_MATCHING PATTERN "*.h"
)
