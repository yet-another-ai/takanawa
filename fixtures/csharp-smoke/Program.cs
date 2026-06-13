using System;
using System.IO;
using System.Text;
using System.Threading;
using YetAnotherAI.Takanawa;

var url = Environment.GetEnvironmentVariable("TAKANAWA_TEST_URL")
    ?? throw new InvalidOperationException("TAKANAWA_TEST_URL is required");
var expected = Environment.GetEnvironmentVariable("TAKANAWA_TEST_EXPECTED_BYTES")
    ?? throw new InvalidOperationException("TAKANAWA_TEST_EXPECTED_BYTES is required");
var targetPath = Path.Combine(Path.GetTempPath(), $"takanawa-csharp-smoke-{Guid.NewGuid():N}.bin");

Takanawa.Init();
try
{
    var config = new DownloadConfig(
        url: url,
        targetPath: targetPath,
        chunkSize: 5,
        parallelism: 2,
        maxRetries: 0);

    using var download = TakanawaDownload.Create(config);
    var snapshot = download.Snapshot();
    if (snapshot.Phase != DownloadPhase.Created)
    {
        throw new InvalidOperationException($"expected Created snapshot, got {snapshot.Phase}");
    }

    _ = download.CopyBitmap();
    download.Start();
    snapshot = WaitForCompletion(download);
    if (snapshot.ContentLen != Encoding.UTF8.GetByteCount(expected))
    {
        throw new InvalidOperationException($"expected content length {expected.Length}, got {snapshot.ContentLen}");
    }

    var actual = File.ReadAllText(targetPath, Encoding.UTF8);
    if (!string.Equals(actual, expected, StringComparison.Ordinal))
    {
        throw new InvalidOperationException("downloaded bytes did not match expected payload");
    }
}
finally
{
    if (File.Exists(targetPath))
    {
        File.Delete(targetPath);
    }
    Takanawa.Shutdown();
}

static DownloadSnapshot WaitForCompletion(TakanawaDownload download)
{
    for (var attempt = 0; attempt < 250; attempt++)
    {
        var snapshot = download.Snapshot();
        if (snapshot.Phase == DownloadPhase.Completed)
        {
            return snapshot;
        }
        if (snapshot.Phase == DownloadPhase.Failed)
        {
            throw new InvalidOperationException($"download failed: {download.LastError()}");
        }
        Thread.Sleep(TimeSpan.FromMilliseconds(20));
    }

    throw new TimeoutException("download did not complete in time");
}
