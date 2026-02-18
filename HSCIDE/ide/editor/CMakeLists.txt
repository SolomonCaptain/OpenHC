cmake_minimum_required(VERSION 4.1)
project(editor)

set(CMAKE_CXX_STANDARD 26)

set(VULKAN_SDK_PATH "D:/Coke/VulkanSDK/1.4.341.1")
set(GLM_PATH "${CMAKE_CURRENT_SOURCE_DIR}/libs/glm")
set(GLFW_PATH "${CMAKE_CURRENT_SOURCE_DIR}/libs/glfw")

include_directories(
        ${VULKAN_SDK_PATH}/Include
        ${GLM_PATH}
        ${GLFW_PATH}/include
        ${CMAKE_CURRENT_SOURCE_DIR}/Inc
)

link_directories(
        ${VULKAN_SDK_PATH}/Lib
        ${GLFW_PATH}/cmake-build-release/src
)

add_executable(editor
        Src/main.cpp)

target_link_libraries(${PROJECT_NAME}
        ${VULKAN_SDK_PATH}/Lib/vulkan-1.lib
        ${GLFW_PATH}/cmake-build-release/src/libglfw3.a
)