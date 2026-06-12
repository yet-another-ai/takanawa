import Foundation
import Takanawa

internal protocol TakanawaClosable: AnyObject {
  func close() throws
}

extension TakanawaDownload: TakanawaClosable {}

internal final class TakanawaTaskRegistry<Task: TakanawaClosable> {
  private let lock = NSLock()
  private var tasks: [String: Task] = [:]

  var count: Int {
    lock.lock()
    defer { lock.unlock() }
    return tasks.count
  }

  func insert(_ task: Task) -> String {
    lock.lock()
    defer { lock.unlock() }

    while true {
      let taskId = UUID().uuidString
      if tasks[taskId] == nil {
        tasks[taskId] = task
        return taskId
      }
    }
  }

  func get(_ taskId: String) throws -> Task {
    lock.lock()
    defer { lock.unlock() }

    guard let task = tasks[taskId] else {
      throw TakanawaCapacitorError.invalidConfig("unknown download task: \(taskId)")
    }
    return task
  }

  func getOrNil(_ taskId: String) -> Task? {
    lock.lock()
    defer { lock.unlock() }
    return tasks[taskId]
  }

  func close(_ taskId: String) throws {
    let task: Task? = {
      lock.lock()
      defer { lock.unlock() }
      return tasks.removeValue(forKey: taskId)
    }()
    try task?.close()
  }

  func closeAll() {
    let current: [Task] = {
      lock.lock()
      defer { lock.unlock() }
      let current = Array(tasks.values)
      tasks.removeAll()
      return current
    }()

    for task in current {
      try? task.close()
    }
  }
}
