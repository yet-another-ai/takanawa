package ai.yetanother.takanawa.capacitor

import ai.yetanother.takanawa.DownloadConfig
import ai.yetanother.takanawa.HashKind
import java.math.BigInteger
import java.util.Locale
import org.json.JSONObject

internal data class ParsedDownloadOptions(
    val config: DownloadConfig,
    val maxIo: Int,
)

internal object TakanawaCapacitorOptions {
    private const val DEFAULT_MAX_IO = 4
    private val maxLong = BigInteger.valueOf(Long.MAX_VALUE)

    fun parse(options: JSONObject): ParsedDownloadOptions {
        val (hashKind, expectedHash) = parseHash(options)
        val config = DownloadConfig(
            url = requiredString(options, "url"),
            targetPath = requiredString(options, "targetPath"),
            chunkSize = optionalLong(options, "chunkSize", 0),
            parallelism = optionalInt(options, "parallelism", 0),
            maxParallelChunks = optionalInt(options, "maxParallelChunks", 0),
            maxRetries = optionalInt(options, "maxRetries", 4),
            backoffInitialMillis = optionalLong(options, "backoffInitialMs", 100),
            backoffMaxMillis = optionalLong(options, "backoffMaxMs", 3_000),
            connectTimeoutMillis = optionalLong(options, "connectTimeoutMs", 30_000),
            readTimeoutMillis = optionalLong(options, "readTimeoutMs", 0),
            totalTimeoutMillis = optionalLong(options, "totalTimeoutMs", 0),
            bytesPerSecondLimit = optionalLong(options, "bytesPerSecondLimit", 0),
            hashKind = hashKind,
            expectedHash = expectedHash,
        )
        return ParsedDownloadOptions(
            config = config,
            maxIo = normalizeMaxIo(optionalIntOrNull(options, "maxIo")),
        )
    }

    private fun parseHash(options: JSONObject): Pair<HashKind, ByteArray?> {
        val hasHash = options.has("hash") && !options.isNull("hash")
        val hasSha256 = options.has("sha256") && !options.isNull("sha256")
        require(!(hasHash && hasSha256)) { "use either hash or sha256, not both" }

        if (hasSha256) {
            return HashKind.SHA256 to decodeExpectedHash(
                HashKind.SHA256,
                requiredString(options, "sha256"),
            )
        }
        if (!hasHash) {
            return HashKind.NONE to null
        }

        return when (val hash = options.get("hash")) {
            is String -> {
                val separator = hash.indexOf(':')
                require(separator > 0) { "hash string must use the format \"kind:hex\"" }
                val kind = parseHashKind(hash.substring(0, separator))
                kind to decodeExpectedHash(kind, hash.substring(separator + 1))
            }
            is JSONObject -> {
                val kind = parseHashKind(requiredString(hash, "kind"))
                kind to decodeExpectedHash(kind, requiredString(hash, "expected"))
            }
            else -> throw IllegalArgumentException("hash must be a string or object")
        }
    }

    private fun parseHashKind(value: String): HashKind =
        when (value.lowercase(Locale.US)) {
            "sha1", "sha-1" -> HashKind.SHA1
            "sha256", "sha-256" -> HashKind.SHA256
            "sha512", "sha-512" -> HashKind.SHA512
            "md5" -> HashKind.MD5
            "crc32", "crc-32" -> HashKind.CRC32
            else -> throw IllegalArgumentException("unsupported hash kind: $value")
        }

    private fun decodeExpectedHash(kind: HashKind, value: String): ByteArray {
        val normalized = stripHashPrefix(kind, value)
        val expectedHexLength = kind.expectedLength() * 2
        require(normalized.length == expectedHexLength) {
            "invalid ${kind.label()}: expected $expectedHexLength hex characters"
        }

        return ByteArray(kind.expectedLength()) { index ->
            val high = Character.digit(normalized[index * 2], 16)
            val low = Character.digit(normalized[index * 2 + 1], 16)
            require(high >= 0 && low >= 0) { "invalid ${kind.label()}: expected hex characters" }
            ((high shl 4) or low).toByte()
        }
    }

    private fun stripHashPrefix(kind: HashKind, value: String): String {
        val prefixes = listOf(kind.prefix(), kind.legacyPrefix()).distinct()
        return prefixes.firstNotNullOfOrNull { prefix ->
            if (value.regionMatches(0, prefix, 0, prefix.length, ignoreCase = true)) {
                value.substring(prefix.length)
            } else {
                null
            }
        } ?: value
    }

    private fun requiredString(options: JSONObject, key: String): String {
        require(options.has(key) && !options.isNull(key)) { "$key is required" }
        return options.getString(key)
    }

    private fun optionalInt(options: JSONObject, key: String, defaultValue: Int): Int =
        optionalIntOrNull(options, key) ?: defaultValue

    private fun optionalIntOrNull(options: JSONObject, key: String): Int? {
        val value = optionalLongOrNull(options, key) ?: return null
        require(value <= Int.MAX_VALUE) { "$key must fit in a 32-bit integer" }
        return value.toInt()
    }

    private fun optionalLong(options: JSONObject, key: String, defaultValue: Long): Long =
        optionalLongOrNull(options, key) ?: defaultValue

    private fun optionalLongOrNull(options: JSONObject, key: String): Long? {
        if (!options.has(key) || options.isNull(key)) {
            return null
        }
        return parseNonNegativeLong(options.get(key), key)
    }

    private fun parseNonNegativeLong(value: Any, key: String): Long =
        when (value) {
            is Number -> {
                val asDouble = value.toDouble()
                require(asDouble.isFinite() && asDouble >= 0 && asDouble % 1.0 == 0.0) {
                    "$key must be a non-negative integer"
                }
                require(asDouble <= Long.MAX_VALUE.toDouble()) { "$key must fit in a signed 64-bit integer" }
                value.toLong()
            }
            is String -> {
                require(value.matches(Regex("^\\d+$"))) { "$key must be an unsigned integer string" }
                val parsed = BigInteger(value)
                require(parsed <= maxLong) { "$key must fit in a signed 64-bit integer" }
                parsed.toLong()
            }
            else -> throw IllegalArgumentException("$key must be a number or unsigned integer string")
        }

    private fun normalizeMaxIo(value: Int?): Int = (value ?: DEFAULT_MAX_IO).coerceAtLeast(1)

    private fun HashKind.prefix(): String =
        when (this) {
            HashKind.SHA1 -> "sha1:"
            HashKind.SHA256 -> "sha256:"
            HashKind.SHA512 -> "sha512:"
            HashKind.MD5 -> "md5:"
            HashKind.CRC32 -> "crc32:"
            HashKind.NONE -> ""
        }

    private fun HashKind.expectedLength(): Int =
        when (this) {
            HashKind.NONE -> 0
            HashKind.SHA1 -> 20
            HashKind.SHA256 -> 32
            HashKind.SHA512 -> 64
            HashKind.MD5 -> 16
            HashKind.CRC32 -> 4
        }

    private fun HashKind.legacyPrefix(): String =
        when (this) {
            HashKind.SHA1 -> "sha-1:"
            HashKind.SHA256 -> "sha-256:"
            HashKind.SHA512 -> "sha-512:"
            HashKind.CRC32 -> "crc-32:"
            HashKind.NONE,
            HashKind.MD5 -> prefix()
        }

    private fun HashKind.label(): String =
        when (this) {
            HashKind.SHA1 -> "sha1"
            HashKind.SHA256 -> "sha256"
            HashKind.SHA512 -> "sha512"
            HashKind.MD5 -> "md5"
            HashKind.CRC32 -> "crc32"
            HashKind.NONE -> "none"
        }
}
