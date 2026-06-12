using System;

namespace YetAnotherAI.Takanawa
{
    public sealed class TakanawaException : Exception
    {
        public TakanawaException(TakanawaStatus status, string? message = null)
            : base(string.IsNullOrEmpty(message) ? status.ToString() : message)
        {
            Status = status;
        }

        public TakanawaStatus Status { get; }
    }
}
