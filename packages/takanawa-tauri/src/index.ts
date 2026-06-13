import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import {
  createTakanawaApi,
  decodeBase64ToUint8Array,
  type DownloadListenerHandle as CoreDownloadListenerHandle,
  type DownloadOptions as CoreDownloadOptions,
  type DownloadProgressListener as CoreDownloadProgressListener,
  type DownloadSnapshot as CoreDownloadSnapshot,
  type DownloadSpeedListener as CoreDownloadSpeedListener,
  type TakanawaTargetAdapter
} from 'takanawa-js-core'

import type {
  NativeBitmapResult,
  NativeDownloadOptions,
  NativeDownloadProgressEvent,
  NativeDownloadSpeedEvent,
  NativeDownloadSnapshot,
  NativeSnapshotResult,
  NativeTaskResult
} from './definitions'

export type DownloadPhase =
  | 'created'
  | 'starting'
  | 'allocating'
  | 'running'
  | 'pausing'
  | 'paused'
  | 'verifying'
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

export const TAKANAWA_PROGRESS_EVENT = 'takanawa://download-progress'
export const TAKANAWA_SPEED_EVENT = 'takanawa://download-speed'

const PLUGIN = 'takanawa'

const tauriAdapter: TakanawaTargetAdapter<string> = {
  async create(options) {
    const { taskId } = await invoke<NativeTaskResult>(command('create'), { options: options as NativeDownloadOptions })
    return taskId
  },
  start(taskId) {
    return invoke(command('start'), { taskId })
  },
  pause(taskId) {
    return invoke(command('pause'), { taskId })
  },
  cancel(taskId) {
    return invoke(command('cancel'), { taskId })
  },
  async snapshot(taskId) {
    const { snapshot } = await invoke<NativeSnapshotResult>(command('snapshot'), { taskId })
    return snapshot
  },
  async bitmap(taskId) {
    const { data } = await invoke<NativeBitmapResult>(command('bitmap'), { taskId })
    return decodeBase64ToUint8Array(data)
  },
  close(taskId) {
    return invoke(command('close'), { taskId })
  },
  async addProgressListener(taskId, listener) {
    const unlisten = await listen<NativeDownloadProgressEvent>(TAKANAWA_PROGRESS_EVENT, (event) => {
      if (event.payload.taskId === taskId) {
        listener(event.payload.snapshot)
      }
    })
    return {
      async remove() {
        unlisten()
      }
    } satisfies CoreDownloadListenerHandle
  },
  async addSpeedListener(taskId, listener) {
    const unlisten = await listen<NativeDownloadSpeedEvent>(TAKANAWA_SPEED_EVENT, (event) => {
      if (event.payload.taskId === taskId) {
        listener(event.payload.snapshot)
      }
    })
    return {
      async remove() {
        unlisten()
      }
    } satisfies CoreDownloadListenerHandle
  },
  async downloadToCompletion(options) {
    const { snapshot } = await invoke<NativeSnapshotResult>(command('download_to_completion'), {
      options: options as NativeDownloadOptions
    })
    return snapshot
  }
}

const tauriApi = createTakanawaApi(tauriAdapter)

export class DownloadTask {
  readonly #inner: InstanceType<typeof tauriApi.DownloadTask>

  constructor(options: DownloadOptions) {
    this.#inner = new tauriApi.DownloadTask(options as CoreDownloadOptions)
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
  return tauriApi.downloadToCompletion(options as CoreDownloadOptions) as Promise<CoreDownloadSnapshot> as Promise<DownloadSnapshot>
}

function command(name: string): string {
  return `plugin:${PLUGIN}|${name}`
}

export type {
  NativeBitmapResult,
  NativeDownloadOptions,
  NativeDownloadProgressEvent,
  NativeDownloadSpeedEvent,
  NativeDownloadSnapshot,
  NativeDownloadSpeedSnapshot,
  NativeHashConfig,
  NativeSnapshotResult,
  NativeTaskResult
} from './definitions'
