import { WebPlugin } from '@capacitor/core'

import type {
  NativeBitmapResult,
  NativeDownloadOptions,
  NativeSnapshotResult,
  NativeTaskOptions,
  NativeTaskResult,
  TakanawaCapacitorPlugin
} from './definitions'

export class TakanawaCapacitorWeb extends WebPlugin implements TakanawaCapacitorPlugin {
  create(_options: NativeDownloadOptions): Promise<NativeTaskResult> {
    return unsupported()
  }

  start(_options: NativeTaskOptions): Promise<void> {
    return unsupported()
  }

  pause(_options: NativeTaskOptions): Promise<void> {
    return unsupported()
  }

  cancel(_options: NativeTaskOptions): Promise<void> {
    return unsupported()
  }

  snapshot(_options: NativeTaskOptions): Promise<NativeSnapshotResult> {
    return unsupported()
  }

  bitmap(_options: NativeTaskOptions): Promise<NativeBitmapResult> {
    return unsupported()
  }

  close(_options: NativeTaskOptions): Promise<void> {
    return unsupported()
  }

  downloadToCompletion(_options: NativeDownloadOptions): Promise<NativeSnapshotResult> {
    return unsupported()
  }
}

function unsupported(): Promise<never> {
  return Promise.reject(new Error('takanawa-capacitor is only available on Android and iOS'))
}
