# C and C++

Takanawa exposes a stable C ABI through `include/takanawa.h`. C and C++ users
can consume the Rust download core through CMake directly or through the local
vcpkg overlay port.

## CMake

Add Takanawa as a subdirectory when developing from a source checkout:

```cmake
cmake_minimum_required(VERSION 3.21)
project(example LANGUAGES CXX)

add_subdirectory(path/to/takanawa)

add_executable(example main.cpp)
target_link_libraries(example PRIVATE Takanawa::takanawa)
```

Install the CMake package when you want a normal `find_package` flow:

```sh
cmake -S . -B build -DCMAKE_INSTALL_PREFIX=/opt/takanawa
cmake --build build --target install
```

Consumers can then use:

```cmake
find_package(Takanawa CONFIG REQUIRED)

add_executable(example main.cpp)
target_link_libraries(example PRIVATE Takanawa::takanawa)
```

The CMake build runs Cargo for the `takanawa-ffi` crate and installs
`takanawa.h`, the native library, and `TakanawaConfig.cmake`.

## vcpkg

Use the checked-in overlay port from a local checkout:

```sh
vcpkg install takanawa --overlay-ports=/path/to/takanawa/ports
```

For manifest mode, add Takanawa to your `vcpkg.json`:

```json
{
  "dependencies": [
    "takanawa"
  ]
}
```

Then configure your project with the vcpkg toolchain and the overlay path:

```sh
cmake -S . -B build \
  -DCMAKE_TOOLCHAIN_FILE=/path/to/vcpkg/scripts/buildsystems/vcpkg.cmake \
  -DVCPKG_OVERLAY_PORTS=/path/to/takanawa/ports
```

The overlay port currently installs the dynamic C ABI library.

## Minimal C++ Usage

```cpp
#include <cstddef>
#include "takanawa.h"

int main() {
  TknwGlobalConfig config{
    TKNW_ABI_VERSION,
    sizeof(TknwGlobalConfig),
    0,
  };

  if (tknw_global_init(&config) != TKNW_STATUS_OK) {
    return 1;
  }

  tknw_global_shutdown();
  return 0;
}
```

`tknw_global_init` must be called before creating downloads. Every
`TknwDownload*` returned by `tknw_download_create` must later be released with
`tknw_download_release`.
