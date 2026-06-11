# takanawa-node

Node.js and Electron bindings for Takanawa, built with napi-rs and wrapped with a Vite + TypeScript API.

## Development

```sh
pnpm --filter takanawa-node build
```

The package builds the native Node-API addon first, then emits the TypeScript wrapper as both ESM and CommonJS.

## Usage

```ts
import { DownloadTask, downloadToCompletion } from '@yetanother.ai/takanawa'

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

task.start()
console.log(task.snapshot())
```

Large byte counts are exposed as `bigint` in the TypeScript API so Node and Electron callers do not lose precision.

`hash` supports `sha1`, `sha256`, `sha512`, `md5`, and `crc32` expected
digests. You can also pass a compact string such as `sha512:<hex>`. The legacy
`sha256` option remains available for existing callers.
