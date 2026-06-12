export interface NativeHashConfig {
  kind: string
  expected: string
}

export interface NativeDownloadOptions {
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
  hash?: NativeHashConfig
  sha256?: string
}

export interface NativeDownloadSnapshot {
  phase: string
  contentLen: string
  downloadedBytes: string
  chunkSize: string
  chunkCount: string
  completedChunks: string
  activeIo: number
  lastError?: string
}

export interface NativeDownloadSpeedSnapshot {
  phase: string
  contentLen: string
  receivedBytes: string
  intervalBytes: string
  elapsedMillis: string
  bytesPerSecond: number
  activeIo: number
}

export interface NativeTaskResult {
  taskId: string
}

export interface NativeSnapshotResult {
  snapshot: NativeDownloadSnapshot
}

export interface NativeBitmapResult {
  data: string
}

export interface NativeDownloadProgressEvent {
  taskId: string
  snapshot: NativeDownloadSnapshot
}

export interface NativeDownloadSpeedEvent {
  taskId: string
  snapshot: NativeDownloadSpeedSnapshot
}
