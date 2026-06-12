using System;
using YetAnotherAI.Takanawa;

Takanawa.Init();
try
{
    var config = new DownloadConfig(
        url: "https://example.com/file.bin",
        targetPath: System.IO.Path.Combine(System.IO.Path.GetTempPath(), $"takanawa-csharp-smoke-{Guid.NewGuid():N}.bin"));

    using var download = TakanawaDownload.Create(config);
    var snapshot = download.Snapshot();
    if (snapshot.Phase != DownloadPhase.Created)
    {
        throw new InvalidOperationException($"expected Created snapshot, got {snapshot.Phase}");
    }

    _ = download.CopyBitmap();
}
finally
{
    Takanawa.Shutdown();
}
