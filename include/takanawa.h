#ifndef TAKANAWA_H
#define TAKANAWA_H

#pragma once

#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

#define TKNW_ABI_VERSION 1

enum TknwStatus
#ifdef __cplusplus
  : int32_t
#endif // __cplusplus
 {
  TKNW_STATUS_OK = 0,
  TKNW_STATUS_BUFFER_TOO_SMALL = 1,
  TKNW_STATUS_NULL_POINTER = -1,
  TKNW_STATUS_ABI_MISMATCH = -2,
  TKNW_STATUS_INVALID_CONFIG = -3,
  TKNW_STATUS_RUNTIME_NOT_INITIALIZED = -4,
  TKNW_STATUS_TARGET_EXISTS = -10,
  TKNW_STATUS_PART_BUSY = -11,
  TKNW_STATUS_PART_SIZE_MISMATCH = -12,
  TKNW_STATUS_PART_CORRUPT = -13,
  TKNW_STATUS_REMOTE_CHANGED = -14,
  TKNW_STATUS_HTTP_PROTOCOL = -20,
  TKNW_STATUS_NETWORK = -21,
  TKNW_STATUS_IO = -30,
  TKNW_STATUS_HASH_MISMATCH = -40,
  TKNW_STATUS_CANCELLED = -50,
  TKNW_STATUS_ALREADY_STARTED = -51,
  TKNW_STATUS_PANIC = -100,
  TKNW_STATUS_INTERNAL = -101,
};
#ifndef __cplusplus
typedef int32_t TknwStatus;
#endif // __cplusplus

typedef struct TknwDownload TknwDownload;

typedef struct TknwGlobalConfig {
  uint32_t abi_version;
  size_t struct_size;
  size_t max_io;
} TknwGlobalConfig;

typedef struct TknwDownloadConfig {
  uint32_t abi_version;
  size_t struct_size;
  const char *url;
  const char *target_path;
  uint64_t chunk_size;
  size_t parallelism;
  size_t max_parallel_chunks;
  uint32_t max_retries;
  uint64_t backoff_initial_millis;
  uint64_t backoff_max_millis;
  uint64_t connect_timeout_millis;
  uint64_t read_timeout_millis;
  uint64_t total_timeout_millis;
  uint64_t bytes_per_second_limit;
  uint32_t hash_kind;
  const unsigned char *expected_sha256;
  size_t expected_sha256_len;
} TknwDownloadConfig;

typedef struct TknwDownloadSnapshot {
  uint32_t abi_version;
  size_t struct_size;
  uint32_t phase;
  uint64_t content_len;
  uint64_t downloaded_bytes;
  uint64_t chunk_size;
  uint64_t chunk_count;
  uint64_t completed_chunks;
  size_t active_io;
} TknwDownloadSnapshot;

typedef void (*TknwProgressCallback)(const struct TknwDownloadSnapshot *snapshot, void *context);

typedef void (*TknwProgressCallbackRelease)(void *context);

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

TknwStatus tknw_global_init(const struct TknwGlobalConfig *config);

TknwStatus tknw_global_shutdown(void);

TknwStatus tknw_global_set_max_io(size_t max_io);

TknwStatus tknw_download_create(const struct TknwDownloadConfig *config,
                                struct TknwDownload **out_download);

TknwStatus tknw_download_start(struct TknwDownload *download);

TknwStatus tknw_download_pause(struct TknwDownload *download);

TknwStatus tknw_download_cancel(struct TknwDownload *download);

TknwStatus tknw_download_snapshot(const struct TknwDownload *download,
                                  struct TknwDownloadSnapshot *snapshot);

TknwStatus tknw_download_set_progress_callback(struct TknwDownload *download,
                                               TknwProgressCallback callback,
                                               void *context,
                                               TknwProgressCallbackRelease context_release);

TknwStatus tknw_download_copy_bitmap(const struct TknwDownload *download,
                                     unsigned char *buffer,
                                     size_t buffer_len,
                                     size_t *written);

TknwStatus tknw_download_last_error(const struct TknwDownload *download,
                                    char *buffer,
                                    size_t buffer_len,
                                    size_t *written);

TknwStatus tknw_download_release(struct TknwDownload **download);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  /* TAKANAWA_H */
