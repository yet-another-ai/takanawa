export type Awaitable<T> = T | Promise<T>

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

export interface NormalizedHashConfig {
  kind: HashKind
  expected: string
}

export interface NormalizedDownloadOptions {
  url: string
  targetPath: string
  chunkSize?: string
  parallelism?: number
  maxParallelChunks?: number
  maxIo: number
  maxRetries?: number
  backoffInitialMs?: number
  backoffMaxMs?: number
  connectTimeoutMs?: number
  readTimeoutMs?: number
  totalTimeoutMs?: number
  bytesPerSecondLimit?: string
  hash?: NormalizedHashConfig
  sha256?: undefined
}

export interface NormalizedDownloadSnapshot {
  phase: string
  contentLen: string
  downloadedBytes: string
  chunkSize: string
  chunkCount: string
  completedChunks: string
  activeIo: number
  lastError?: string
}

export interface NormalizedDownloadSpeedSnapshot {
  phase: string
  contentLen: string
  receivedBytes: string
  intervalBytes: string
  elapsedMillis: string
  bytesPerSecond: number
  activeIo: number
}

export interface TakanawaTargetAdapter<TTask> {
  create(options: NormalizedDownloadOptions): Awaitable<TTask>
  start(task: TTask): Awaitable<void>
  pause(task: TTask): Awaitable<void>
  cancel(task: TTask): Awaitable<void>
  snapshot(task: TTask): Awaitable<NormalizedDownloadSnapshot>
  bitmap(task: TTask): Awaitable<Uint8Array>
  close(task: TTask): Awaitable<void>
  addProgressListener(
    task: TTask,
    listener: (snapshot: NormalizedDownloadSnapshot) => void
  ): Awaitable<DownloadListenerHandle>
  addSpeedListener(
    task: TTask,
    listener: (snapshot: NormalizedDownloadSpeedSnapshot) => void
  ): Awaitable<DownloadListenerHandle>
  downloadToCompletion(options: NormalizedDownloadOptions): Awaitable<NormalizedDownloadSnapshot>
}

const DEFAULT_MAX_IO = 4

export function createTakanawaApi<TTask>(adapter: TakanawaTargetAdapter<TTask>) {
  class DownloadTask {
    readonly #options: DownloadOptions
    #task?: TTask
    #taskPromise?: Promise<TTask>
    #closed = false
    readonly #listenerHandles = new Set<DownloadListenerHandle>()

    constructor(options: DownloadOptions) {
      this.#options = options
    }

    async start(): Promise<void> {
      await adapter.start(await this.#ensureTask())
    }

    async pause(): Promise<void> {
      await adapter.pause(await this.#ensureTask())
    }

    async cancel(): Promise<void> {
      await adapter.cancel(await this.#ensureTask())
    }

    async snapshot(): Promise<DownloadSnapshot> {
      return mapSnapshot(await adapter.snapshot(await this.#ensureTask()))
    }

    async bitmap(): Promise<Uint8Array> {
      return adapter.bitmap(await this.#ensureTask())
    }

    async close(): Promise<void> {
      if (this.#closed) {
        return
      }
      this.#closed = true

      const listenerHandles = [...this.#listenerHandles]
      this.#listenerHandles.clear()
      await Promise.all(listenerHandles.map((handle) => handle.remove()))

      const taskPromise = this.#taskPromise
      this.#taskPromise = undefined
      this.#task = undefined
      if (taskPromise === undefined) {
        return
      }
      await adapter.close(await taskPromise)
    }

    async addProgressListener(listener: DownloadProgressListener): Promise<DownloadListenerHandle> {
      const task = await this.#ensureTask()
      const adapterHandle = await adapter.addProgressListener(task, (snapshot) => {
        listener(mapSnapshot(snapshot))
      })
      let removed = false
      const handle = {
        remove: async () => {
          if (removed) {
            return
          }
          removed = true
          this.#listenerHandles.delete(handle)
          await adapterHandle.remove()
        }
      } satisfies DownloadListenerHandle
      this.#listenerHandles.add(handle)

      try {
        listener(await this.snapshot())
      } catch (error) {
        await handle.remove()
        throw error
      }

      return handle
    }

    async addSpeedListener(listener: DownloadSpeedListener): Promise<DownloadListenerHandle> {
      const task = await this.#ensureTask()
      const adapterHandle = await adapter.addSpeedListener(task, (snapshot) => {
        listener(mapSpeedSnapshot(snapshot))
      })
      let removed = false
      const handle = {
        remove: async () => {
          if (removed) {
            return
          }
          removed = true
          this.#listenerHandles.delete(handle)
          await adapterHandle.remove()
        }
      } satisfies DownloadListenerHandle
      this.#listenerHandles.add(handle)

      return handle
    }

    async #ensureTask(): Promise<TTask> {
      if (this.#closed) {
        throw new Error('download task is closed')
      }
      if (this.#task !== undefined) {
        return this.#task
      }
      if (this.#taskPromise === undefined) {
        this.#taskPromise = Promise.resolve(adapter.create(normalizeOptions(this.#options))).then(
          (task) => {
            this.#task = task
            return task
          },
          (error: unknown) => {
            this.#taskPromise = undefined
            throw error
          }
        )
      }
      return this.#taskPromise
    }
  }

  async function downloadToCompletion(options: DownloadOptions): Promise<DownloadSnapshot> {
    return mapSnapshot(await adapter.downloadToCompletion(normalizeOptions(options)))
  }

  return { DownloadTask, downloadToCompletion }
}

export function normalizeOptions(options: DownloadOptions): NormalizedDownloadOptions {
  return {
    url: options.url,
    targetPath: options.targetPath,
    chunkSize: normalizeOptionalU64(options.chunkSize, 'chunkSize'),
    parallelism: options.parallelism,
    maxParallelChunks: options.maxParallelChunks,
    maxIo: normalizeMaxIo(options.maxIo),
    maxRetries: options.maxRetries,
    backoffInitialMs: options.backoffInitialMs,
    backoffMaxMs: options.backoffMaxMs,
    connectTimeoutMs: options.connectTimeoutMs,
    readTimeoutMs: options.readTimeoutMs,
    totalTimeoutMs: options.totalTimeoutMs,
    bytesPerSecondLimit: normalizeOptionalU64(options.bytesPerSecondLimit, 'bytesPerSecondLimit'),
    hash: normalizeHash(options),
    sha256: undefined
  }
}

export function normalizeHash(options: DownloadOptions): NormalizedHashConfig | undefined {
  if (options.hash !== undefined && options.sha256 !== undefined) {
    throw new TypeError('use either hash or sha256, not both')
  }
  if (options.hash === undefined) {
    return options.sha256 === undefined ? undefined : { kind: 'sha256', expected: options.sha256 }
  }
  if (typeof options.hash === 'string') {
    const separator = options.hash.indexOf(':')
    if (separator === -1) {
      throw new TypeError('hash string must use the format "kind:hex"')
    }
    return {
      kind: normalizeHashKind(options.hash.slice(0, separator)),
      expected: options.hash.slice(separator + 1)
    }
  }
  return {
    kind: normalizeHashKind(options.hash.kind),
    expected: options.hash.expected
  }
}

export function normalizeHashKind(kind: string): HashKind {
  switch (kind.toLowerCase()) {
    case 'sha1':
    case 'sha-1':
      return 'sha1'
    case 'sha256':
    case 'sha-256':
      return 'sha256'
    case 'sha512':
    case 'sha-512':
      return 'sha512'
    case 'md5':
      return 'md5'
    case 'crc32':
    case 'crc-32':
      return 'crc32'
    default:
      throw new TypeError(`unsupported hash kind: ${kind}`)
  }
}

export function mapSnapshot(snapshot: NormalizedDownloadSnapshot): DownloadSnapshot {
  return {
    phase: snapshot.phase as DownloadPhase,
    contentLen: BigInt(snapshot.contentLen),
    downloadedBytes: BigInt(snapshot.downloadedBytes),
    chunkSize: BigInt(snapshot.chunkSize),
    chunkCount: BigInt(snapshot.chunkCount),
    completedChunks: BigInt(snapshot.completedChunks),
    activeIo: snapshot.activeIo,
    lastError: snapshot.lastError
  }
}

export function mapSpeedSnapshot(snapshot: NormalizedDownloadSpeedSnapshot): DownloadSpeedSnapshot {
  return {
    phase: snapshot.phase as DownloadPhase,
    contentLen: BigInt(snapshot.contentLen),
    receivedBytes: BigInt(snapshot.receivedBytes),
    intervalBytes: BigInt(snapshot.intervalBytes),
    elapsedMillis: BigInt(snapshot.elapsedMillis),
    bytesPerSecond: snapshot.bytesPerSecond,
    activeIo: snapshot.activeIo
  }
}

export function decodeBase64ToUint8Array(data: string): Uint8Array {
  if (data.length === 0) {
    return new Uint8Array()
  }

  const bufferConstructor = (globalThis as { Buffer?: { from(data: string, encoding: 'base64'): Uint8Array } }).Buffer
  if (bufferConstructor !== undefined) {
    return Uint8Array.from(bufferConstructor.from(data, 'base64'))
  }

  const binary = globalThis.atob(data)
  const bytes = new Uint8Array(binary.length)
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index)
  }
  return bytes
}

function normalizeMaxIo(value: number | undefined): number {
  if (value === undefined) {
    return DEFAULT_MAX_IO
  }
  if (!Number.isSafeInteger(value) || value < 0) {
    throw new TypeError('maxIo must be a non-negative safe integer')
  }
  return Math.max(1, value)
}

function normalizeOptionalU64(
  value: bigint | number | string | undefined,
  fieldName: string
): string | undefined {
  if (value === undefined) {
    return undefined
  }
  if (typeof value === 'number' && !Number.isSafeInteger(value)) {
    throw new TypeError(`${fieldName} must be a safe integer number; pass a bigint or string for larger values`)
  }
  const text = value.toString()
  if (!/^\d+$/.test(text)) {
    throw new TypeError(`expected an unsigned integer string for ${fieldName}, got ${text}`)
  }
  return text
}
