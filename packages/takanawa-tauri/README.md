# takanawa-tauri

Tauri v2 bindings for Takanawa, built as a Tauri plugin and wrapped with the
shared Takanawa TypeScript API.

## Install

Install the frontend package in your Tauri app:

```sh
pnpm add takanawa-tauri @tauri-apps/api
```

Add the Rust plugin crate to `src-tauri/Cargo.toml`:

```toml
[dependencies]
takanawa-tauri = { package = "tauri-plugin-takanawa", version = "0.5.0" }
```

Register the plugin in the Tauri builder:

```rust
tauri::Builder::default()
    .plugin(takanawa_tauri::init())
    .run(tauri::generate_context!())
    .expect("error while running Tauri application");
```

Enable the default plugin permission in your app capability:

```json
{
  "permissions": ["takanawa:default", "core:event:default"]
}
```

## Usage

```ts
import { DownloadTask, downloadToCompletion } from 'takanawa-tauri'

await downloadToCompletion({
  url: 'https://example.com/file.zip',
  targetPath: '/path/to/file.zip',
  parallelism: 4,
  hash: {
    kind: 'sha256',
    expected: '...64 hex characters...'
  }
})

const task = new DownloadTask({
  url: 'https://example.com/file.zip',
  targetPath: '/path/to/file.zip'
})

const progress = await task.addProgressListener((snapshot) => {
  console.log(snapshot.phase, snapshot.downloadedBytes, snapshot.contentLen)
})

await task.start()
console.log(await task.snapshot())

await progress.remove()
await task.close()
```

The public TypeScript API matches `takanawa-node`: task creation is lazy, task
methods return promises, and large byte counts are exposed as `bigint`. The
Tauri bridge transports counters as decimal strings and bitmaps as base64.

`targetPath` is handled by the Rust backend as a native filesystem path. Prefer
paths created through Tauri path APIs such as `downloadDir()` or `appDataDir()`
and make sure your app capabilities allow the directory you write to.
