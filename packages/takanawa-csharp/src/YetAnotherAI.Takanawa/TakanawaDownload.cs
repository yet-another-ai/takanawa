using System;
using System.Text;
using System.Runtime.InteropServices;

namespace YetAnotherAI.Takanawa
{
    public sealed class TakanawaDownload : IDisposable
    {
        private static readonly NativeMethods.ProgressCallback ProgressTrampoline = OnProgress;
        private static readonly NativeMethods.SpeedCallback SpeedTrampoline = OnSpeed;
        private static readonly NativeMethods.CallbackRelease CallbackRelease = OnCallbackRelease;

        private readonly object syncRoot = new object();
        private TakanawaDownloadHandle handle;
        private bool disposed;

        internal TakanawaDownload(TakanawaDownloadHandle handle)
        {
            this.handle = handle;
        }

        public static TakanawaDownload Create(DownloadConfig config)
        {
            if (config == null)
            {
                throw new ArgumentNullException(nameof(config));
            }

            var url = NullTerminatedUtf8(config.Url);
            var targetPath = NullTerminatedUtf8(config.TargetPath);
            var expectedHash = config.ExpectedHashCopy();

            unsafe
            {
                fixed (byte* urlPtr = url)
                fixed (byte* targetPathPtr = targetPath)
                fixed (byte* expectedHashPtr = expectedHash)
                {
                    var native = new NativeMethods.TknwDownloadConfig
                    {
                        AbiVersion = NativeMethods.AbiVersion,
                        StructSize = NativeConversions.SizeOf<NativeMethods.TknwDownloadConfig>(),
                        Url = (IntPtr)urlPtr,
                        TargetPath = (IntPtr)targetPathPtr,
                        ChunkSize = NativeConversions.ToUInt64(config.ChunkSize, nameof(config.ChunkSize)),
                        Parallelism = NativeConversions.ToUIntPtr(config.Parallelism, nameof(config.Parallelism)),
                        MaxParallelChunks = NativeConversions.ToUIntPtr(config.MaxParallelChunks, nameof(config.MaxParallelChunks)),
                        MaxRetries = NativeConversions.ToUInt32(config.MaxRetries, nameof(config.MaxRetries)),
                        BackoffInitialMillis = NativeConversions.ToUInt64(config.BackoffInitialMillis, nameof(config.BackoffInitialMillis)),
                        BackoffMaxMillis = NativeConversions.ToUInt64(config.BackoffMaxMillis, nameof(config.BackoffMaxMillis)),
                        ConnectTimeoutMillis = NativeConversions.ToUInt64(config.ConnectTimeoutMillis, nameof(config.ConnectTimeoutMillis)),
                        ReadTimeoutMillis = NativeConversions.ToUInt64(config.ReadTimeoutMillis, nameof(config.ReadTimeoutMillis)),
                        TotalTimeoutMillis = NativeConversions.ToUInt64(config.TotalTimeoutMillis, nameof(config.TotalTimeoutMillis)),
                        BytesPerSecondLimit = NativeConversions.ToUInt64(config.BytesPerSecondLimit, nameof(config.BytesPerSecondLimit)),
                        HashKind = (uint)config.HashKind,
                        ExpectedSha256 = expectedHash == null ? IntPtr.Zero : (IntPtr)expectedHashPtr,
                        ExpectedSha256Len = expectedHash == null ? UIntPtr.Zero : new UIntPtr((uint)expectedHash.Length),
                    };

                    Takanawa.CheckStatus(NativeMethods.DownloadCreate(ref native, out var nativeHandle));
                    if (nativeHandle == IntPtr.Zero)
                    {
                        throw new TakanawaException(TakanawaStatus.NullPointer, "native download handle was not returned");
                    }

                    return new TakanawaDownload(new TakanawaDownloadHandle(nativeHandle));
                }
            }
        }

        public void Start()
        {
            WithHandle(current => Takanawa.CheckStatus(NativeMethods.DownloadStart(current), current));
        }

        public void Pause()
        {
            WithHandle(current => Takanawa.CheckStatus(NativeMethods.DownloadPause(current), current));
        }

        public void Cancel()
        {
            WithHandle(current => Takanawa.CheckStatus(NativeMethods.DownloadCancel(current), current));
        }

        public DownloadSnapshot Snapshot()
        {
            return WithHandle(current =>
            {
                var native = new NativeMethods.TknwDownloadSnapshot
                {
                    AbiVersion = NativeMethods.AbiVersion,
                    StructSize = NativeConversions.SizeOf<NativeMethods.TknwDownloadSnapshot>(),
                };
                Takanawa.CheckStatus(NativeMethods.DownloadSnapshot(current, ref native), current);
                return new DownloadSnapshot(native);
            });
        }

        public void SetProgressCallback(Action<DownloadSnapshot>? callback)
        {
            WithHandle(current =>
            {
                ClearProgressCallback(current);
                if (callback == null)
                {
                    return;
                }

                var context = GCHandle.ToIntPtr(GCHandle.Alloc(new ProgressCallbackBox(callback)));
                var status = NativeMethods.DownloadSetProgressCallback(
                    current,
                    ProgressTrampoline,
                    context,
                    CallbackRelease);
                if (status != TakanawaStatus.Ok)
                {
                    GCHandle.FromIntPtr(context).Free();
                    Takanawa.CheckStatus(status, current);
                }
            });
        }

        public void ClearProgressCallback()
        {
            WithHandle(ClearProgressCallback);
        }

        public void SetSpeedCallback(Action<DownloadSpeedSnapshot>? callback)
        {
            WithHandle(current =>
            {
                ClearSpeedCallback(current);
                if (callback == null)
                {
                    return;
                }

                var context = GCHandle.ToIntPtr(GCHandle.Alloc(new SpeedCallbackBox(callback)));
                var status = NativeMethods.DownloadSetSpeedCallback(
                    current,
                    SpeedTrampoline,
                    context,
                    CallbackRelease);
                if (status != TakanawaStatus.Ok)
                {
                    GCHandle.FromIntPtr(context).Free();
                    Takanawa.CheckStatus(status, current);
                }
            });
        }

        public void ClearSpeedCallback()
        {
            WithHandle(ClearSpeedCallback);
        }

        public byte[] CopyBitmap()
        {
            return WithHandle(NativeBuffers.CopyBitmap);
        }

        public string LastError()
        {
            return WithHandle(NativeBuffers.ReadLastError);
        }

        public void Dispose()
        {
            lock (syncRoot)
            {
                if (disposed)
                {
                    return;
                }

                if (handle.IsInvalid)
                {
                    disposed = true;
                    GC.SuppressFinalize(this);
                    return;
                }

                var current = handle.Open();
                ClearProgressCallback(current);
                ClearSpeedCallback(current);
                handle.ReleaseOrThrow();
                disposed = true;
                GC.SuppressFinalize(this);
            }
        }

        private void ClearProgressCallback(IntPtr current)
        {
            Takanawa.CheckStatus(
                NativeMethods.DownloadSetProgressCallback(current, null, IntPtr.Zero, null),
                current);
        }

        private void ClearSpeedCallback(IntPtr current)
        {
            Takanawa.CheckStatus(
                NativeMethods.DownloadSetSpeedCallback(current, null, IntPtr.Zero, null),
                current);
        }

        private void WithHandle(Action<IntPtr> action)
        {
            lock (syncRoot)
            {
                action(OpenHandle());
            }
        }

        private TResult WithHandle<TResult>(Func<IntPtr, TResult> action)
        {
            lock (syncRoot)
            {
                return action(OpenHandle());
            }
        }

        private IntPtr OpenHandle()
        {
            if (disposed)
            {
                throw new TakanawaException(TakanawaStatus.InvalidConfig, "download is closed");
            }

            return handle.Open();
        }

        private static byte[] NullTerminatedUtf8(string value)
        {
            var bytes = Encoding.UTF8.GetBytes(value);
            var result = new byte[bytes.Length + 1];
            Buffer.BlockCopy(bytes, 0, result, 0, bytes.Length);
            return result;
        }

        private static void OnProgress(IntPtr snapshot, IntPtr context)
        {
            if (snapshot == IntPtr.Zero || context == IntPtr.Zero)
            {
                return;
            }

            try
            {
                var box = (ProgressCallbackBox)GCHandle.FromIntPtr(context).Target!;
                var native = (NativeMethods.TknwDownloadSnapshot)Marshal.PtrToStructure(
                    snapshot,
                    typeof(NativeMethods.TknwDownloadSnapshot))!;
                box.Callback(new DownloadSnapshot(native));
            }
            catch
            {
                // Managed callback exceptions cannot cross the native callback boundary.
            }
        }

        private static void OnSpeed(IntPtr snapshot, IntPtr context)
        {
            if (snapshot == IntPtr.Zero || context == IntPtr.Zero)
            {
                return;
            }

            try
            {
                var box = (SpeedCallbackBox)GCHandle.FromIntPtr(context).Target!;
                var native = (NativeMethods.TknwDownloadSpeedSnapshot)Marshal.PtrToStructure(
                    snapshot,
                    typeof(NativeMethods.TknwDownloadSpeedSnapshot))!;
                box.Callback(new DownloadSpeedSnapshot(native));
            }
            catch
            {
                // Managed callback exceptions cannot cross the native callback boundary.
            }
        }

        private static void OnCallbackRelease(IntPtr context)
        {
            if (context == IntPtr.Zero)
            {
                return;
            }

            GCHandle.FromIntPtr(context).Free();
        }

        private sealed class ProgressCallbackBox
        {
            internal ProgressCallbackBox(Action<DownloadSnapshot> callback)
            {
                Callback = callback;
            }

            internal Action<DownloadSnapshot> Callback { get; }
        }

        private sealed class SpeedCallbackBox
        {
            internal SpeedCallbackBox(Action<DownloadSpeedSnapshot> callback)
            {
                Callback = callback;
            }

            internal Action<DownloadSpeedSnapshot> Callback { get; }
        }
    }
}
