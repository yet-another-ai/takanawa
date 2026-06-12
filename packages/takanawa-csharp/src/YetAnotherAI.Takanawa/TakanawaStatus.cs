namespace YetAnotherAI.Takanawa
{
    public enum TakanawaStatus
    {
        Ok = 0,
        BufferTooSmall = 1,
        NullPointer = -1,
        AbiMismatch = -2,
        InvalidConfig = -3,
        RuntimeNotInitialized = -4,
        TargetExists = -10,
        PartBusy = -11,
        PartSizeMismatch = -12,
        PartCorrupt = -13,
        RemoteChanged = -14,
        HttpProtocol = -20,
        Network = -21,
        Io = -30,
        HashMismatch = -40,
        Cancelled = -50,
        AlreadyStarted = -51,
        Panic = -100,
        Internal = -101,
    }
}
