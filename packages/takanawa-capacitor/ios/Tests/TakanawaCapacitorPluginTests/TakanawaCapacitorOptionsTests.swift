import Capacitor
import Takanawa
@testable import TakanawaCapacitorPlugin
import XCTest

final class TakanawaCapacitorOptionsTests: XCTestCase {
  func testAppliesMaxIoDefaultsAndZeroNormalization() throws {
    XCTAssertEqual(try TakanawaCapacitorOptions.parse(baseOptions()).maxIo, 4)

    var zero = baseOptions()
    zero["maxIo"] = 0
    XCTAssertEqual(try TakanawaCapacitorOptions.parse(zero).maxIo, 1)

    var eight = baseOptions()
    eight["maxIo"] = 8
    XCTAssertEqual(try TakanawaCapacitorOptions.parse(eight).maxIo, 8)
  }

  func testParsesHashObjectsWithAliasesAndPrefixes() throws {
    var options = baseOptions()
    options["hash"] = [
      "kind": "sha-1",
      "expected": "sha-1:" + String(repeating: "00", count: 20)
    ] as JSObject

    let config = try TakanawaCapacitorOptions.parse(options).config

    XCTAssertEqual(config.hashKind, .sha1)
    XCTAssertEqual(config.expectedHash, Data(repeating: 0, count: 20))
  }

  func testParsesLegacySha256Option() throws {
    var options = baseOptions()
    options["sha256"] = String(repeating: "11", count: 32)

    let config = try TakanawaCapacitorOptions.parse(options).config

    XCTAssertEqual(config.hashKind, .sha256)
    XCTAssertEqual(config.expectedHash, Data(repeating: 0x11, count: 32))
  }

  func testRejectsInvalidHashes() {
    var shortHash = baseOptions()
    shortHash["hash"] = ["kind": "md5", "expected": "00"] as JSObject
    XCTAssertThrowsError(try TakanawaCapacitorOptions.parse(shortHash))

    var duplicateHash = baseOptions()
    duplicateHash["hash"] = [
      "kind": "sha256",
      "expected": String(repeating: "00", count: 32)
    ] as JSObject
    duplicateHash["sha256"] = String(repeating: "11", count: 32)
    XCTAssertThrowsError(try TakanawaCapacitorOptions.parse(duplicateHash))
  }

  func testRejectsNegativeValues() {
    var options = baseOptions()
    options["chunkSize"] = -1
    XCTAssertThrowsError(try TakanawaCapacitorOptions.parse(options))
  }

  private func baseOptions() -> JSObject {
    [
      "url": "https://example.test/file.bin",
      "targetPath": "/tmp/file.bin"
    ]
  }
}
