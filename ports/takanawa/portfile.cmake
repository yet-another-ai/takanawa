if(VCPKG_LIBRARY_LINKAGE STREQUAL "static")
  message(STATUS "Takanawa's vcpkg overlay port currently installs the Rust cdylib; switching to dynamic linkage.")
  set(VCPKG_LIBRARY_LINKAGE dynamic)
endif()

file(REAL_PATH "${CURRENT_PORT_DIR}/../.." SOURCE_PATH)

vcpkg_cmake_configure(
  SOURCE_PATH "${SOURCE_PATH}"
  OPTIONS
    -DBUILD_SHARED_LIBS=ON
    -DTAKANAWA_CARGO_PROFILE=release
)
vcpkg_cmake_install()
vcpkg_cmake_config_fixup(PACKAGE_NAME Takanawa CONFIG_PATH lib/cmake/Takanawa)
vcpkg_copy_pdbs()

file(REMOVE_RECURSE "${CURRENT_PACKAGES_DIR}/debug/include")

vcpkg_install_copyright(FILE_LIST "${SOURCE_PATH}/LICENSE")
