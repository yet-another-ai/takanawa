# Tauri

Use `takanawa-tauri` in Tauri v2 desktop apps. The frontend package shares the
same TypeScript surface as `takanawa-node`, while the Rust plugin compiles into
your Tauri application and calls the Takanawa HTTP engine directly.

## Install

Install the frontend package:

```sh
pnpm add takanawa-tauri @tauri-apps/api
```

Add the Rust plugin crate in `src-tauri/Cargo.toml`:

```toml
[dependencies]
takanawa-tauri = { package = "tauri-plugin-takanawa", version = "0.5.0" }
```

Register the plugin in your Tauri builder:

```rust
tauri::Builder::default()
    .plugin(takanawa_tauri::init())
    .run(tauri::generate_context!())
    .expect("error while running Tauri application");
```

Enable the plugin and event permissions in your app capability:

```json
{
  "permissions": ["takanawa:default", "core:event:default"]
}
```

If your Tauri capability restricts filesystem paths, include the directory that
will contain `targetPath`.

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

`new DownloadTask(options)` is synchronous, but native task creation is lazy.
Methods that cross the Tauri bridge return promises: `start`, `pause`,
`cancel`, `snapshot`, `bitmap`, `close`, and `addProgressListener`.

Large snapshot counters are returned as `bigint`. The Tauri bridge transports
them as decimal strings and transports `bitmap()` data as base64 before the
frontend converts it back to `Uint8Array`.

## Target Paths

`targetPath` is resolved by the Rust backend as a native path. In a Tauri app,
prefer paths created with `@tauri-apps/api/path`, such as `downloadDir()` or
`appDataDir()`, then join your filename before passing it to Takanawa.

## Local Development

Build and test the JavaScript package:

```sh
pnpm --filter takanawa-tauri test
```

Run Rust plugin tests:

```sh
cargo test -p takanawa-tauri
```
