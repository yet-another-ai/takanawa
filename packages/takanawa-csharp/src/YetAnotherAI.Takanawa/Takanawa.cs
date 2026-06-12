using System;

namespace YetAnotherAI.Takanawa
{
    public static class Takanawa
    {
        public static void Init(int maxIo = 0)
        {
            if (maxIo < 0)
            {
                throw new ArgumentOutOfRangeException(nameof(maxIo), maxIo, "maxIo must be greater than or equal to 0");
            }

            var config = new NativeMethods.TknwGlobalConfig
            {
                AbiVersion = NativeMethods.AbiVersion,
                StructSize = NativeConversions.SizeOf<NativeMethods.TknwGlobalConfig>(),
                MaxIo = NativeConversions.ToUIntPtr(maxIo, nameof(maxIo)),
            };
            CheckStatus(NativeMethods.GlobalInit(ref config));
        }

        public static void SetMaxIo(int maxIo)
        {
            if (maxIo < 0)
            {
                throw new ArgumentOutOfRangeException(nameof(maxIo), maxIo, "maxIo must be greater than or equal to 0");
            }

            CheckStatus(NativeMethods.GlobalSetMaxIo(NativeConversions.ToUIntPtr(maxIo, nameof(maxIo))));
        }

        public static void Shutdown()
        {
            CheckStatus(NativeMethods.GlobalShutdown());
        }

        internal static void CheckStatus(TakanawaStatus status, IntPtr handle = default)
        {
            var normalized = NativeConversions.NormalizeStatus(status);
            if (normalized == TakanawaStatus.Ok)
            {
                return;
            }

            string? nativeMessage = null;
            if (handle != IntPtr.Zero)
            {
                nativeMessage = NativeBuffers.ReadLastError(handle);
            }

            throw new TakanawaException(normalized, nativeMessage);
        }
    }
}
