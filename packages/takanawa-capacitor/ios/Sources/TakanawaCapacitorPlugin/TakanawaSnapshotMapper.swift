import Capacitor
import Foundation
import Takanawa

internal func snapshotPayload(_ snapshot: DownloadSnapshot, lastError: String? = nil) -> JSObject {
  var payload = JSObject()
  payload["phase"] = snapshot.phase.jsPhase
  payload["contentLen"] = String(snapshot.contentLen)
  payload["downloadedBytes"] = String(snapshot.downloadedBytes)
  payload["chunkSize"] = String(snapshot.chunkSize)
  payload["chunkCount"] = String(snapshot.chunkCount)
  payload["completedChunks"] = String(snapshot.completedChunks)
  payload["activeIo"] = snapshot.activeIo
  if let lastError, !lastError.isEmpty {
    payload["lastError"] = lastError
  }
  return payload
}

internal func speedPayload(_ snapshot: DownloadSpeedSnapshot) -> JSObject {
  var payload = JSObject()
  payload["phase"] = snapshot.phase.jsPhase
  payload["contentLen"] = String(snapshot.contentLen)
  payload["receivedBytes"] = String(snapshot.receivedBytes)
  payload["intervalBytes"] = String(snapshot.intervalBytes)
  payload["elapsedMillis"] = String(snapshot.elapsedMillis)
  payload["bytesPerSecond"] = snapshot.bytesPerSecond
  payload["activeIo"] = snapshot.activeIo
  return payload
}

internal extension DownloadPhase {
  var jsPhase: String {
    switch self {
    case .created:
      return "created"
    case .running:
      return "running"
    case .pausing:
      return "pausing"
    case .paused:
      return "paused"
    case .cancelling:
      return "cancelling"
    case .cancelled:
      return "cancelled"
    case .completed:
      return "completed"
    case .failed:
      return "failed"
    }
  }

  var isTerminal: Bool {
    self == .completed || self == .failed || self == .cancelled
  }
}
