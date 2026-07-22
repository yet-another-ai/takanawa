# Node and Electron

Use `takanawa-node` for Node.js and Electron runtimes. The package ships a
Node-API native addon plus ESM and CommonJS TypeScript wrappers.

## Install

Install from npm:

```sh
pnpm add takanawa-node
```

Other package managers work as usual:

```sh
npm install takanawa-node
yarn add takanawa-node
```

## Usage

```ts
import { DownloadTask, downloadToCompletion } from 'takanawa-node'

await downloadToCompletion({
  url: 'https://example.com/file.zip',
  targetPath: '/tmp/file.zip',
  parallelism: 4,
  hash: {
    kind: 'sha256',
    expected: '...64 hex characters...'
  }
})

const task = new DownloadTask({
  url: 'https://example.com/file.zip',
  targetPath: '/tmp/file.zip'
})

const progress = await task.addProgressListener((snapshot) => {
  console.log(snapshot.phase, snapshot.downloadedBytes, snapshot.contentLen)
})
const speed = await task.addSpeedListener((snapshot) => {
  console.log(snapshot.bytesPerSecond, snapshot.receivedBytes)
})

await task.start()
console.log(await task.snapshot())

await progress.remove()
await speed.remove()
await task.close()
```

The public API matches the other Takanawa npm targets. Task creation is lazy,
and task methods return promises: `start`, `pause`, `cancel`, `snapshot`,
`bitmap`, `close`, `addProgressListener`, and `addSpeedListener`.

Large snapshot counters are returned as `bigint`. Hash verification supports
`sha1`, `sha256`, `sha512`, `md5`, and `crc32` with either
`hash: { kind, expected }` or a compact string such as `sha512:<hex>`.

## Local Development

Build the native addon and TypeScript wrapper:

```sh
pnpm --filter takanawa-node build
```

Run the package tests:

```sh
pnpm --filter takanawa-node test
```
