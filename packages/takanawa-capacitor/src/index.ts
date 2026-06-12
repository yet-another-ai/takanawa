import { registerPlugin } from '@capacitor/core'
import {
  createTakanawaApi,
  decodeBase64ToUint8Array,
  type DownloadListenerHandle as CoreDownloadListenerHandle,
  type DownloadOptions as CoreDownloadOptions,
  type DownloadProgressListener as CoreDownloadProgressListener,
  type DownloadSnapshot as CoreDownloadSnapshot,
  type TakanawaTargetAdapter
} from 'takanawa-js-core'

import type { TakanawaCapacitorPlugin } from './definitions'

const TakanawaCapacitor = registerPlugin<TakanawaCapacitorPlugin>('TakanawaCapacitor', {
  web: () => import('./web').then((module) => new module.TakanawaCapacitorWeb())
})

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

export type DownloadProgressListener = (snapshot: DownloadSnapshot) => void

export interface DownloadListenerHandle {
  remove(): Promise<void>
}

const capacitorAdapter: TakanawaTargetAdapter<string> = {
  async create(options) {
    const { taskId } = await TakanawaCapacitor.create(options)
    return taskId
  },
  start(taskId) {
    return TakanawaCapacitor.start({ taskId })
  },
  pause(taskId) {
    return TakanawaCapacitor.pause({ taskId })
  },
  cancel(taskId) {
    return TakanawaCapacitor.cancel({ taskId })
  },
  async snapshot(taskId) {
    const { snapshot } = await TakanawaCapacitor.snapshot({ taskId })
    return snapshot
  },
  async bitmap(taskId) {
    const { data } = await TakanawaCapacitor.bitmap({ taskId })
    return decodeBase64ToUint8Array(data)
  },
  close(taskId) {
    return TakanawaCapacitor.close({ taskId })
  },
  async addProgressListener(taskId, listener) {
    const handle = await TakanawaCapacitor.addListener('downloadProgress', (event) => {
      if (event.taskId === taskId) {
        listener(event.snapshot)
      }
    })
    return {
      remove: () => handle.remove()
    } satisfies CoreDownloadListenerHandle
  },
  async downloadToCompletion(options) {
    const { snapshot } = await TakanawaCapacitor.downloadToCompletion(options)
    return snapshot
  }
}

const capacitorApi = createTakanawaApi(capacitorAdapter)

export class DownloadTask {
  readonly #inner: InstanceType<typeof capacitorApi.DownloadTask>

  constructor(options: DownloadOptions) {
    this.#inner = new capacitorApi.DownloadTask(options as CoreDownloadOptions)
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
}

export function downloadToCompletion(options: DownloadOptions): Promise<DownloadSnapshot> {
  return capacitorApi.downloadToCompletion(options as CoreDownloadOptions) as Promise<CoreDownloadSnapshot> as Promise<DownloadSnapshot>
}
