#ifndef TAKANAWA_H
#define TAKANAWA_H

#pragma once

#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * ABI version expected by all C-facing configuration structs.
 */
#define TKNW_ABI_VERSION 1

/**
 * Status codes returned by the C ABI.
 */
enum TknwStatus
#ifdef __cplusplus
  : int32_t
#endif // __cplusplus
 {
  /**
   * Operation completed successfully.
   */
  TKNW_STATUS_OK = 0,
  /**
   * Caller-provided output buffer was too small.
   */
  TKNW_STATUS_BUFFER_TOO_SMALL = 1,
  /**
   * A required pointer argument was null.
   */
  TKNW_STATUS_NULL_POINTER = -1,
  /**
   * ABI version or struct size did not match this library.
   */
  TKNW_STATUS_ABI_MISMATCH = -2,
  /**
   * Configuration was invalid.
   */
  TKNW_STATUS_INVALID_CONFIG = -3,
  /**
   * The global runtime has not been initialized.
   */
  TKNW_STATUS_RUNTIME_NOT_INITIALIZED = -4,
  /**
   * The final target file already exists.
   */
  TKNW_STATUS_TARGET_EXISTS = -10,
  /**
   * The part file is locked by another process or handle.
   */
  TKNW_STATUS_PART_BUSY = -11,
  /**
   * Existing part-file size did not match expected metadata.
   */
  TKNW_STATUS_PART_SIZE_MISMATCH = -12,
  /**
   * Stored part metadata is corrupt.
   */
  TKNW_STATUS_PART_CORRUPT = -13,
  /**
   * Remote validators or size changed while resuming.
   */
  TKNW_STATUS_REMOTE_CHANGED = -14,
  /**
   * HTTP response did not satisfy range download requirements.
   */
  TKNW_STATUS_HTTP_PROTOCOL = -20,
  /**
   * Network transport failed.
   */
  TKNW_STATUS_NETWORK = -21,
  /**
   * Filesystem I/O failed.
   */
  TKNW_STATUS_IO = -30,
  /**
   * Downloaded bytes did not match the configured hash.
   */
  TKNW_STATUS_HASH_MISMATCH = -40,
  /**
   * Download was cancelled.
   */
  TKNW_STATUS_CANCELLED = -50,
  /**
   * Download was already running.
   */
  TKNW_STATUS_ALREADY_STARTED = -51,
  /**
   * A panic was caught at the FFI boundary.
   */
  TKNW_STATUS_PANIC = -100,
  /**
   * Internal task or FFI boundary failure.
   */
  TKNW_STATUS_INTERNAL = -101,
};
#ifndef __cplusplus
typedef int32_t TknwStatus;
#endif // __cplusplus

/**
 * Download lifecycle phases reported in snapshot `phase` fields.
 */
enum TknwDownloadPhase
#ifdef __cplusplus
  : uint32_t
#endif // __cplusplus
 {
  /**
   * The download handle has been created but not started.
   */
  TKNW_DOWNLOAD_PHASE_CREATED = 0,
  /**
   * The download is actively fetching, writing, or finalizing.
   */
  TKNW_DOWNLOAD_PHASE_RUNNING = 1,
  /**
   * The download is paused and can be started again.
   */
  TKNW_DOWNLOAD_PHASE_PAUSED = 2,
  /**
   * The download has been cancelled.
   */
  TKNW_DOWNLOAD_PHASE_CANCELLED = 3,
  /**
   * The download finished successfully.
   */
  TKNW_DOWNLOAD_PHASE_COMPLETED = 4,
  /**
   * The download failed.
   */
  TKNW_DOWNLOAD_PHASE_FAILED = 5,
  /**
   * A pause was requested and in-flight work is winding down.
   */
  TKNW_DOWNLOAD_PHASE_PAUSING = 6,
  /**
   * A cancellation was requested and in-flight work is winding down.
   */
  TKNW_DOWNLOAD_PHASE_CANCELLING = 7,
  /**
   * A start or resume request was accepted and background work is starting.
   */
  TKNW_DOWNLOAD_PHASE_STARTING = 8,
  /**
   * The download is opening, validating, or allocating its part file.
   */
  TKNW_DOWNLOAD_PHASE_ALLOCATING = 9,
  /**
   * The completed part file is being verified before promotion.
   */
  TKNW_DOWNLOAD_PHASE_VERIFYING = 10,
};
#ifndef __cplusplus
typedef uint32_t TknwDownloadPhase;
#endif // __cplusplus

/**
 * Opaque download handle owned by the C ABI caller.
 */
typedef struct TknwDownload TknwDownload;

/**
 * Global runtime configuration for [`tknw_global_init`].
 */
typedef struct TknwGlobalConfig {
  /**
   * Must be [`TKNW_ABI_VERSION`].
   */
  uint32_t abi_version;
  /**
   * Must be at least `size_of::<TknwGlobalConfig>()`.
   */
  size_t struct_size;
  /**
   * Maximum in-flight I/O operations. `0` selects the default.
   */
  size_t max_io;
} TknwGlobalConfig;

/**
 * Download creation configuration for [`tknw_download_create`].
 */
typedef struct TknwDownloadConfig {
  /**
   * Must be [`TKNW_ABI_VERSION`].
   */
  uint32_t abi_version;
  /**
   * Must be at least `size_of::<TknwDownloadConfig>()`.
   */
  size_t struct_size;
  /**
   * Null-terminated source URL string.
   */
  const char *url;
  /**
   * Null-terminated final target path string.
   */
  const char *target_path;
  /**
   * Requested chunk size in bytes. `0` selects the default.
   */
  uint64_t chunk_size;
  /**
   * Requested chunk parallelism. `0` lets the engine choose a default.
   */
  size_t parallelism;
  /**
   * Maximum chunks to download at the same time. `0` falls back to `parallelism`.
   */
  size_t max_parallel_chunks;
  /**
   * Number of retries after the first attempt.
   */
  uint32_t max_retries;
  /**
   * Initial retry backoff in milliseconds. `0` selects the default.
   */
  uint64_t backoff_initial_millis;
  /**
   * Maximum retry backoff in milliseconds. `0` selects the default.
   */
  uint64_t backoff_max_millis;
  /**
   * Connection timeout in milliseconds. `0` selects the default.
   */
  uint64_t connect_timeout_millis;
  /**
   * Per-read timeout in milliseconds. `0` disables this timeout.
   */
  uint64_t read_timeout_millis;
  /**
   * Total timeout per probe/chunk attempt in milliseconds. `0` disables this timeout.
   */
  uint64_t total_timeout_millis;
  /**
   * Aggregate response-body bandwidth limit in bytes per second. `0` disables limiting.
   */
  uint64_t bytes_per_second_limit;
  /**
   * Hash algorithm identifier from [`TknwHashKind`].
   */
  uint32_t hash_kind;
  /**
   * Pointer to expected hash bytes for the configured hash algorithm.
   */
  const unsigned char *expected_sha256;
  /**
   * Length of `expected_sha256` in bytes.
   */
  size_t expected_sha256_len;
} TknwDownloadConfig;

/**
 * Progress snapshot written by the C ABI.
 */
typedef struct TknwDownloadSnapshot {
  /**
   * Always [`TKNW_ABI_VERSION`] on output and required on input.
   */
  uint32_t abi_version;
  /**
   * Must be at least `size_of::<TknwDownloadSnapshot>()` on input.
   */
  size_t struct_size;
  /**
   * Current phase as a `TknwDownloadPhase` numeric value.
   */
  uint32_t phase;
  /**
   * Total content length in bytes.
   */
  uint64_t content_len;
  /**
   * Number of bytes represented by committed chunks.
   */
  uint64_t downloaded_bytes;
  /**
   * Chunk size in bytes.
   */
  uint64_t chunk_size;
  /**
   * Total chunk count.
   */
  uint64_t chunk_count;
  /**
   * Number of chunks committed complete.
   */
  uint64_t completed_chunks;
  /**
   * Current number of active I/O operations.
   */
  size_t active_io;
} TknwDownloadSnapshot;

/**
 * C callback invoked when download progress changes.
 */
typedef void (*TknwProgressCallback)(const struct TknwDownloadSnapshot *snapshot, void *context);

/**
 * C callback invoked when a progress callback context is released.
 */
typedef void (*TknwProgressCallbackRelease)(void *context);

/**
 * Download speed sample written by the C ABI.
 */
typedef struct TknwDownloadSpeedSnapshot {
  /**
   * Always [`TKNW_ABI_VERSION`] on output and required on input.
   */
  uint32_t abi_version;
  /**
   * Must be at least `size_of::<TknwDownloadSpeedSnapshot>()` on input.
   */
  size_t struct_size;
  /**
   * Current phase as a `TknwDownloadPhase` numeric value.
   */
  uint32_t phase;
  /**
   * Total content length in bytes.
   */
  uint64_t content_len;
  /**
   * Bytes represented by committed chunks plus response-body bytes observed for this task.
   */
  uint64_t received_bytes;
  /**
   * Bytes observed since the previous speed sample.
   */
  uint64_t interval_bytes;
  /**
   * Milliseconds elapsed since the previous speed sample.
   */
  uint64_t elapsed_millis;
  /**
   * Current transfer speed in bytes per second for this sample interval.
   */
  double bytes_per_second;
  /**
   * Current number of active I/O operations.
   */
  size_t active_io;
} TknwDownloadSpeedSnapshot;

/**
 * C callback invoked when download speed samples change.
 */
typedef void (*TknwSpeedCallback)(const struct TknwDownloadSpeedSnapshot *snapshot, void *context);

/**
 * C callback invoked when a speed callback context is released.
 */
typedef void (*TknwSpeedCallbackRelease)(void *context);

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/**
 * Initializes or updates the global runtime used by C ABI downloads.
 *
 * Pass a null `config` pointer to use defaults.
 *
 * # Panics
 *
 * Panics if the global runtime mutex is poisoned.
 */
TknwStatus tknw_global_init(const struct TknwGlobalConfig *config);

/**
 * Shuts down the global runtime and drops shared engine state.
 *
 * # Panics
 *
 * Panics if the global runtime mutex is poisoned.
 */
TknwStatus tknw_global_shutdown(void);

/**
 * Updates the global maximum number of in-flight I/O operations.
 *
 * # Panics
 *
 * Panics if the global runtime mutex or shared limiter mutex is poisoned.
 */
TknwStatus tknw_global_set_max_io(size_t max_io);

/**
 * Creates a download handle.
 *
 * On success, writes a non-null handle to `out_download`. Release it with
 * [`tknw_download_release`].
 *
 * # Panics
 *
 * Panics if the global runtime mutex is poisoned.
 */
TknwStatus tknw_download_create(const struct TknwDownloadConfig *config,
                                struct TknwDownload **out_download);

/**
 * Starts or resumes a download.
 *
 * # Panics
 *
 * Panics if the handle's join-handle mutex is poisoned.
 */
TknwStatus tknw_download_start(struct TknwDownload *download);

/**
 * Requests that a download pause after in-flight work winds down.
 */
TknwStatus tknw_download_pause(struct TknwDownload *download);

/**
 * Requests cancellation of a download.
 *
 * # Panics
 *
 * Panics if the handle's join-handle mutex is poisoned.
 */
TknwStatus tknw_download_cancel(struct TknwDownload *download);

/**
 * Writes the current download snapshot to `snapshot`.
 *
 * `snapshot` must point to writable memory initialized with ABI metadata.
 *
 * # Panics
 *
 * Panics if shared progress state is poisoned.
 */
TknwStatus tknw_download_snapshot(const struct TknwDownload *download,
                                  struct TknwDownloadSnapshot *snapshot);

/**
 * Installs or removes a progress callback for a download.
 *
 * Passing `None` as `callback` removes the callback. A non-null `context` or
 * release callback requires a non-null progress callback.
 *
 * # Panics
 *
 * Panics if the last-error mutex or callback mutex is poisoned.
 */
TknwStatus tknw_download_set_progress_callback(struct TknwDownload *download,
                                               TknwProgressCallback callback,
                                               void *context,
                                               TknwProgressCallbackRelease context_release);

/**
 * Installs or removes a speed callback for a download.
 *
 * Passing `None` as `callback` removes the callback. A non-null `context` or
 * release callback requires a non-null speed callback.
 *
 * # Panics
 *
 * Panics if the last-error mutex or callback mutex is poisoned.
 */
TknwStatus tknw_download_set_speed_callback(struct TknwDownload *download,
                                            TknwSpeedCallback callback,
                                            void *context,
                                            TknwSpeedCallbackRelease context_release);

/**
 * Copies the serialized completion bitmap into `buffer`.
 *
 * Always writes the required byte count to `written`. If `buffer_len` is too
 * small, returns [`TknwStatus::BufferTooSmall`] without copying bytes.
 *
 * # Panics
 *
 * Panics if shared progress state is poisoned.
 */
TknwStatus tknw_download_copy_bitmap(const struct TknwDownload *download,
                                     unsigned char *buffer,
                                     size_t buffer_len,
                                     size_t *written);

/**
 * Copies the most recent download error message into `buffer` as a C string.
 *
 * Always writes the required byte count, including the null terminator, to
 * `written`. If `buffer_len` is too small, returns
 * [`TknwStatus::BufferTooSmall`] without copying bytes.
 *
 * # Panics
 *
 * Panics if shared progress state or the last-error mutex is poisoned.
 */
TknwStatus tknw_download_last_error(const struct TknwDownload *download,
                                    char *buffer,
                                    size_t buffer_len,
                                    size_t *written);

/**
 * Releases a download handle and sets the caller's handle pointer to null.
 */
TknwStatus tknw_download_release(struct TknwDownload **download);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  /* TAKANAWA_H */
