import Foundation

internal enum TakanawaCapacitorError: Error, CustomStringConvertible {
  case invalidConfig(String)

  var description: String {
    switch self {
    case let .invalidConfig(message):
      return message
    }
  }
}
