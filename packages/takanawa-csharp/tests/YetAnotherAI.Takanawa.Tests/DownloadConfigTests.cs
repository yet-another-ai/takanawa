using System;
using Xunit;

namespace YetAnotherAI.Takanawa.Tests
{
    public sealed class DownloadConfigTests
    {
        [Theory]
        [InlineData("")]
        [InlineData("   ")]
        public void RejectsBlankUrl(string url)
        {
            Assert.Throws<ArgumentException>(() => new DownloadConfig(url, "/tmp/file.bin"));
        }

        [Fact]
        public void RejectsBlankTargetPath()
        {
            Assert.Throws<ArgumentException>(() => new DownloadConfig("https://example.com/file.bin", " "));
        }

        [Theory]
        [InlineData(-1, "chunkSize")]
        [InlineData(-2, "backoffInitialMillis")]
        public void RejectsNegativeLongValues(long value, string parameter)
        {
            var exception = Assert.Throws<ArgumentOutOfRangeException>(() =>
            {
                if (parameter == "chunkSize")
                {
                    _ = new DownloadConfig("https://example.com/file.bin", "/tmp/file.bin", chunkSize: value);
                }
                else
                {
                    _ = new DownloadConfig("https://example.com/file.bin", "/tmp/file.bin", backoffInitialMillis: value);
                }
            });
            Assert.Equal(parameter, exception.ParamName);
        }

        [Fact]
        public void ExpectedSha256ShortcutSelectsSha256()
        {
            var expected = new byte[32];
            var config = new DownloadConfig("https://example.com/file.bin", "/tmp/file.bin", expectedSha256: expected);

            Assert.Equal(HashKind.Sha256, config.HashKind);
            Assert.Equal(32, config.ExpectedHash!.Length);
            Assert.NotSame(expected, config.ExpectedHash);
        }

        [Theory]
        [InlineData(HashKind.Sha1, 20)]
        [InlineData(HashKind.Sha256, 32)]
        [InlineData(HashKind.Sha512, 64)]
        [InlineData(HashKind.Md5, 16)]
        [InlineData(HashKind.Crc32, 4)]
        public void AcceptsSupportedHashKinds(HashKind kind, int length)
        {
            var config = new DownloadConfig(
                "https://example.com/file.bin",
                "/tmp/file.bin",
                hashKind: kind,
                expectedHash: new byte[length]);

            Assert.Equal(kind, config.HashKind);
            Assert.Equal(length, config.ExpectedHash!.Length);
        }

        [Fact]
        public void RejectsMismatchedHashLength()
        {
            Assert.Throws<ArgumentException>(() => new DownloadConfig(
                "https://example.com/file.bin",
                "/tmp/file.bin",
                hashKind: HashKind.Sha1,
                expectedHash: new byte[32]));
        }

        [Fact]
        public void RejectsHashWithoutExpectedBytes()
        {
            Assert.Throws<ArgumentException>(() => new DownloadConfig(
                "https://example.com/file.bin",
                "/tmp/file.bin",
                hashKind: HashKind.Sha1));
        }
    }
}
