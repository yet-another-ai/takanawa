namespace YetAnotherAI.Takanawa
{
    public enum DownloadPhase
    {
        Created = 0,
        Running = 1,
        Paused = 2,
        Cancelled = 3,
        Completed = 4,
        Failed = 5,
        Pausing = 6,
        Cancelling = 7,
        Starting = 8,
        Allocating = 9,
        Verifying = 10,
    }
}
