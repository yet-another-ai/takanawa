# C# and NuGet

Use `YetAnotherAI.Takanawa` for desktop .NET applications, Unity, Godot C#
projects, and .NET Android or iOS applications. The package exposes a
`netstandard2.0` managed API and ships native assets built from the Rust FFI
library.

## Install

Add the NuGet package:

```xml
<PackageReference Include="YetAnotherAI.Takanawa" Version="{{ takanawaVersion }}" />
```

Unity and Godot projects can install the package through their NuGet workflow or
copy the package into their normal managed dependency pipeline.

## Usage

```csharp
using YetAnotherAI.Takanawa;

Takanawa.Init(maxIo: 4);

using var download = TakanawaDownload.Create(new DownloadConfig(
    url: "https://example.com/file.zip",
    targetPath: "/tmp/file.zip",
    parallelism: 4,
    hashKind: HashKind.Sha256,
    expectedHash: expectedSha256Bytes));

download.SetProgressCallback(snapshot =>
{
    Console.WriteLine($"{snapshot.Phase}: {snapshot.DownloadedBytes}/{snapshot.ContentLen}");
});

download.SetSpeedCallback(snapshot =>
{
    Console.WriteLine($"{snapshot.BytesPerSecond} B/s");
});

download.Start();
```

Dispose each `TakanawaDownload` when it is no longer needed. Call
`Takanawa.Shutdown()` when the process or app domain is done using Takanawa.

## Native Assets

The NuGet package includes 64-bit native runtime assets for Windows, macOS,
Linux, and Android. Apple mobile builds consume the packaged
`Takanawa.xcframework` through the package's transitive MSBuild target.

## Local Development

Run the C# binding checks:

```sh
mise run test:csharp
```

Pack and verify the NuGet package after staging release native artifacts:

```sh
mise run pack:csharp
```
