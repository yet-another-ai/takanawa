namespace YetAnotherAI.Takanawa
{
    public readonly struct DownloadSpeedSnapshot
    {
        internal DownloadSpeedSnapshot(NativeMethods.TknwDownloadSpeedSnapshot native)
        {
            Phase = NativeConversions.ToDownloadPhase(native.Phase);
            ContentLen = NativeConversions.ToLong(native.ContentLen, nameof(ContentLen));
            ReceivedBytes = NativeConversions.ToLong(native.ReceivedBytes, nameof(ReceivedBytes));
            IntervalBytes = NativeConversions.ToLong(native.IntervalBytes, nameof(IntervalBytes));
            ElapsedMillis = NativeConversions.ToLong(native.ElapsedMillis, nameof(ElapsedMillis));
            BytesPerSecond = native.BytesPerSecond;
            ActiveIo = NativeConversions.ToInt32(native.ActiveIo, nameof(ActiveIo));
        }

        public DownloadPhase Phase { get; }

        public long ContentLen { get; }

        public long ReceivedBytes { get; }

        public long IntervalBytes { get; }

        public long ElapsedMillis { get; }

        public double BytesPerSecond { get; }

        public int ActiveIo { get; }
    }
}
