import test from 'node:test'
import assert from 'node:assert/strict'

import {
  createTakanawaApi,
  decodeBase64ToUint8Array,
  mapSnapshot,
  normalizeHashKind,
  normalizeOptions
} from '../dist/index.mjs'

const nativeSnapshot = {
  phase: 'running',
  contentLen: '9007199254740993',
  downloadedBytes: '10',
  chunkSize: '5',
  chunkCount: '2',
  completedChunks: '1',
  activeIo: 1,
  lastError: undefined
}

test('normalizes node-like download options', () => {
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
  const snapshot = mapSnapshot(nativeSnapshot)

  assert.equal(snapshot.phase, 'running')
  assert.equal(snapshot.contentLen, 9007199254740993n)
  assert.equal(snapshot.downloadedBytes, 10n)
})

test('decodes base64 bitmaps', () => {
  assert.deepEqual([...decodeBase64ToUint8Array('AQIDBA==')], [1, 2, 3, 4])
})

test('injects download task facade around target adapter', async () => {
  const listenerSnapshots = []
  const calls = []
  const adapter = {
    create(options) {
      calls.push(['create', options.maxIo])
      return { options }
    },
    start(task) {
      calls.push(['start', task.options.url])
    },
    pause() {},
    cancel() {},
    snapshot() {
      return nativeSnapshot
    },
    bitmap() {
      return new Uint8Array([1, 2, 3])
    },
    close() {
      calls.push(['close'])
    },
    addProgressListener(_task, listener) {
      calls.push(['listen'])
      listener({
        ...nativeSnapshot,
        downloadedBytes: '11'
      })
      return {
        async remove() {
          calls.push(['remove'])
        }
      }
    },
    downloadToCompletion() {
      return {
        ...nativeSnapshot,
        phase: 'completed'
      }
    }
  }

  const { DownloadTask, downloadToCompletion } = createTakanawaApi(adapter)
  const task = new DownloadTask({
    url: 'https://example.test/file.bin',
    targetPath: '/tmp/file.bin'
  })

  await task.start()
  const handle = await task.addProgressListener((snapshot) => {
    listenerSnapshots.push(snapshot.downloadedBytes)
  })
  await handle.remove()
  assert.deepEqual(listenerSnapshots, [11n, 10n])
  assert.deepEqual(await task.bitmap(), new Uint8Array([1, 2, 3]))
  await task.close()
  assert.deepEqual(calls, [
    ['create', 4],
    ['start', 'https://example.test/file.bin'],
    ['listen'],
    ['remove'],
    ['close']
  ])

  const completed = await downloadToCompletion({
    url: 'https://example.test/file.bin',
    targetPath: '/tmp/file.bin'
  })
  assert.equal(completed.phase, 'completed')
})
