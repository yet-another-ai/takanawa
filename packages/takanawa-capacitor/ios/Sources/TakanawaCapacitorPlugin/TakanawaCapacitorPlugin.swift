import Capacitor
import Foundation
import Takanawa

@objc(TakanawaCapacitorPlugin)
public class TakanawaCapacitorPlugin: CAPPlugin, CAPBridgedPlugin {
  public let identifier = "TakanawaCapacitorPlugin"
  public let jsName = "TakanawaCapacitor"
  public let pluginMethods: [CAPPluginMethod] = [
    CAPPluginMethod(name: "create", returnType: CAPPluginReturnPromise),
    CAPPluginMethod(name: "start", returnType: CAPPluginReturnPromise),
    CAPPluginMethod(name: "pause", returnType: CAPPluginReturnPromise),
    CAPPluginMethod(name: "cancel", returnType: CAPPluginReturnPromise),
    CAPPluginMethod(name: "snapshot", returnType: CAPPluginReturnPromise),
    CAPPluginMethod(name: "bitmap", returnType: CAPPluginReturnPromise),
    CAPPluginMethod(name: "close", returnType: CAPPluginReturnPromise),
    CAPPluginMethod(name: "downloadToCompletion", returnType: CAPPluginReturnPromise)
  ]

  private let tasks = TakanawaTaskRegistry<TakanawaDownload>()
  private let queue = DispatchQueue(label: "ai.yetanother.takanawa.capacitor", qos: .utility, attributes: .concurrent)

  deinit {
    tasks.closeAll()
  }

  @objc public func create(_ call: CAPPluginCall) {
    runAsync(call) {
      let parsed = try TakanawaCapacitorOptions.parse(call.options)
      try Takanawa.initialize(maxIo: parsed.maxIo)
      let download = try TakanawaDownload.create(parsed.config)
      let taskId = self.tasks.insert(download)
      do {
        try download.setProgressCallback { [weak self] snapshot in
          self?.emitProgress(taskId: taskId, snapshot: snapshot)
        }
      } catch {
        try? self.tasks.close(taskId)
        throw error
      }
      return ["taskId": taskId]
    }
  }

  @objc public func start(_ call: CAPPluginCall) {
    runAsync(call) {
      try self.tasks.get(self.requiredTaskId(call)).start()
      return nil
    }
  }

  @objc public func pause(_ call: CAPPluginCall) {
    runAsync(call) {
      try self.tasks.get(self.requiredTaskId(call)).pause()
      return nil
    }
  }

  @objc public func cancel(_ call: CAPPluginCall) {
    runAsync(call) {
      try self.tasks.get(self.requiredTaskId(call)).cancel()
      return nil
    }
  }

  @objc public func snapshot(_ call: CAPPluginCall) {
    runAsync(call) {
      let task = try self.tasks.get(self.requiredTaskId(call))
      let snapshot = try task.snapshot()
      return ["snapshot": snapshotPayload(snapshot, lastError: self.snapshotLastError(task: task, snapshot: snapshot))]
    }
  }

  @objc public func bitmap(_ call: CAPPluginCall) {
    runAsync(call) {
      let data = try self.tasks.get(self.requiredTaskId(call)).copyBitmap()
      return ["data": data.base64EncodedString()]
    }
  }

  @objc public func close(_ call: CAPPluginCall) {
    runAsync(call) {
      try self.tasks.close(self.requiredTaskId(call))
      return nil
    }
  }

  @objc public func downloadToCompletion(_ call: CAPPluginCall) {
    runAsync(call) {
      let parsed = try TakanawaCapacitorOptions.parse(call.options)
      try Takanawa.initialize(maxIo: parsed.maxIo)
      let download = try TakanawaDownload.create(parsed.config)
      defer {
        try? download.close()
      }

      let terminalLock = NSLock()
      var terminalSnapshot: DownloadSnapshot?
      let terminal = DispatchSemaphore(value: 0)
      try download.setProgressCallback { snapshot in
        guard snapshot.phase.isTerminal else {
          return
        }
        terminalLock.lock()
        if terminalSnapshot == nil {
          terminalSnapshot = snapshot
          terminal.signal()
        }
        terminalLock.unlock()
      }
      try download.start()
      terminal.wait()

      terminalLock.lock()
      let snapshot = terminalSnapshot
      terminalLock.unlock()

      guard let snapshot else {
        throw TakanawaCapacitorError.invalidConfig("download ended before reaching a terminal phase")
      }

      switch snapshot.phase {
      case .completed:
        return ["snapshot": snapshotPayload(snapshot)]
      case .failed:
        throw TakanawaCapacitorError.invalidConfig(self.snapshotLastError(task: download, snapshot: snapshot) ?? "download failed")
      case .cancelled:
        throw TakanawaCapacitorError.invalidConfig(self.snapshotLastError(task: download, snapshot: snapshot) ?? "download cancelled")
      default:
        throw TakanawaCapacitorError.invalidConfig("download ended before reaching a terminal phase")
      }
    }
  }

  private func emitProgress(taskId: String, snapshot: DownloadSnapshot) {
    var payload = JSObject()
    payload["taskId"] = taskId
    if let task = tasks.getOrNil(taskId) {
      payload["snapshot"] = snapshotPayload(snapshot, lastError: snapshotLastError(task: task, snapshot: snapshot))
    } else {
      payload["snapshot"] = snapshotPayload(snapshot)
    }

    DispatchQueue.main.async { [weak self] in
      self?.notifyListeners("downloadProgress", data: payload)
    }
  }

  private func snapshotLastError(task: TakanawaDownload, snapshot: DownloadSnapshot) -> String? {
    guard snapshot.phase == .failed else {
      return nil
    }
    return (try? task.lastError()).flatMap { $0.isEmpty ? nil : $0 }
  }

  private func requiredTaskId(_ call: CAPPluginCall) throws -> String {
    guard let taskId = call.getString("taskId"), !taskId.isEmpty else {
      throw TakanawaCapacitorError.invalidConfig("taskId is required")
    }
    return taskId
  }

  private func runAsync(_ call: CAPPluginCall, _ block: @escaping () throws -> JSObject?) {
    queue.async {
      do {
        let result = try block()
        DispatchQueue.main.async {
          if let result {
            call.resolve(result)
          } else {
            call.resolve()
          }
        }
      } catch {
        DispatchQueue.main.async {
          call.reject(String(describing: error))
        }
      }
    }
  }
}
