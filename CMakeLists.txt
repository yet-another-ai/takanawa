cmake_minimum_required(VERSION 3.21)

project(Takanawa VERSION 0.4.2 LANGUAGES C)

include(CMakePackageConfigHelpers)
include(GNUInstallDirs)

option(BUILD_SHARED_LIBS "Build and install Takanawa as a shared library" ON)
set(TAKANAWA_CARGO_PROFILE "release" CACHE STRING "Cargo profile used for the FFI library")
set_property(CACHE TAKANAWA_CARGO_PROFILE PROPERTY STRINGS debug release)
set(TAKANAWA_CARGO_FEATURES "" CACHE STRING "Comma-separated Cargo features for takanawa-ffi")
option(TAKANAWA_CARGO_NO_DEFAULT_FEATURES "Disable default Cargo features for takanawa-ffi" OFF)

find_program(CARGO_EXECUTABLE cargo REQUIRED)

set(_takanawa_cargo_target_dir "${CMAKE_CURRENT_BINARY_DIR}/cargo")
set(_takanawa_cargo_profile_dir "${TAKANAWA_CARGO_PROFILE}")
file(GLOB_RECURSE _takanawa_rust_sources CONFIGURE_DEPENDS
  "${CMAKE_CURRENT_SOURCE_DIR}/crates/*.rs"
  "${CMAKE_CURRENT_SOURCE_DIR}/crates/**/Cargo.toml"
)
set(_takanawa_cargo_profile_args)
if(TAKANAWA_CARGO_PROFILE STREQUAL "release")
  list(APPEND _takanawa_cargo_profile_args "--release")
elseif(NOT TAKANAWA_CARGO_PROFILE STREQUAL "debug")
  message(FATAL_ERROR "TAKANAWA_CARGO_PROFILE must be 'debug' or 'release'")
endif()

set(_takanawa_cargo_feature_args)
if(TAKANAWA_CARGO_NO_DEFAULT_FEATURES)
  list(APPEND _takanawa_cargo_feature_args "--no-default-features")
endif()
if(TAKANAWA_CARGO_FEATURES)
  list(APPEND _takanawa_cargo_feature_args "--features" "${TAKANAWA_CARGO_FEATURES}")
endif()

set(_takanawa_profile_dir "${_takanawa_cargo_target_dir}/${_takanawa_cargo_profile_dir}")

if(WIN32)
  set(_takanawa_runtime_library "${_takanawa_profile_dir}/takanawa_ffi.dll")
  if(MSVC)
    set(_takanawa_shared_implib "${_takanawa_profile_dir}/takanawa_ffi.dll.lib")
    set(_takanawa_static_library "${_takanawa_profile_dir}/takanawa_ffi.lib")
  else()
    set(_takanawa_shared_implib "${_takanawa_profile_dir}/libtakanawa_ffi.dll.a")
    set(_takanawa_static_library "${_takanawa_profile_dir}/libtakanawa_ffi.a")
  endif()
elseif(APPLE)
  set(_takanawa_runtime_library "${_takanawa_profile_dir}/libtakanawa_ffi.dylib")
  set(_takanawa_static_library "${_takanawa_profile_dir}/libtakanawa_ffi.a")
else()
  set(_takanawa_runtime_library "${_takanawa_profile_dir}/libtakanawa_ffi.so")
  set(_takanawa_static_library "${_takanawa_profile_dir}/libtakanawa_ffi.a")
endif()

if(BUILD_SHARED_LIBS)
  set(_takanawa_link_library "${_takanawa_runtime_library}")
  if(WIN32)
    set(_takanawa_link_library "${_takanawa_shared_implib}")
  endif()
else()
  set(_takanawa_link_library "${_takanawa_static_library}")
endif()

set(_takanawa_cargo_outputs "${_takanawa_link_library}")
if(WIN32 AND BUILD_SHARED_LIBS)
  list(APPEND _takanawa_cargo_outputs "${_takanawa_runtime_library}")
endif()

add_custom_command(
  OUTPUT ${_takanawa_cargo_outputs}
  COMMAND
    "${CMAKE_COMMAND}" -E env
    "CARGO_TARGET_DIR=${_takanawa_cargo_target_dir}"
    "${CARGO_EXECUTABLE}" build
    --package takanawa-ffi
    --locked
    ${_takanawa_cargo_profile_args}
    ${_takanawa_cargo_feature_args}
  WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
  DEPENDS
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.lock"
    "${CMAKE_CURRENT_SOURCE_DIR}/include/takanawa.h"
    ${_takanawa_rust_sources}
  COMMENT "Building Takanawa C ABI library with Cargo"
  VERBATIM
)

add_custom_target(takanawa_ffi_build ALL DEPENDS ${_takanawa_cargo_outputs})

add_library(takanawa UNKNOWN IMPORTED GLOBAL)
add_dependencies(takanawa takanawa_ffi_build)
set_target_properties(takanawa PROPERTIES
  IMPORTED_LOCATION "${_takanawa_link_library}"
  INTERFACE_INCLUDE_DIRECTORIES "${CMAKE_CURRENT_SOURCE_DIR}/include"
)
if(WIN32 AND BUILD_SHARED_LIBS)
  set_target_properties(takanawa PROPERTIES
    IMPORTED_IMPLIB "${_takanawa_shared_implib}"
    IMPORTED_LOCATION "${_takanawa_runtime_library}"
  )
endif()
add_library(Takanawa::takanawa ALIAS takanawa)

install(FILES "${CMAKE_CURRENT_SOURCE_DIR}/include/takanawa.h"
  DESTINATION "${CMAKE_INSTALL_INCLUDEDIR}"
)

if(WIN32 AND BUILD_SHARED_LIBS)
  install(FILES "${_takanawa_shared_implib}"
    DESTINATION "${CMAKE_INSTALL_LIBDIR}"
  )
  install(PROGRAMS "${_takanawa_runtime_library}"
    DESTINATION "${CMAKE_INSTALL_BINDIR}"
  )
else()
  install(FILES "${_takanawa_link_library}"
    DESTINATION "${CMAKE_INSTALL_LIBDIR}"
  )
endif()

configure_package_config_file(
  "${CMAKE_CURRENT_SOURCE_DIR}/cmake/TakanawaConfig.cmake.in"
  "${CMAKE_CURRENT_BINARY_DIR}/TakanawaConfig.cmake"
  INSTALL_DESTINATION "${CMAKE_INSTALL_LIBDIR}/cmake/Takanawa"
)
write_basic_package_version_file(
  "${CMAKE_CURRENT_BINARY_DIR}/TakanawaConfigVersion.cmake"
  VERSION "${PROJECT_VERSION}"
  COMPATIBILITY SameMajorVersion
)
install(FILES
  "${CMAKE_CURRENT_BINARY_DIR}/TakanawaConfig.cmake"
  "${CMAKE_CURRENT_BINARY_DIR}/TakanawaConfigVersion.cmake"
  DESTINATION "${CMAKE_INSTALL_LIBDIR}/cmake/Takanawa"
)
