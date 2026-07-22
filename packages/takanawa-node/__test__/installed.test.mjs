import test from 'node:test'
import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { DownloadTask, downloadToCompletion } from 'takanawa-node'

const require = createRequire(import.meta.url)

test('loads installed ESM and CommonJS exports', () => {
  const commonjs = require('takanawa-node')

  assert.equal(typeof DownloadTask, 'function')
  assert.equal(typeof downloadToCompletion, 'function')
  assert.equal(typeof commonjs.DownloadTask, 'function')
  assert.equal(typeof commonjs.downloadToCompletion, 'function')
})

test('calls the installed native binding', async () => {
  const task = new DownloadTask({
    url: 'http://127.0.0.1:1/file',
    targetPath: join(tmpdir(), `takanawa-node-installed-${process.pid}.tmp`)
  })

  try {
    const snapshot = await task.snapshot()
    assert.equal(snapshot.phase, 'created')
  } finally {
    await task.close()
  }
})
