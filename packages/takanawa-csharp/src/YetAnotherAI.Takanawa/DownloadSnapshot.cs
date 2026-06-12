namespace YetAnotherAI.Takanawa
{
    public readonly struct DownloadSnapshot
    {
        internal DownloadSnapshot(NativeMethods.TknwDownloadSnapshot native)
        {
            Phase = NativeConversions.ToDownloadPhase(native.Phase);
            ContentLen = NativeConversions.ToLong(native.ContentLen, nameof(ContentLen));
            DownloadedBytes = NativeConversions.ToLong(native.DownloadedBytes, nameof(DownloadedBytes));
            ChunkSize = NativeConversions.ToLong(native.ChunkSize, nameof(ChunkSize));
            ChunkCount = NativeConversions.ToLong(native.ChunkCount, nameof(ChunkCount));
            CompletedChunks = NativeConversions.ToLong(native.CompletedChunks, nameof(CompletedChunks));
            ActiveIo = NativeConversions.ToInt32(native.ActiveIo, nameof(ActiveIo));
        }

        public DownloadPhase Phase { get; }

        public long ContentLen { get; }

        public long DownloadedBytes { get; }

        public long ChunkSize { get; }

        public long ChunkCount { get; }

        public long CompletedChunks { get; }

        public int ActiveIo { get; }
    }
}
