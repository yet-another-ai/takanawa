import { defineConfig } from 'vite'

const extensionByFormat = {
  es: 'mjs',
  cjs: 'cjs'
} as const

export default defineConfig({
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
      external: ['@capacitor/core']
    },
    sourcemap: true
  }
})
