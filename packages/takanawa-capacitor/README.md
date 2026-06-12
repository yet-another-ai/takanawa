# takanawa-capacitor

Capacitor bindings for Takanawa on Android and iOS.

## Install

```sh
pnpm add takanawa-capacitor
npx cap sync
```

## Usage

```ts
import { DownloadTask, downloadToCompletion } from 'takanawa-capacitor'

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

The TypeScript API is provided by the shared Takanawa npm facade, so it matches
`takanawa-node`. Task methods return promises because they cross the native
bridge. Large byte counts are exposed as `bigint`; the native bridge transports
them as decimal strings.

The `maxIo` option follows the Node defaults at the JS boundary: omitted uses
`4`, `0` becomes `1`, and positive values are passed through. On Android and iOS
this configures the shared native Takanawa runtime.
