# YetAnotherAI.Takanawa

C# binding for the Takanawa Rust range-download library.

```csharp
using YetAnotherAI.Takanawa;

Takanawa.Init();
using var download = TakanawaDownload.Create(new DownloadConfig(
    url: "https://example.com/file.bin",
    targetPath: "/tmp/file.bin"));

download.SetProgressCallback(snapshot =>
    Console.WriteLine($"{snapshot.Phase}: {snapshot.DownloadedBytes}/{snapshot.ContentLen}"));
download.Start();
Takanawa.Shutdown();
```

The package targets `netstandard2.0` and includes native assets for desktop,
Android, and Apple targets when built from the release artifact pipeline.
