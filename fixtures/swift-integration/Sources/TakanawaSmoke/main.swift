import Foundation
import Takanawa

try Takanawa.initialize(maxIo: 1)
try Takanawa.setMaxIo(1)

let tempFile = FileManager.default.temporaryDirectory
  .appendingPathComponent("takanawa-swift-smoke.bin")
  .path

let config = try DownloadConfig(
  url: "https://example.com/file.bin",
  targetPath: tempFile,
  chunkSize: 0,
  parallelism: 0
)
let download = try TakanawaDownload.create(config)
_ = try download.snapshot()
_ = try download.copyBitmap()
try download.close()

try Takanawa.shutdown()
