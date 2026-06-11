import {
  nativeBinding,
  type NativeDownloadOptions,
  type NativeDownloadSnapshot,
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

export class DownloadTask {
  readonly #native: NativeDownloadTask

  constructor(options: DownloadOptions) {
    this.#native = new nativeBinding.NativeDownloadTask(normalizeOptions(options))
  }

  start(): void {
    this.#native.start()
  }

  pause(): void {
    this.#native.pause()
  }

  cancel(): void {
    this.#native.cancel()
  }

  snapshot(): DownloadSnapshot {
    return mapSnapshot(this.#native.snapshot())
  }

  bitmap(): Uint8Array {
    return this.#native.bitmap()
  }
}

export async function downloadToCompletion(options: DownloadOptions): Promise<DownloadSnapshot> {
  const snapshot = await nativeBinding.nativeDownloadToCompletion(normalizeOptions(options))
  return mapSnapshot(snapshot)
}

function normalizeOptions(options: DownloadOptions): NativeDownloadOptions {
  return {
    url: options.url,
    target_path: options.targetPath,
    chunk_size: normalizeOptionalU64(options.chunkSize),
    parallelism: options.parallelism,
    max_parallel_chunks: options.maxParallelChunks,
    max_io: options.maxIo,
    max_retries: options.maxRetries,
    backoff_initial_ms: options.backoffInitialMs,
    backoff_max_ms: options.backoffMaxMs,
    connect_timeout_ms: options.connectTimeoutMs,
    read_timeout_ms: options.readTimeoutMs,
    total_timeout_ms: options.totalTimeoutMs,
    bytes_per_second_limit: normalizeOptionalU64(options.bytesPerSecondLimit),
    hash: normalizeHash(options),
    sha256: undefined
  }
}

function normalizeHash(options: DownloadOptions): NativeDownloadOptions['hash'] {
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

function normalizeHashKind(kind: string): HashKind {
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

function normalizeOptionalU64(value: bigint | number | string | undefined): string | undefined {
  if (value === undefined) {
    return undefined
  }
  if (typeof value === 'number' && !Number.isSafeInteger(value)) {
    throw new TypeError('numeric u64 options must be safe integers; pass a bigint or string for larger values')
  }
  const text = value.toString()
  if (!/^\d+$/.test(text)) {
    throw new TypeError(`expected an unsigned integer string, got ${text}`)
  }
  return text
}

function mapSnapshot(snapshot: NativeDownloadSnapshot): DownloadSnapshot {
  return {
    phase: snapshot.phase as DownloadPhase,
    contentLen: BigInt(snapshot.content_len),
    downloadedBytes: BigInt(snapshot.downloaded_bytes),
    chunkSize: BigInt(snapshot.chunk_size),
    chunkCount: BigInt(snapshot.chunk_count),
    completedChunks: BigInt(snapshot.completed_chunks),
    activeIo: snapshot.active_io,
    lastError: snapshot.last_error
  }
}
