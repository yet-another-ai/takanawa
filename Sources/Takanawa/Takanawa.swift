import Foundation

@_exported import TakanawaBinary

public enum Takanawa {
  public static func initialize(maxIo: Int = 0) throws {
    guard maxIo >= 0 else {
      throw TakanawaError.invalidConfig("maxIo must be greater than or equal to 0")
    }

    var config = TknwGlobalConfig()
    config.abi_version = UInt32(TKNW_ABI_VERSION)
    config.struct_size = MemoryLayout<TknwGlobalConfig>.stride
    config.max_io = maxIo

    try TakanawaError.check(tknw_global_init(&config))
  }

  public static func setMaxIo(_ maxIo: Int) throws {
    guard maxIo >= 0 else {
      throw TakanawaError.invalidConfig("maxIo must be greater than or equal to 0")
    }

    try TakanawaError.check(tknw_global_set_max_io(maxIo))
  }

  public static func shutdown() throws {
    try TakanawaError.check(tknw_global_shutdown())
  }
}

public enum HashKind: UInt32, Sendable {
  case none = 0
  case sha256 = 1
  case sha1 = 2
  case sha512 = 3
  case md5 = 4
  case crc32 = 5

  public var expectedLength: Int {
    switch self {
    case .none:
      return 0
    case .sha1:
      return 20
    case .sha256:
      return 32
    case .sha512:
      return 64
    case .md5:
      return 16
    case .crc32:
      return 4
    }
  }
}

public struct DownloadConfig: Sendable {
  public var url: String
  public var targetPath: String
  public var chunkSize: UInt64
  public var parallelism: Int
  public var maxParallelChunks: Int
  public var maxRetries: UInt32
  public var backoffInitialMillis: UInt64
  public var backoffMaxMillis: UInt64
  public var connectTimeoutMillis: UInt64
  public var readTimeoutMillis: UInt64
  public var totalTimeoutMillis: UInt64
  public var bytesPerSecondLimit: UInt64
  public var expectedSha256: Data?
  public var hashKind: HashKind
  public var expectedHash: Data?

  public init(
    url: String,
    targetPath: String,
    chunkSize: UInt64 = 0,
    parallelism: Int = 0,
    maxParallelChunks: Int = 0,
    maxRetries: UInt32 = 4,
    backoffInitialMillis: UInt64 = 100,
    backoffMaxMillis: UInt64 = 3_000,
    connectTimeoutMillis: UInt64 = 30_000,
    readTimeoutMillis: UInt64 = 0,
    totalTimeoutMillis: UInt64 = 0,
    bytesPerSecondLimit: UInt64 = 0,
    expectedSha256: Data? = nil,
    hashKind: HashKind? = nil,
    expectedHash: Data? = nil
  ) throws {
    guard !url.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
      throw TakanawaError.invalidConfig("url must not be blank")
    }
    guard !targetPath.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
      throw TakanawaError.invalidConfig("targetPath must not be blank")
    }
    guard parallelism >= 0 else {
      throw TakanawaError.invalidConfig("parallelism must be greater than or equal to 0")
    }
    guard maxParallelChunks >= 0 else {
      throw TakanawaError.invalidConfig("maxParallelChunks must be greater than or equal to 0")
    }
    if let expectedSha256, expectedSha256.count != HashKind.sha256.expectedLength {
      throw TakanawaError.invalidConfig("expectedSha256 must be exactly 32 bytes")
    }
    let resolvedHashKind = hashKind ?? (expectedSha256 == nil ? .none : .sha256)
    let resolvedExpectedHash = expectedHash ?? expectedSha256
    guard (resolvedHashKind == .none) == (resolvedExpectedHash == nil) else {
      throw TakanawaError.invalidConfig(
        "expectedHash must be nil when hashKind is none and non-nil otherwise"
      )
    }
    if let resolvedExpectedHash, resolvedExpectedHash.count != resolvedHashKind.expectedLength {
      throw TakanawaError.invalidConfig(
        "expectedHash for \(resolvedHashKind) must be exactly \(resolvedHashKind.expectedLength) bytes"
      )
    }

    self.url = url
    self.targetPath = targetPath
    self.chunkSize = chunkSize
    self.parallelism = parallelism
    self.maxParallelChunks = maxParallelChunks
    self.maxRetries = maxRetries
    self.backoffInitialMillis = backoffInitialMillis
    self.backoffMaxMillis = backoffMaxMillis
    self.connectTimeoutMillis = connectTimeoutMillis
    self.readTimeoutMillis = readTimeoutMillis
    self.totalTimeoutMillis = totalTimeoutMillis
    self.bytesPerSecondLimit = bytesPerSecondLimit
    self.expectedSha256 = expectedSha256
    self.hashKind = resolvedHashKind
    self.expectedHash = resolvedExpectedHash
  }
}

public final class TakanawaDownload {
  private var handle: OpaquePointer?

  private init(handle: OpaquePointer) {
    self.handle = handle
  }

  deinit {
    releaseIgnoringErrors()
  }

  public static func create(_ config: DownloadConfig) throws -> TakanawaDownload {
    var outDownload: OpaquePointer?
    let status = config.url.withCString { urlPointer in
      config.targetPath.withCString { targetPathPointer in
        config.expectedHash.withUnsafeNullableBytes { expectedHashPointer, expectedHashLen in
          var native = TknwDownloadConfig()
          native.abi_version = UInt32(TKNW_ABI_VERSION)
          native.struct_size = MemoryLayout<TknwDownloadConfig>.stride
          native.url = urlPointer
          native.target_path = targetPathPointer
          native.chunk_size = config.chunkSize
          native.parallelism = config.parallelism
          native.max_parallel_chunks = config.maxParallelChunks
          native.max_retries = config.maxRetries
          native.backoff_initial_millis = config.backoffInitialMillis
          native.backoff_max_millis = config.backoffMaxMillis
          native.connect_timeout_millis = config.connectTimeoutMillis
          native.read_timeout_millis = config.readTimeoutMillis
          native.total_timeout_millis = config.totalTimeoutMillis
          native.bytes_per_second_limit = config.bytesPerSecondLimit
          native.hash_kind = config.hashKind.rawValue
          native.expected_sha256 = expectedHashPointer
          native.expected_sha256_len = expectedHashLen

          return tknw_download_create(&native, &outDownload)
        }
      }
    }

    try TakanawaError.check(status)
    guard let outDownload else {
      throw TakanawaError.nullPointer("native download handle was not returned")
    }
    return TakanawaDownload(handle: outDownload)
  }

  public func start() throws {
    let currentHandle = try openHandle()
    try TakanawaError.check(tknw_download_start(currentHandle), handle: currentHandle)
  }

  public func pause() throws {
    let currentHandle = try openHandle()
    try TakanawaError.check(tknw_download_pause(currentHandle), handle: currentHandle)
  }

  public func cancel() throws {
    let currentHandle = try openHandle()
    try TakanawaError.check(tknw_download_cancel(currentHandle), handle: currentHandle)
  }

  public func snapshot() throws -> DownloadSnapshot {
    let currentHandle = try openHandle()
    var native = TknwDownloadSnapshot()
    native.abi_version = UInt32(TKNW_ABI_VERSION)
    native.struct_size = MemoryLayout<TknwDownloadSnapshot>.stride

    try TakanawaError.check(tknw_download_snapshot(currentHandle, &native), handle: currentHandle)
    return DownloadSnapshot(native)
  }

  public func setProgressCallback(_ callback: (@Sendable (DownloadSnapshot) -> Void)?) throws {
    let currentHandle = try openHandle()
    try clearProgressCallback(for: currentHandle)

    guard let callback else {
      return
    }

    let box = ProgressCallbackBox(callback)
    let context = Unmanaged.passRetained(box).toOpaque()
    let status = tknw_download_set_progress_callback(
      currentHandle,
      progressCallbackTrampoline,
      context,
      progressCallbackRelease
    )
    do {
      try TakanawaError.check(status, handle: currentHandle)
    } catch {
      Unmanaged<ProgressCallbackBox>.fromOpaque(context).release()
      throw error
    }
  }

  public func clearProgressCallback() throws {
    try clearProgressCallback(for: openHandle())
  }

  public func copyBitmap() throws -> Data {
    let currentHandle = try openHandle()
    var written = 0
    let sizeStatus = tknw_download_copy_bitmap(currentHandle, nil, 0, &written)
    guard takanawaStatusCode(sizeStatus) == TakanawaError.bufferTooSmallCode || written == 0 else {
      try TakanawaError.check(sizeStatus, handle: currentHandle)
      return Data()
    }
    guard written > 0 else {
      return Data()
    }

    var bitmap = Data(count: written)
    let status = bitmap.withUnsafeMutableBytes { buffer in
      tknw_download_copy_bitmap(
        currentHandle,
        buffer.bindMemory(to: UInt8.self).baseAddress,
        buffer.count,
        &written
      )
    }
    try TakanawaError.check(status, handle: currentHandle)
    return bitmap
  }

  public func lastError() throws -> String {
    try lastErrorMessage(for: openHandle()) ?? ""
  }

  public func close() throws {
    guard let currentHandle = handle else {
      return
    }
    try clearProgressCallback(for: currentHandle)
    try TakanawaError.check(tknw_download_release(&handle))
  }

  private func openHandle() throws -> OpaquePointer {
    guard let handle else {
      throw TakanawaError.invalidConfig("download is closed")
    }
    return handle
  }

  private func releaseIgnoringErrors() {
    guard let currentHandle = handle else {
      return
    }
    clearProgressCallbackIgnoringErrors(for: currentHandle)
    _ = tknw_download_release(&handle)
  }

  private func clearProgressCallback(for currentHandle: OpaquePointer) throws {
    let status = tknw_download_set_progress_callback(currentHandle, nil, nil, nil)
    try TakanawaError.check(status, handle: currentHandle)
  }

  private func clearProgressCallbackIgnoringErrors(for currentHandle: OpaquePointer) {
    _ = tknw_download_set_progress_callback(currentHandle, nil, nil, nil)
  }
}

public struct DownloadSnapshot: Sendable, Equatable {
  public var phase: DownloadPhase
  public var contentLen: UInt64
  public var downloadedBytes: UInt64
  public var chunkSize: UInt64
  public var chunkCount: UInt64
  public var completedChunks: UInt64
  public var activeIo: Int

  fileprivate init(_ native: TknwDownloadSnapshot) {
    self.phase = DownloadPhase(rawValue: native.phase) ?? .failed
    self.contentLen = native.content_len
    self.downloadedBytes = native.downloaded_bytes
    self.chunkSize = native.chunk_size
    self.chunkCount = native.chunk_count
    self.completedChunks = native.completed_chunks
    self.activeIo = native.active_io
  }
}

public enum DownloadPhase: UInt32, Sendable {
  case created = 0
  case running = 1
  case paused = 2
  case cancelled = 3
  case completed = 4
  case failed = 5
  case pausing = 6
  case cancelling = 7
}

public enum TakanawaError: Error, Sendable, Equatable, CustomStringConvertible {
  case bufferTooSmall(String? = nil)
  case nullPointer(String? = nil)
  case abiMismatch(String? = nil)
  case invalidConfig(String? = nil)
  case runtimeNotInitialized(String? = nil)
  case targetExists(String? = nil)
  case partBusy(String? = nil)
  case partSizeMismatch(String? = nil)
  case partCorrupt(String? = nil)
  case remoteChanged(String? = nil)
  case httpProtocol(String? = nil)
  case network(String? = nil)
  case io(String? = nil)
  case hashMismatch(String? = nil)
  case cancelled(String? = nil)
  case alreadyStarted(String? = nil)
  case panic(String? = nil)
  case internalError(String? = nil)
  case unknown(statusCode: Int32, message: String? = nil)

  public var statusCode: Int32 {
    switch self {
    case .bufferTooSmall:
      return Self.bufferTooSmallCode
    case .nullPointer:
      return -1
    case .abiMismatch:
      return -2
    case .invalidConfig:
      return -3
    case .runtimeNotInitialized:
      return -4
    case .targetExists:
      return -10
    case .partBusy:
      return -11
    case .partSizeMismatch:
      return -12
    case .partCorrupt:
      return -13
    case .remoteChanged:
      return -14
    case .httpProtocol:
      return -20
    case .network:
      return -21
    case .io:
      return -30
    case .hashMismatch:
      return -40
    case .cancelled:
      return -50
    case .alreadyStarted:
      return -51
    case .panic:
      return -100
    case .internalError:
      return -101
    case let .unknown(statusCode, _):
      return statusCode
    }
  }

  public var message: String? {
    switch self {
    case let .bufferTooSmall(message),
         let .nullPointer(message),
         let .abiMismatch(message),
         let .invalidConfig(message),
         let .runtimeNotInitialized(message),
         let .targetExists(message),
         let .partBusy(message),
         let .partSizeMismatch(message),
         let .partCorrupt(message),
         let .remoteChanged(message),
         let .httpProtocol(message),
         let .network(message),
         let .io(message),
         let .hashMismatch(message),
         let .cancelled(message),
         let .alreadyStarted(message),
         let .panic(message),
         let .internalError(message),
         let .unknown(_, message):
      return message
    }
  }

  public var description: String {
    if let message, !message.isEmpty {
      return message
    }
    return String(describing: self).split(separator: "(").first.map(String.init) ?? "unknown"
  }

  fileprivate static let okCode: Int32 = 0
  fileprivate static let bufferTooSmallCode: Int32 = 1

  fileprivate static func check<Status>(_ status: Status, handle: OpaquePointer? = nil) throws {
    let code = takanawaStatusCode(status)
    guard code != okCode else {
      return
    }
    throw from(statusCode: code, message: lastErrorMessage(for: handle))
  }

  private static func from(statusCode: Int32, message: String?) -> TakanawaError {
    switch statusCode {
    case bufferTooSmallCode:
      return .bufferTooSmall(message)
    case -1:
      return .nullPointer(message)
    case -2:
      return .abiMismatch(message)
    case -3:
      return .invalidConfig(message)
    case -4:
      return .runtimeNotInitialized(message)
    case -10:
      return .targetExists(message)
    case -11:
      return .partBusy(message)
    case -12:
      return .partSizeMismatch(message)
    case -13:
      return .partCorrupt(message)
    case -14:
      return .remoteChanged(message)
    case -20:
      return .httpProtocol(message)
    case -21:
      return .network(message)
    case -30:
      return .io(message)
    case -40:
      return .hashMismatch(message)
    case -50:
      return .cancelled(message)
    case -51:
      return .alreadyStarted(message)
    case -100:
      return .panic(message)
    case -101:
      return .internalError(message)
    default:
      return .unknown(statusCode: statusCode, message: message)
    }
  }
}

private func lastErrorMessage(for handle: OpaquePointer?) -> String? {
  guard let handle else {
    return nil
  }

  var written = 0
  let sizeStatus = tknw_download_last_error(handle, nil, 0, &written)
  guard takanawaStatusCode(sizeStatus) == TakanawaError.bufferTooSmallCode, written > 0 else {
    return nil
  }

  var buffer = [CChar](repeating: 0, count: written)
  let status = tknw_download_last_error(handle, &buffer, buffer.count, &written)
  guard takanawaStatusCode(status) == TakanawaError.okCode else {
    return nil
  }
  return String(cString: buffer)
}

private final class ProgressCallbackBox {
  let callback: @Sendable (DownloadSnapshot) -> Void

  init(_ callback: @Sendable @escaping (DownloadSnapshot) -> Void) {
    self.callback = callback
  }
}

private let progressCallbackTrampoline:
  @convention(c) (UnsafePointer<TknwDownloadSnapshot>?, UnsafeMutableRawPointer?) -> Void = {
    snapshotPointer,
    context in
    guard let snapshotPointer, let context else {
      return
    }

    let box = Unmanaged<ProgressCallbackBox>.fromOpaque(context).takeUnretainedValue()
    box.callback(DownloadSnapshot(snapshotPointer.pointee))
  }

private let progressCallbackRelease: @convention(c) (UnsafeMutableRawPointer?) -> Void = { context in
  guard let context else {
    return
  }

  Unmanaged<ProgressCallbackBox>.fromOpaque(context).release()
}

private func takanawaStatusCode<Status>(_ status: Status) -> Int32 {
  withUnsafeBytes(of: status) { bytes in
    bytes.load(as: Int32.self)
  }
}

private extension Optional where Wrapped == Data {
  func withUnsafeNullableBytes<R>(
    _ body: (UnsafePointer<UInt8>?, Int) -> R
  ) -> R {
    switch self {
    case let .some(data):
      return data.withUnsafeBytes { buffer in
        body(buffer.bindMemory(to: UInt8.self).baseAddress, buffer.count)
      }
    case .none:
      return body(nil, 0)
    }
  }
}
