import test from 'node:test'
import assert from 'node:assert/strict'

import { DownloadTask, downloadToCompletion } from '../dist/index.mjs'
import {
  decodeBase64ToUint8Array,
  mapSnapshot,
  normalizeHashKind,
  normalizeOptions
} from '../dist/testing.mjs'

test('exports public API', () => {
  assert.equal(typeof DownloadTask, 'function')
  assert.equal(typeof downloadToCompletion, 'function')
  for (const method of [
    'start',
    'pause',
    'cancel',
    'snapshot',
    'bitmap',
    'close',
    'addProgressListener'
  ]) {
    assert.equal(typeof DownloadTask.prototype[method], 'function')
  }
})

test('normalizes node-like download options for capacitor native bridge', () => {
  const options = normalizeOptions({
    url: 'https://example.test/file.bin',
    targetPath: '/tmp/file.bin',
    chunkSize: 10n,
    bytesPerSecondLimit: '2000',
    maxIo: 0,
    hash: 'sha-512:abc123'
  })

  assert.equal(options.chunkSize, '10')
  assert.equal(options.bytesPerSecondLimit, '2000')
  assert.equal(options.maxIo, 1)
  assert.deepEqual(options.hash, { kind: 'sha512', expected: 'abc123' })
})

test('normalizes hash kind aliases', () => {
  assert.equal(normalizeHashKind('sha-1'), 'sha1')
  assert.equal(normalizeHashKind('sha-256'), 'sha256')
  assert.equal(normalizeHashKind('crc-32'), 'crc32')
})

test('rejects conflicting hash options and unsafe numeric u64 values', () => {
  assert.throws(
    () =>
      normalizeOptions({
        url: 'https://example.test/file.bin',
        targetPath: '/tmp/file.bin',
        hash: { kind: 'sha256', expected: 'abc123' },
        sha256: 'abc123'
      }),
    /use either hash or sha256/
  )
  assert.throws(
    () =>
      normalizeOptions({
        url: 'https://example.test/file.bin',
        targetPath: '/tmp/file.bin',
        chunkSize: Number.MAX_SAFE_INTEGER + 1
      }),
    /safe integer/
  )
})

test('maps native snapshots to bigint public snapshots', () => {
  const snapshot = mapSnapshot({
    phase: 'running',
    contentLen: '9007199254740993',
    downloadedBytes: '10',
    chunkSize: '5',
    chunkCount: '2',
    completedChunks: '1',
    activeIo: 1,
    lastError: undefined
  })

  assert.equal(snapshot.phase, 'running')
  assert.equal(snapshot.contentLen, 9007199254740993n)
  assert.equal(snapshot.downloadedBytes, 10n)
})

test('decodes base64 bitmaps', () => {
  assert.deepEqual([...decodeBase64ToUint8Array('AQIDBA==')], [1, 2, 3, 4])
})
