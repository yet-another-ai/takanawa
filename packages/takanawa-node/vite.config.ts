import { resolve } from 'node:path'

import { defineConfig } from 'vite'

export default defineConfig({
  resolve: {
    alias: {
      'takanawa-js-core': resolve(__dirname, '../takanawa-js-core/src/index.ts')
    }
  },
  build: {
    lib: {
      entry: 'src/index.ts',
      formats: ['es', 'cjs'],
      fileName: (format) => (format === 'es' ? 'index.mjs' : 'index.cjs')
    },
    rolldownOptions: {
      external: ['node:module']
    },
    sourcemap: true
  }
})
