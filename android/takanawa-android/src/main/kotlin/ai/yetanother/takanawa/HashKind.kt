package ai.yetanother.takanawa

enum class HashKind(internal val code: Int, internal val expectedLength: Int) {
    NONE(0, 0),
    SHA256(1, 32),
    SHA1(2, 20),
    SHA512(3, 64),
    MD5(4, 16),
    CRC32(5, 4),
}
