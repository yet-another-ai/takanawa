import Foundation
import Takanawa

let environment = ProcessInfo.processInfo.environment
guard let url = environment["TAKANAWA_TEST_URL"] else {
  throw TakanawaError.invalidConfig("TAKANAWA_TEST_URL is required")
}
guard let expected = environment["TAKANAWA_TEST_EXPECTED_BYTES"] else {
  throw TakanawaError.invalidConfig("TAKANAWA_TEST_EXPECTED_BYTES is required")
}
let expectedData = Data(expected.utf8)

try Takanawa.initialize(maxIo: 1)
defer {
  try? Takanawa.shutdown()
}
try Takanawa.setMaxIo(1)

let tempFile = FileManager.default.temporaryDirectory
  .appendingPathComponent("takanawa-swift-smoke-\(UUID().uuidString).bin")
  .path

let config = try DownloadConfig(
  url: url,
  targetPath: tempFile,
  chunkSize: 5,
  parallelism: 2,
  maxRetries: 0
)
let download = try TakanawaDownload.create(config)
defer {
  try? download.close()
  try? FileManager.default.removeItem(atPath: tempFile)
}
_ = try download.snapshot()
_ = try download.copyBitmap()
try download.start()
let snapshot = try waitForCompletion(download)
guard snapshot.contentLen == UInt64(expectedData.count) else {
  throw TakanawaError.internalError(
    "expected content length \(expectedData.count), got \(snapshot.contentLen)"
  )
}
let actualData = try Data(contentsOf: URL(fileURLWithPath: tempFile))
guard actualData == expectedData else {
  throw TakanawaError.internalError("downloaded bytes did not match expected payload")
}

func waitForCompletion(_ download: TakanawaDownload) throws -> DownloadSnapshot {
  for _ in 0..<250 {
    let snapshot = try download.snapshot()
    switch snapshot.phase {
    case .completed:
      return snapshot
    case .failed:
      let message = try download.lastError()
      throw TakanawaError.internalError("download failed: \(message)")
    default:
      Thread.sleep(forTimeInterval: 0.02)
    }
  }

  throw TakanawaError.internalError("download did not complete in time")
}
