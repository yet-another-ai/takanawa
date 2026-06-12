import Capacitor
import Foundation
import Takanawa

internal struct ParsedDownloadOptions {
  let config: DownloadConfig
  let maxIo: Int
}

internal enum TakanawaCapacitorOptions {
  private static let defaultMaxIo = 4

  static func parse(_ rawOptions: [AnyHashable: Any]) throws -> ParsedDownloadOptions {
    let options = try normalizeOptions(rawOptions)
    let hash = try parseHash(options)
    let config = try DownloadConfig(
      url: requiredString(options, "url"),
      targetPath: requiredString(options, "targetPath"),
      chunkSize: optionalUInt64(options, "chunkSize", defaultValue: 0),
      parallelism: optionalInt(options, "parallelism", defaultValue: 0),
      maxParallelChunks: optionalInt(options, "maxParallelChunks", defaultValue: 0),
      maxRetries: optionalUInt32(options, "maxRetries", defaultValue: 4),
      backoffInitialMillis: optionalUInt64(options, "backoffInitialMs", defaultValue: 100),
      backoffMaxMillis: optionalUInt64(options, "backoffMaxMs", defaultValue: 3_000),
      connectTimeoutMillis: optionalUInt64(options, "connectTimeoutMs", defaultValue: 30_000),
      readTimeoutMillis: optionalUInt64(options, "readTimeoutMs", defaultValue: 0),
      totalTimeoutMillis: optionalUInt64(options, "totalTimeoutMs", defaultValue: 0),
      bytesPerSecondLimit: optionalUInt64(options, "bytesPerSecondLimit", defaultValue: 0),
      hashKind: hash.kind,
      expectedHash: hash.expected
    )

    return ParsedDownloadOptions(
      config: config,
      maxIo: max(try optionalIntOrNil(options, "maxIo") ?? defaultMaxIo, 1)
    )
  }

  private static func normalizeOptions(_ options: [AnyHashable: Any]) throws -> JSObject {
    var normalized = JSObject()
    for (key, value) in options {
      if let key = key as? String {
        normalized[key] = try normalizeValue(value, path: key)
      }
    }
    return normalized
  }

  private static func normalizeObject(_ options: [String: Any], path: String) throws -> JSObject {
    var normalized = JSObject()
    for (key, value) in options {
      normalized[key] = try normalizeValue(value, path: "\(path).\(key)")
    }
    return normalized
  }

  private static func normalizeArray(_ values: [Any], path: String) throws -> JSArray {
    try values.enumerated().map { index, value in
      try normalizeValue(value, path: "\(path)[\(index)]")
    }
  }

  private static func normalizeValue(_ value: Any, path: String) throws -> any JSValue {
    if let object = value as? [String: Any] {
      return try normalizeObject(object, path: path)
    }
    if let object = value as? [AnyHashable: Any] {
      return try normalizeOptions(object)
    }
    if let array = value as? [Any] {
      return try normalizeArray(array, path: path)
    }
    if let jsValue = value as? any JSValue {
      return jsValue
    }
    throw TakanawaCapacitorError.invalidConfig("\(path) must be a JSON-compatible value")
  }

  private static func parseHash(_ options: JSObject) throws -> (kind: HashKind, expected: Data?) {
    let hashValue = options["hash"]
    let sha256Value = options["sha256"]
    let hasHash = hashValue != nil && !(hashValue is NSNull)
    let hasSha256 = sha256Value != nil && !(sha256Value is NSNull)
    if hasHash && hasSha256 {
      throw TakanawaCapacitorError.invalidConfig("use either hash or sha256, not both")
    }

    if hasSha256 {
      return (
        .sha256,
        try decodeExpectedHash(kind: .sha256, value: requiredString(options, "sha256"))
      )
    }
    guard hasHash, let hashValue else {
      return (.none, nil)
    }

    if let hashString = hashValue as? String {
      guard let separator = hashString.firstIndex(of: ":") else {
        throw TakanawaCapacitorError.invalidConfig("hash string must use the format \"kind:hex\"")
      }
      let kind = try parseHashKind(String(hashString[..<separator]))
      let expected = String(hashString[hashString.index(after: separator)...])
      return (kind, try decodeExpectedHash(kind: kind, value: expected))
    }

    guard let hashObject = hashValue as? JSObject else {
      throw TakanawaCapacitorError.invalidConfig("hash must be a string or object")
    }
    let kind = try parseHashKind(requiredString(hashObject, "kind"))
    return (kind, try decodeExpectedHash(kind: kind, value: requiredString(hashObject, "expected")))
  }

  private static func parseHashKind(_ value: String) throws -> HashKind {
    switch value.lowercased() {
    case "sha1", "sha-1":
      return .sha1
    case "sha256", "sha-256":
      return .sha256
    case "sha512", "sha-512":
      return .sha512
    case "md5":
      return .md5
    case "crc32", "crc-32":
      return .crc32
    default:
      throw TakanawaCapacitorError.invalidConfig("unsupported hash kind: \(value)")
    }
  }

  private static func decodeExpectedHash(kind: HashKind, value: String) throws -> Data {
    let normalized = stripHashPrefix(kind: kind, value: value)
    let expectedHexLength = kind.expectedLength * 2
    guard normalized.count == expectedHexLength else {
      throw TakanawaCapacitorError.invalidConfig(
        "invalid \(kind.label): expected \(expectedHexLength) hex characters"
      )
    }

    var bytes = Data()
    bytes.reserveCapacity(kind.expectedLength)
    var index = normalized.startIndex
    while index < normalized.endIndex {
      let next = normalized.index(index, offsetBy: 2)
      guard let byte = UInt8(normalized[index..<next], radix: 16) else {
        throw TakanawaCapacitorError.invalidConfig("invalid \(kind.label): expected hex characters")
      }
      bytes.append(byte)
      index = next
    }
    return bytes
  }

  private static func stripHashPrefix(kind: HashKind, value: String) -> String {
    for prefix in Array(Set([kind.prefix, kind.legacyPrefix])) {
      if value.lowercased().hasPrefix(prefix) {
        return String(value.dropFirst(prefix.count))
      }
    }
    return value
  }

  private static func requiredString(_ options: JSObject, _ key: String) throws -> String {
    guard let value = options[key], !(value is NSNull) else {
      throw TakanawaCapacitorError.invalidConfig("\(key) is required")
    }
    guard let string = value as? String else {
      throw TakanawaCapacitorError.invalidConfig("\(key) must be a string")
    }
    return string
  }

  private static func optionalInt(
    _ options: JSObject,
    _ key: String,
    defaultValue: Int
  ) throws -> Int {
    try optionalIntOrNil(options, key) ?? defaultValue
  }

  private static func optionalIntOrNil(_ options: JSObject, _ key: String) throws -> Int? {
    guard let value = try optionalUInt64OrNil(options, key) else {
      return nil
    }
    guard value <= UInt64(Int.max) else {
      throw TakanawaCapacitorError.invalidConfig("\(key) must fit in an Int")
    }
    return Int(value)
  }

  private static func optionalUInt32(
    _ options: JSObject,
    _ key: String,
    defaultValue: UInt32
  ) throws -> UInt32 {
    guard let value = try optionalUInt64OrNil(options, key) else {
      return defaultValue
    }
    guard value <= UInt64(UInt32.max) else {
      throw TakanawaCapacitorError.invalidConfig("\(key) must fit in a 32-bit unsigned integer")
    }
    return UInt32(value)
  }

  private static func optionalUInt64(
    _ options: JSObject,
    _ key: String,
    defaultValue: UInt64
  ) throws -> UInt64 {
    try optionalUInt64OrNil(options, key) ?? defaultValue
  }

  private static func optionalUInt64OrNil(_ options: JSObject, _ key: String) throws -> UInt64? {
    guard let value = options[key], !(value is NSNull) else {
      return nil
    }
    switch value {
    case let number as NSNumber:
      let double = number.doubleValue
      guard double.isFinite, double >= 0, double.rounded() == double else {
        throw TakanawaCapacitorError.invalidConfig("\(key) must be a non-negative integer")
      }
      guard double <= Double(UInt64.max) else {
        throw TakanawaCapacitorError.invalidConfig("\(key) must fit in UInt64")
      }
      return UInt64(double)
    case let string as String:
      guard let parsed = UInt64(string) else {
        throw TakanawaCapacitorError.invalidConfig("\(key) must be an unsigned integer string")
      }
      return parsed
    default:
      throw TakanawaCapacitorError.invalidConfig("\(key) must be a number or unsigned integer string")
    }
  }
}

private extension HashKind {
  var expectedLength: Int {
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

  var prefix: String {
    switch self {
    case .none:
      return ""
    case .sha1:
      return "sha1:"
    case .sha256:
      return "sha256:"
    case .sha512:
      return "sha512:"
    case .md5:
      return "md5:"
    case .crc32:
      return "crc32:"
    }
  }

  var legacyPrefix: String {
    switch self {
    case .sha1:
      return "sha-1:"
    case .sha256:
      return "sha-256:"
    case .sha512:
      return "sha-512:"
    case .crc32:
      return "crc-32:"
    case .none, .md5:
      return prefix
    }
  }

  var label: String {
    switch self {
    case .none:
      return "none"
    case .sha1:
      return "sha1"
    case .sha256:
      return "sha256"
    case .sha512:
      return "sha512"
    case .md5:
      return "md5"
    case .crc32:
      return "crc32"
    }
  }
}
