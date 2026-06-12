import type { NativeDownloadOptions, NativeDownloadSnapshot, NativeHashConfig } from './definitions'

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

const DEFAULT_MAX_IO = 4

export function normalizeOptions(options: DownloadOptions): NativeDownloadOptions {
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

export function normalizeHash(options: DownloadOptions): NativeHashConfig | undefined {
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

export function mapSnapshot(snapshot: NativeDownloadSnapshot): DownloadSnapshot {
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
