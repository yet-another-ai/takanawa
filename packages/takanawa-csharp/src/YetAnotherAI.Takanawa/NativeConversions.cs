using System;
using System.Runtime.InteropServices;

namespace YetAnotherAI.Takanawa
{
    internal static class NativeConversions
    {
        internal static UIntPtr SizeOf<T>()
        {
            return new UIntPtr((uint)Marshal.SizeOf(typeof(T)));
        }

        internal static UIntPtr ToUIntPtr(int value, string name)
        {
            if (value < 0)
            {
                throw new ArgumentOutOfRangeException(name, value, $"{name} must be greater than or equal to 0");
            }

            return new UIntPtr((uint)value);
        }

        internal static ulong ToUInt64(long value, string name)
        {
            if (value < 0)
            {
                throw new ArgumentOutOfRangeException(name, value, $"{name} must be greater than or equal to 0");
            }

            return (ulong)value;
        }

        internal static uint ToUInt32(int value, string name)
        {
            if (value < 0)
            {
                throw new ArgumentOutOfRangeException(name, value, $"{name} must be greater than or equal to 0");
            }

            return (uint)value;
        }

        internal static long ToLong(ulong value, string name)
        {
            if (value > long.MaxValue)
            {
                throw new TakanawaException(TakanawaStatus.Internal, $"{name} exceeded Int64.MaxValue");
            }

            return (long)value;
        }

        internal static int ToInt32(UIntPtr value, string name)
        {
            var unsigned = value.ToUInt64();
            if (unsigned > int.MaxValue)
            {
                throw new TakanawaException(TakanawaStatus.Internal, $"{name} exceeded Int32.MaxValue");
            }

            return (int)unsigned;
        }

        internal static DownloadPhase ToDownloadPhase(uint value)
        {
            if (value > int.MaxValue)
            {
                return DownloadPhase.Failed;
            }

            var phase = (DownloadPhase)(int)value;
            return Enum.IsDefined(typeof(DownloadPhase), phase) ? phase : DownloadPhase.Failed;
        }

        internal static TakanawaStatus NormalizeStatus(TakanawaStatus status)
        {
            return Enum.IsDefined(typeof(TakanawaStatus), status) ? status : TakanawaStatus.Internal;
        }
    }
}
