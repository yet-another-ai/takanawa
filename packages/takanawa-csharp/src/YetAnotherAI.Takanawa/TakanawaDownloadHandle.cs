using System;
using System.Runtime.InteropServices;

namespace YetAnotherAI.Takanawa
{
    internal sealed class TakanawaDownloadHandle : SafeHandle
    {
        private TakanawaDownloadHandle()
            : base(IntPtr.Zero, true)
        {
        }

        internal TakanawaDownloadHandle(IntPtr handle)
            : base(IntPtr.Zero, true)
        {
            SetHandle(handle);
        }

        public override bool IsInvalid => handle == IntPtr.Zero;

        internal IntPtr Open()
        {
            if (IsInvalid || IsClosed)
            {
                throw new TakanawaException(TakanawaStatus.InvalidConfig, "download is closed");
            }

            return handle;
        }

        internal void ReleaseOrThrow()
        {
            if (IsInvalid)
            {
                return;
            }

            var current = handle;
            var status = NativeMethods.DownloadRelease(ref current);
            if (status == TakanawaStatus.Ok)
            {
                SetHandle(IntPtr.Zero);
                SetHandleAsInvalid();
                return;
            }

            Takanawa.CheckStatus(status);
        }

        protected override bool ReleaseHandle()
        {
            if (handle == IntPtr.Zero)
            {
                return true;
            }

            var current = handle;
            var status = NativeMethods.DownloadRelease(ref current);
            if (status == TakanawaStatus.Ok)
            {
                handle = IntPtr.Zero;
                return true;
            }

            return false;
        }
    }
}
