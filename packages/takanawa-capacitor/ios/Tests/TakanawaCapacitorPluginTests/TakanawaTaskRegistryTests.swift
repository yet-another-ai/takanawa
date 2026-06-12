@testable import TakanawaCapacitorPlugin
import XCTest

final class TakanawaTaskRegistryTests: XCTestCase {
  func testClosesAndRemovesTasksById() throws {
    let registry = TakanawaTaskRegistry<FakeTask>()
    let task = FakeTask()
    let taskId = registry.insert(task)

    XCTAssertEqual(registry.count, 1)
    try registry.close(taskId)

    XCTAssertEqual(registry.count, 0)
    XCTAssertEqual(task.closeCount, 1)
    try registry.close(taskId)
    XCTAssertEqual(task.closeCount, 1)
  }

  func testRejectsUnknownTaskIds() {
    let registry = TakanawaTaskRegistry<FakeTask>()

    XCTAssertThrowsError(try registry.get("missing"))
  }

  private final class FakeTask: TakanawaClosable {
    var closeCount = 0

    func close() throws {
      closeCount += 1
    }
  }
}
