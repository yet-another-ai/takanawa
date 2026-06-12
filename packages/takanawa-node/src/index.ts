import {
  createTakanawaApi,
  type DownloadListenerHandle as CoreDownloadListenerHandle,
  type DownloadOptions as CoreDownloadOptions,
  type DownloadProgressListener as CoreDownloadProgressListener,
  type DownloadSnapshot as CoreDownloadSnapshot,
  type DownloadSpeedListener as CoreDownloadSpeedListener,
  type NormalizedDownloadOptions,
  type NormalizedDownloadSnapshot,
  type NormalizedDownloadSpeedSnapshot,
  type TakanawaTargetAdapter
} from 'takanawa-js-core'

import {
  nativeBinding,
  type NativeDownloadOptions,
  type NativeDownloadSnapshot,
  type NativeDownloadSpeedSnapshot,
  type NativeDownloadTask
} from './binding'

export type DownloadPhase =
  | 'created'
  | 'running'
  | 'pausing'
  | 'paused'
  | 'cancelling'
  | 'cancelled'
  | 'completed'
  | 'failed'

export type HashKind = 'sha1' | 'sha256' | 'sha512' | 'md5' | 'crc32'

export interface HashConfig {
  kind: HashKind
  expected: string
}

export interface DownloadOptions {
  url: string
  targetPath: string
  chunkSize?: bigint | number | string
  parallelism?: number
  maxParallelChunks?: number
  maxIo?: number
  maxRetries?: number
  backoffInitialMs?: number
  backoffMaxMs?: number
  connectTimeoutMs?: number
  readTimeoutMs?: number
  totalTimeoutMs?: number
  bytesPerSecondLimit?: bigint | number | string
  hash?: HashConfig | `${HashKind}:${string}`
  /**
   * @deprecated Use `hash: { kind: 'sha256', expected: value }` instead.
   */
  sha256?: string
}

export interface DownloadSnapshot {
  phase: DownloadPhase
  contentLen: bigint
  downloadedBytes: bigint
  chunkSize: bigint
  chunkCount: bigint
  completedChunks: bigint
  activeIo: number
  lastError?: string
}

export interface DownloadSpeedSnapshot {
  phase: DownloadPhase
  contentLen: bigint
  receivedBytes: bigint
  intervalBytes: bigint
  elapsedMillis: bigint
  bytesPerSecond: number
  activeIo: number
}

export type DownloadProgressListener = (snapshot: DownloadSnapshot) => void
export type DownloadSpeedListener = (snapshot: DownloadSpeedSnapshot) => void

export interface DownloadListenerHandle {
  remove(): Promise<void>
}

const PROGRESS_POLL_INTERVAL_MS = 250

const nodeAdapter: TakanawaTargetAdapter<NativeDownloadTask> = {
  create(options) {
    return new nativeBinding.NativeDownloadTask(toNativeOptions(options))
  },
  start(task) {
    task.start()
  },
  pause(task) {
    task.pause()
  },
  cancel(task) {
    task.cancel()
  },
  snapshot(task) {
    return fromNativeSnapshot(task.snapshot())
  },
  bitmap(task) {
    return task.bitmap()
  },
  close() {},
  addProgressListener(task, listener) {
    let previous = snapshotKey(fromNativeSnapshot(task.snapshot()))
    const timer = setInterval(() => {
      const snapshot = fromNativeSnapshot(task.snapshot())
      const key = snapshotKey(snapshot)
      if (key !== previous) {
        previous = key
        listener(snapshot)
      }
    }, PROGRESS_POLL_INTERVAL_MS)

    return {
      async remove() {
        clearInterval(timer)
      }
    } satisfies CoreDownloadListenerHandle
  },
  addSpeedListener(task, listener) {
    let previous = speedSnapshotKey(fromNativeSpeedSnapshot(task.speedSnapshot()))
    const timer = setInterval(() => {
      const snapshot = fromNativeSpeedSnapshot(task.speedSnapshot())
      const key = speedSnapshotKey(snapshot)
      if (key !== previous) {
        previous = key
        listener(snapshot)
      }
    }, PROGRESS_POLL_INTERVAL_MS)

    return {
      async remove() {
        clearInterval(timer)
      }
    } satisfies CoreDownloadListenerHandle
  },
  async downloadToCompletion(options) {
    return fromNativeSnapshot(await nativeBinding.nativeDownloadToCompletion(toNativeOptions(options)))
  }
}

const nodeApi = createTakanawaApi(nodeAdapter)

export class DownloadTask {
  readonly #inner: InstanceType<typeof nodeApi.DownloadTask>

  constructor(options: DownloadOptions) {
    this.#inner = new nodeApi.DownloadTask(options as CoreDownloadOptions)
  }

  start(): Promise<void> {
    return this.#inner.start()
  }

  pause(): Promise<void> {
    return this.#inner.pause()
  }

  cancel(): Promise<void> {
    return this.#inner.cancel()
  }

  snapshot(): Promise<DownloadSnapshot> {
    return this.#inner.snapshot() as Promise<DownloadSnapshot>
  }

  bitmap(): Promise<Uint8Array> {
    return this.#inner.bitmap()
  }

  close(): Promise<void> {
    return this.#inner.close()
  }

  addProgressListener(listener: DownloadProgressListener): Promise<DownloadListenerHandle> {
    return this.#inner.addProgressListener(listener as CoreDownloadProgressListener)
  }

  addSpeedListener(listener: DownloadSpeedListener): Promise<DownloadListenerHandle> {
    return this.#inner.addSpeedListener(listener as CoreDownloadSpeedListener)
  }
}

export function downloadToCompletion(options: DownloadOptions): Promise<DownloadSnapshot> {
  return nodeApi.downloadToCompletion(options as CoreDownloadOptions) as Promise<CoreDownloadSnapshot> as Promise<DownloadSnapshot>
}

function toNativeOptions(options: NormalizedDownloadOptions): NativeDownloadOptions {
  return {
    url: options.url,
    target_path: options.targetPath,
    chunk_size: options.chunkSize,
    parallelism: options.parallelism,
    max_parallel_chunks: options.maxParallelChunks,
    max_io: options.maxIo,
    max_retries: options.maxRetries,
    backoff_initial_ms: options.backoffInitialMs,
    backoff_max_ms: options.backoffMaxMs,
    connect_timeout_ms: options.connectTimeoutMs,
    read_timeout_ms: options.readTimeoutMs,
    total_timeout_ms: options.totalTimeoutMs,
    bytes_per_second_limit: options.bytesPerSecondLimit,
    hash: options.hash,
    sha256: undefined
  }
}

function fromNativeSnapshot(snapshot: NativeDownloadSnapshot): NormalizedDownloadSnapshot {
  return {
    phase: snapshot.phase,
    contentLen: snapshot.content_len,
    downloadedBytes: snapshot.downloaded_bytes,
    chunkSize: snapshot.chunk_size,
    chunkCount: snapshot.chunk_count,
    completedChunks: snapshot.completed_chunks,
    activeIo: snapshot.active_io,
    lastError: snapshot.last_error
  }
}

function fromNativeSpeedSnapshot(snapshot: NativeDownloadSpeedSnapshot): NormalizedDownloadSpeedSnapshot {
  return {
    phase: snapshot.phase,
    contentLen: snapshot.content_len,
    receivedBytes: snapshot.received_bytes,
    intervalBytes: snapshot.interval_bytes,
    elapsedMillis: snapshot.elapsed_millis,
    bytesPerSecond: snapshot.bytes_per_second,
    activeIo: snapshot.active_io
  }
}

function snapshotKey(snapshot: NormalizedDownloadSnapshot): string {
  return [
    snapshot.phase,
    snapshot.contentLen,
    snapshot.downloadedBytes,
    snapshot.chunkSize,
    snapshot.chunkCount,
    snapshot.completedChunks,
    snapshot.activeIo,
    snapshot.lastError ?? ''
  ].join('\0')
}

function speedSnapshotKey(snapshot: NormalizedDownloadSpeedSnapshot): string {
  return [
    snapshot.phase,
    snapshot.contentLen,
    snapshot.receivedBytes,
    snapshot.intervalBytes,
    snapshot.elapsedMillis,
    snapshot.bytesPerSecond.toString(),
    snapshot.activeIo
  ].join('\0')
}
