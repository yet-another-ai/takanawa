cmake_minimum_required(VERSION 3.21)

project(takanawa_cmake_package_consumer LANGUAGES CXX)

find_package(Takanawa CONFIG REQUIRED)

add_executable(takanawa_cpp_package_smoke ../cmake-integration/main.cpp)
target_link_libraries(takanawa_cpp_package_smoke PRIVATE Takanawa::takanawa)
