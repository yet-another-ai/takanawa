using System;
using System.Text;

namespace YetAnotherAI.Takanawa
{
    internal static class NativeBuffers
    {
        internal static byte[] CopyBitmap(IntPtr handle)
        {
            var status = NativeMethods.DownloadCopyBitmap(handle, IntPtr.Zero, UIntPtr.Zero, out var written);
            var required = checked((int)written.ToUInt64());
            if (status != TakanawaStatus.BufferTooSmall)
            {
                Takanawa.CheckStatus(status, handle);
                return Array.Empty<byte>();
            }

            if (required == 0)
            {
                return Array.Empty<byte>();
            }

            var buffer = new byte[required];
            unsafe
            {
                fixed (byte* bufferPtr = buffer)
                {
                    Takanawa.CheckStatus(
                        NativeMethods.DownloadCopyBitmap(handle, (IntPtr)bufferPtr, new UIntPtr((uint)buffer.Length), out _),
                        handle);
                }
            }

            return buffer;
        }

        internal static string ReadLastError(IntPtr handle)
        {
            var status = NativeMethods.DownloadLastError(handle, IntPtr.Zero, UIntPtr.Zero, out var written);
            var required = checked((int)written.ToUInt64());
            if (status != TakanawaStatus.BufferTooSmall || required == 0)
            {
                return string.Empty;
            }

            var buffer = new byte[required];
            unsafe
            {
                fixed (byte* bufferPtr = buffer)
                {
                    status = NativeMethods.DownloadLastError(handle, (IntPtr)bufferPtr, new UIntPtr((uint)buffer.Length), out _);
                }
            }

            if (status != TakanawaStatus.Ok)
            {
                return string.Empty;
            }

            var length = Array.IndexOf(buffer, (byte)0);
            if (length < 0)
            {
                length = buffer.Length;
            }

            return Encoding.UTF8.GetString(buffer, 0, length);
        }
    }
}
