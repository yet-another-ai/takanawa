import { resolve } from 'node:path'

import { defineConfig } from 'vite'

const extensionByFormat = {
  es: 'mjs',
  cjs: 'cjs'
} as const

export default defineConfig({
  resolve: {
    alias: {
      'takanawa-js-core': resolve(__dirname, '../takanawa-js-core/src/index.ts')
    }
  },
  build: {
    lib: {
      entry: {
        index: 'src/index.ts',
        testing: 'src/testing.ts'
      },
      formats: ['es', 'cjs'],
      fileName: (format, entryName) => `${entryName}.${extensionByFormat[format as 'es' | 'cjs']}`
    },
    rollupOptions: {
      external: ['@tauri-apps/api/core', '@tauri-apps/api/event']
    },
    sourcemap: true
  }
})
