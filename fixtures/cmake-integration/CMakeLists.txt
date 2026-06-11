cmake_minimum_required(VERSION 3.21)

project(takanawa_cmake_integration LANGUAGES CXX)

add_subdirectory("../.." takanawa-build)

add_executable(takanawa_cpp_smoke main.cpp)
target_link_libraries(takanawa_cpp_smoke PRIVATE Takanawa::takanawa)

if(WIN32 AND BUILD_SHARED_LIBS)
  add_custom_command(TARGET takanawa_cpp_smoke POST_BUILD
    COMMAND "${CMAKE_COMMAND}" -E copy_if_different
      "$<TARGET_FILE:Takanawa::takanawa>"
      "$<TARGET_FILE_DIR:takanawa_cpp_smoke>"
    VERBATIM
  )
endif()
