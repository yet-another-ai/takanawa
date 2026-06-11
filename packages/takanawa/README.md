# @yetanother.ai/takanawa

Node.js and Electron bindings for Takanawa, built with napi-rs and wrapped with a Vite + TypeScript API.

## Development

```sh
pnpm --filter @yetanother.ai/takanawa build
```

The package builds the native Node-API addon first, then emits the TypeScript wrapper as both ESM and CommonJS.

## Usage

```ts
import { DownloadTask, downloadToCompletion } from '@yetanother.ai/takanawa'

await downloadToCompletion({
  url: 'https://example.com/file.zip',
  targetPath: '/tmp/file.zip',
  parallelism: 4
})

const task = new DownloadTask({
  url: 'https://example.com/file.zip',
  targetPath: '/tmp/file.zip'
})

task.start()
console.log(task.snapshot())
```

Large byte counts are exposed as `bigint` in the TypeScript API so Node and Electron callers do not lose precision.
