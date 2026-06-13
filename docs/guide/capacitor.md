# Capacitor

Use `takanawa-capacitor` in Capacitor v8 apps that target Android or iOS. The
TypeScript surface is shared with `takanawa-node`, while the native work is
delegated to the Android AAR and SwiftPM package.

## Install

Install the plugin in the Capacitor app:

```sh
pnpm add takanawa-capacitor
npx cap sync
```

The package has a peer dependency on `@capacitor/core >=8 <9`. Its Android
bridge source, iOS bridge source, and `Takanawa.xcframework` are shipped in the
npm package; it does not require a separate Maven or SwiftPM package for the
plugin layer.

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

`new DownloadTask(options)` is synchronous, but native task creation is lazy.
Methods that cross the Capacitor bridge return promises: `start`, `pause`,
`cancel`, `snapshot`, `bitmap`, `close`, and `addProgressListener`.

`bitmap()` resolves to `Uint8Array`; the native bridge transports the bitmap as
base64 internally. Large snapshot counters are returned as `bigint`.

`maxIo` keeps the Node option name. On mobile it configures the shared native
runtime with Node-like defaults: omitted uses `4`, `0` becomes `1`, and positive
values are passed through.

## Target Paths

`targetPath` is resolved by the native platform. In a Capacitor app, prefer
paths created from the platform filesystem APIs instead of browser-only URLs.
The web fallback registers the plugin but rejects download operations because
the native downloader is Android/iOS only.

## Local Development

Build and test the JavaScript package:

```sh
pnpm --filter takanawa-capacitor test
```

Run Android plugin unit tests after dependencies are installed:

```sh
./gradlew -Ptakanawa.skipRustBuild=true :takanawa-capacitor:test
```

iOS plugin builds are verified in CI with an Apple SDK available:

```sh
mise run test:capacitor-ios
```
