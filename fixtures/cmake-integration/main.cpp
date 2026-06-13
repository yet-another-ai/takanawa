#include <chrono>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <iterator>
#include <iostream>
#include <string>
#include <thread>
#include <vector>

#include "takanawa.h"

namespace {

int status_code(TknwStatus status) {
  return static_cast<int>(status);
}

int require_ok(TknwStatus status) {
  return status == TKNW_STATUS_OK ? 0 : status_code(status);
}

std::filesystem::path smoke_target_path() {
  const auto suffix = std::chrono::steady_clock::now().time_since_epoch().count();
  return std::filesystem::temp_directory_path() /
         ("takanawa-cpp-smoke-" + std::to_string(suffix) + ".bin");
}

std::string read_file(const std::filesystem::path& path) {
  std::ifstream input(path, std::ios::binary);
  return std::string(std::istreambuf_iterator<char>(input), std::istreambuf_iterator<char>());
}

std::string last_error(TknwDownload* download) {
  size_t written = 0;
  auto status = tknw_download_last_error(download, nullptr, 0, &written);
  if (status != TKNW_STATUS_BUFFER_TOO_SMALL || written == 0) {
    return {};
  }
  std::vector<char> buffer(written);
  status = tknw_download_last_error(download, buffer.data(), buffer.size(), &written);
  if (status != TKNW_STATUS_OK) {
    return {};
  }
  return buffer.data();
}

int wait_for_completion(TknwDownload* download) {
  for (int attempt = 0; attempt < 250; ++attempt) {
    TknwDownloadSnapshot snapshot{
      TKNW_ABI_VERSION,
      sizeof(TknwDownloadSnapshot),
      0,
      0,
      0,
      0,
      0,
      0,
      0,
    };
    const auto status = tknw_download_snapshot(download, &snapshot);
    if (status != TKNW_STATUS_OK) {
      return status_code(status);
    }
    if (snapshot.phase == TKNW_DOWNLOAD_PHASE_COMPLETED) {
      return 0;
    }
    if (snapshot.phase == TKNW_DOWNLOAD_PHASE_FAILED) {
      std::cerr << "download failed: " << last_error(download) << "\n";
      return 101;
    }
    std::this_thread::sleep_for(std::chrono::milliseconds(20));
  }
  return 102;
}

}  // namespace

int main() {
  const char* url = std::getenv("TAKANAWA_TEST_URL");
  const char* expected = std::getenv("TAKANAWA_TEST_EXPECTED_BYTES");
  if (url == nullptr || expected == nullptr) {
    return 100;
  }

  TknwGlobalConfig config{
    TKNW_ABI_VERSION,
    sizeof(TknwGlobalConfig),
    2,
  };

  auto status = tknw_global_init(&config);
  if (const auto code = require_ok(status); code != 0) {
    return code;
  }

  const auto target = smoke_target_path();
  const auto target_string = target.string();
  TknwDownloadConfig download_config{
    TKNW_ABI_VERSION,
    sizeof(TknwDownloadConfig),
    url,
    target_string.c_str(),
    5,
    2,
    0,
    0,
    1,
    1,
    30000,
    0,
    0,
    0,
    0,
    nullptr,
    0,
  };
  TknwDownload* download = nullptr;
  status = tknw_download_create(&download_config, &download);
  if (const auto code = require_ok(status); code != 0) {
    tknw_global_shutdown();
    return code;
  }

  status = tknw_download_start(download);
  if (const auto code = require_ok(status); code != 0) {
    tknw_download_release(&download);
    tknw_global_shutdown();
    return code;
  }

  if (const auto code = wait_for_completion(download); code != 0) {
    tknw_download_release(&download);
    tknw_global_shutdown();
    return code;
  }

  const auto actual = read_file(target);
  std::filesystem::remove(target);
  status = tknw_download_release(&download);
  if (const auto code = require_ok(status); code != 0) {
    tknw_global_shutdown();
    return code;
  }
  status = tknw_global_shutdown();
  if (const auto code = require_ok(status); code != 0) {
    return code;
  }
  return actual == expected ? 0 : 103;
}
