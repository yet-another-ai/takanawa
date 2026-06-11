import { createRequire } from 'node:module'

const require = createRequire(import.meta.url)

export interface NativeDownloadOptions {
  url: string
  target_path: string
  chunk_size?: string
  parallelism?: number
  max_parallel_chunks?: number
  max_io?: number
  max_retries?: number
  backoff_initial_ms?: number
  backoff_max_ms?: number
  connect_timeout_ms?: number
  read_timeout_ms?: number
  total_timeout_ms?: number
  bytes_per_second_limit?: string
  hash?: NativeHashConfig
  sha256?: string
}

export interface NativeHashConfig {
  kind: string
  expected: string
}

export interface NativeDownloadSnapshot {
  phase: string
  content_len: string
  downloaded_bytes: string
  chunk_size: string
  chunk_count: string
  completed_chunks: string
  active_io: number
  last_error?: string
}

export interface NativeDownloadTask {
  start(): void
  pause(): void
  cancel(): void
  snapshot(): NativeDownloadSnapshot
  bitmap(): Uint8Array
}

export interface TakanawaNativeBinding {
  nativeDownloadToCompletion(options: NativeDownloadOptions): Promise<NativeDownloadSnapshot>
  NativeDownloadTask: new (options: NativeDownloadOptions) => NativeDownloadTask
}

export const nativeBinding = require('../index.js') as TakanawaNativeBinding
