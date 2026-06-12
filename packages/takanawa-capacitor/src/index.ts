import { registerPlugin, type PluginListenerHandle } from '@capacitor/core'

import type { TakanawaCapacitorPlugin } from './definitions'
import {
  decodeBase64ToUint8Array,
  mapSnapshot,
  normalizeOptions,
  type DownloadOptions,
  type DownloadPhase,
  type DownloadSnapshot,
  type HashConfig,
  type HashKind
} from './shared'

const TakanawaCapacitor = registerPlugin<TakanawaCapacitorPlugin>('TakanawaCapacitor', {
  web: () => import('./web').then((module) => new module.TakanawaCapacitorWeb())
})

export type { DownloadOptions, DownloadPhase, DownloadSnapshot, HashConfig, HashKind }

export class DownloadTask {
  readonly #options: DownloadOptions
  #taskId?: string
  #taskIdPromise?: Promise<string>
  #closed = false

  constructor(options: DownloadOptions) {
    this.#options = options
  }

  async start(): Promise<void> {
    const taskId = await this.#ensureTaskId()
    await TakanawaCapacitor.start({ taskId })
  }

  async pause(): Promise<void> {
    const taskId = await this.#ensureTaskId()
    await TakanawaCapacitor.pause({ taskId })
  }

  async cancel(): Promise<void> {
    const taskId = await this.#ensureTaskId()
    await TakanawaCapacitor.cancel({ taskId })
  }

  async snapshot(): Promise<DownloadSnapshot> {
    const taskId = await this.#ensureTaskId()
    const { snapshot } = await TakanawaCapacitor.snapshot({ taskId })
    return mapSnapshot(snapshot)
  }

  async bitmap(): Promise<Uint8Array> {
    const taskId = await this.#ensureTaskId()
    const { data } = await TakanawaCapacitor.bitmap({ taskId })
    return decodeBase64ToUint8Array(data)
  }

  async close(): Promise<void> {
    if (this.#closed) {
      return
    }
    this.#closed = true
    const taskIdPromise = this.#taskIdPromise
    this.#taskIdPromise = undefined
    this.#taskId = undefined
    if (taskIdPromise === undefined) {
      return
    }
    const taskId = await taskIdPromise
    await TakanawaCapacitor.close({ taskId })
  }

  async addProgressListener(
    listener: (snapshot: DownloadSnapshot) => void
  ): Promise<PluginListenerHandle> {
    const taskId = await this.#ensureTaskId()
    const handle = await TakanawaCapacitor.addListener('downloadProgress', (event) => {
      if (event.taskId === taskId) {
        listener(mapSnapshot(event.snapshot))
      }
    })
    try {
      listener(await this.snapshot())
    } catch (error) {
      await handle.remove()
      throw error
    }
    return handle
  }

  async #ensureTaskId(): Promise<string> {
    if (this.#closed) {
      throw new Error('download task is closed')
    }
    if (this.#taskId !== undefined) {
      return this.#taskId
    }
    if (this.#taskIdPromise === undefined) {
      this.#taskIdPromise = TakanawaCapacitor.create(normalizeOptions(this.#options)).then(
        ({ taskId }) => {
          this.#taskId = taskId
          return taskId
        },
        (error: unknown) => {
          this.#taskIdPromise = undefined
          throw error
        }
      )
    }
    return this.#taskIdPromise
  }
}

export async function downloadToCompletion(options: DownloadOptions): Promise<DownloadSnapshot> {
  const { snapshot } = await TakanawaCapacitor.downloadToCompletion(normalizeOptions(options))
  return mapSnapshot(snapshot)
}
