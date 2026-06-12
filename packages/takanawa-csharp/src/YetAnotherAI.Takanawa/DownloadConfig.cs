using System;

namespace YetAnotherAI.Takanawa
{
    public sealed class DownloadConfig
    {
        public DownloadConfig(
            string url,
            string targetPath,
            long chunkSize = 0,
            int parallelism = 0,
            int maxParallelChunks = 0,
            int maxRetries = 4,
            long backoffInitialMillis = 100,
            long backoffMaxMillis = 3_000,
            long connectTimeoutMillis = 30_000,
            long readTimeoutMillis = 0,
            long totalTimeoutMillis = 0,
            long bytesPerSecondLimit = 0,
            byte[]? expectedSha256 = null,
            HashKind? hashKind = null,
            byte[]? expectedHash = null)
        {
            if (string.IsNullOrWhiteSpace(url))
            {
                throw new ArgumentException("url must not be blank", nameof(url));
            }

            if (string.IsNullOrWhiteSpace(targetPath))
            {
                throw new ArgumentException("targetPath must not be blank", nameof(targetPath));
            }

            RequireNonNegative(chunkSize, nameof(chunkSize));
            RequireNonNegative(parallelism, nameof(parallelism));
            RequireNonNegative(maxParallelChunks, nameof(maxParallelChunks));
            RequireNonNegative(maxRetries, nameof(maxRetries));
            RequireNonNegative(backoffInitialMillis, nameof(backoffInitialMillis));
            RequireNonNegative(backoffMaxMillis, nameof(backoffMaxMillis));
            RequireNonNegative(connectTimeoutMillis, nameof(connectTimeoutMillis));
            RequireNonNegative(readTimeoutMillis, nameof(readTimeoutMillis));
            RequireNonNegative(totalTimeoutMillis, nameof(totalTimeoutMillis));
            RequireNonNegative(bytesPerSecondLimit, nameof(bytesPerSecondLimit));

            if (expectedSha256 != null && expectedSha256.Length != HashLength(HashKind.Sha256))
            {
                throw new ArgumentException("expectedSha256 must be exactly 32 bytes", nameof(expectedSha256));
            }

            var resolvedHashKind = hashKind ?? (expectedSha256 == null ? HashKind.None : HashKind.Sha256);
            var resolvedExpectedHash = expectedHash ?? expectedSha256;
            if ((resolvedHashKind == HashKind.None) != (resolvedExpectedHash == null))
            {
                throw new ArgumentException("expectedHash must be null when hashKind is None and non-null otherwise", nameof(expectedHash));
            }

            if (resolvedExpectedHash != null && resolvedExpectedHash.Length != HashLength(resolvedHashKind))
            {
                throw new ArgumentException(
                    $"expectedHash for {resolvedHashKind} must be exactly {HashLength(resolvedHashKind)} bytes",
                    nameof(expectedHash));
            }

            Url = url;
            TargetPath = targetPath;
            ChunkSize = chunkSize;
            Parallelism = parallelism;
            MaxParallelChunks = maxParallelChunks;
            MaxRetries = maxRetries;
            BackoffInitialMillis = backoffInitialMillis;
            BackoffMaxMillis = backoffMaxMillis;
            ConnectTimeoutMillis = connectTimeoutMillis;
            ReadTimeoutMillis = readTimeoutMillis;
            TotalTimeoutMillis = totalTimeoutMillis;
            BytesPerSecondLimit = bytesPerSecondLimit;
            ExpectedSha256 = expectedSha256 == null ? null : (byte[])expectedSha256.Clone();
            HashKind = resolvedHashKind;
            ExpectedHash = resolvedExpectedHash == null ? null : (byte[])resolvedExpectedHash.Clone();
        }

        public string Url { get; }

        public string TargetPath { get; }

        public long ChunkSize { get; }

        public int Parallelism { get; }

        public int MaxParallelChunks { get; }

        public int MaxRetries { get; }

        public long BackoffInitialMillis { get; }

        public long BackoffMaxMillis { get; }

        public long ConnectTimeoutMillis { get; }

        public long ReadTimeoutMillis { get; }

        public long TotalTimeoutMillis { get; }

        public long BytesPerSecondLimit { get; }

        public byte[]? ExpectedSha256 { get; }

        public HashKind HashKind { get; }

        public byte[]? ExpectedHash { get; }

        internal byte[]? ExpectedHashCopy()
        {
            return ExpectedHash == null ? null : (byte[])ExpectedHash.Clone();
        }

        internal static int HashLength(HashKind kind)
        {
            switch (kind)
            {
                case HashKind.None:
                    return 0;
                case HashKind.Sha256:
                    return 32;
                case HashKind.Sha1:
                    return 20;
                case HashKind.Sha512:
                    return 64;
                case HashKind.Md5:
                    return 16;
                case HashKind.Crc32:
                    return 4;
                default:
                    throw new ArgumentException($"unsupported hash kind {kind}", nameof(kind));
            }
        }

        private static void RequireNonNegative(long value, string name)
        {
            if (value < 0)
            {
                throw new ArgumentOutOfRangeException(name, value, $"{name} must be greater than or equal to 0");
            }
        }
    }
}
