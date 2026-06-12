using System;
using System.Runtime.InteropServices;

namespace YetAnotherAI.Takanawa
{
    internal static class NativeMethods
    {
        internal const uint AbiVersion = 1;
        private const string LibraryName = "takanawa_ffi";

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        internal delegate void ProgressCallback(IntPtr snapshot, IntPtr context);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        internal delegate void SpeedCallback(IntPtr snapshot, IntPtr context);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        internal delegate void CallbackRelease(IntPtr context);

        [DllImport(LibraryName, EntryPoint = "tknw_global_init", CallingConvention = CallingConvention.Cdecl)]
        internal static extern TakanawaStatus GlobalInit(ref TknwGlobalConfig config);

        [DllImport(LibraryName, EntryPoint = "tknw_global_set_max_io", CallingConvention = CallingConvention.Cdecl)]
        internal static extern TakanawaStatus GlobalSetMaxIo(UIntPtr maxIo);

        [DllImport(LibraryName, EntryPoint = "tknw_global_shutdown", CallingConvention = CallingConvention.Cdecl)]
        internal static extern TakanawaStatus GlobalShutdown();

        [DllImport(LibraryName, EntryPoint = "tknw_download_create", CallingConvention = CallingConvention.Cdecl)]
        internal static extern TakanawaStatus DownloadCreate(ref TknwDownloadConfig config, out IntPtr download);

        [DllImport(LibraryName, EntryPoint = "tknw_download_start", CallingConvention = CallingConvention.Cdecl)]
        internal static extern TakanawaStatus DownloadStart(IntPtr download);

        [DllImport(LibraryName, EntryPoint = "tknw_download_pause", CallingConvention = CallingConvention.Cdecl)]
        internal static extern TakanawaStatus DownloadPause(IntPtr download);

        [DllImport(LibraryName, EntryPoint = "tknw_download_cancel", CallingConvention = CallingConvention.Cdecl)]
        internal static extern TakanawaStatus DownloadCancel(IntPtr download);

        [DllImport(LibraryName, EntryPoint = "tknw_download_snapshot", CallingConvention = CallingConvention.Cdecl)]
        internal static extern TakanawaStatus DownloadSnapshot(IntPtr download, ref TknwDownloadSnapshot snapshot);

        [DllImport(LibraryName, EntryPoint = "tknw_download_set_progress_callback", CallingConvention = CallingConvention.Cdecl)]
        internal static extern TakanawaStatus DownloadSetProgressCallback(
            IntPtr download,
            ProgressCallback? callback,
            IntPtr context,
            CallbackRelease? contextRelease);

        [DllImport(LibraryName, EntryPoint = "tknw_download_set_speed_callback", CallingConvention = CallingConvention.Cdecl)]
        internal static extern TakanawaStatus DownloadSetSpeedCallback(
            IntPtr download,
            SpeedCallback? callback,
            IntPtr context,
            CallbackRelease? contextRelease);

        [DllImport(LibraryName, EntryPoint = "tknw_download_copy_bitmap", CallingConvention = CallingConvention.Cdecl)]
        internal static extern TakanawaStatus DownloadCopyBitmap(
            IntPtr download,
            IntPtr buffer,
            UIntPtr bufferLen,
            out UIntPtr written);

        [DllImport(LibraryName, EntryPoint = "tknw_download_last_error", CallingConvention = CallingConvention.Cdecl)]
        internal static extern TakanawaStatus DownloadLastError(
            IntPtr download,
            IntPtr buffer,
            UIntPtr bufferLen,
            out UIntPtr written);

        [DllImport(LibraryName, EntryPoint = "tknw_download_release", CallingConvention = CallingConvention.Cdecl)]
        internal static extern TakanawaStatus DownloadRelease(ref IntPtr download);

        [StructLayout(LayoutKind.Sequential)]
        internal struct TknwGlobalConfig
        {
            public uint AbiVersion;
            public UIntPtr StructSize;
            public UIntPtr MaxIo;
        }

        [StructLayout(LayoutKind.Sequential)]
        internal struct TknwDownloadConfig
        {
            public uint AbiVersion;
            public UIntPtr StructSize;
            public IntPtr Url;
            public IntPtr TargetPath;
            public ulong ChunkSize;
            public UIntPtr Parallelism;
            public UIntPtr MaxParallelChunks;
            public uint MaxRetries;
            public ulong BackoffInitialMillis;
            public ulong BackoffMaxMillis;
            public ulong ConnectTimeoutMillis;
            public ulong ReadTimeoutMillis;
            public ulong TotalTimeoutMillis;
            public ulong BytesPerSecondLimit;
            public uint HashKind;
            public IntPtr ExpectedSha256;
            public UIntPtr ExpectedSha256Len;
        }

        [StructLayout(LayoutKind.Sequential)]
        internal struct TknwDownloadSnapshot
        {
            public uint AbiVersion;
            public UIntPtr StructSize;
            public uint Phase;
            public ulong ContentLen;
            public ulong DownloadedBytes;
            public ulong ChunkSize;
            public ulong ChunkCount;
            public ulong CompletedChunks;
            public UIntPtr ActiveIo;
        }

        [StructLayout(LayoutKind.Sequential)]
        internal struct TknwDownloadSpeedSnapshot
        {
            public uint AbiVersion;
            public UIntPtr StructSize;
            public uint Phase;
            public ulong ContentLen;
            public ulong ReceivedBytes;
            public ulong IntervalBytes;
            public ulong ElapsedMillis;
            public double BytesPerSecond;
            public UIntPtr ActiveIo;
        }
    }
}
