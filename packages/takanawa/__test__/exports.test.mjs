import test from 'node:test'
import assert from 'node:assert/strict'

import { DownloadTask, downloadToCompletion } from '../dist/index.mjs'

test('exports public API', () => {
  assert.equal(typeof DownloadTask, 'function')
  assert.equal(typeof downloadToCompletion, 'function')
})
