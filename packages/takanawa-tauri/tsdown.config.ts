import { defineConfig } from 'tsdown'

export default defineConfig({
  entry: {
    index: 'src/index.ts',
    testing: 'src/testing.ts'
  },
  deps: {
    neverBundle: ['@tauri-apps/api/core', '@tauri-apps/api/event']
  },
  format: ['esm', 'cjs'],
  dts: true,
  sourcemap: true,
  outExtensions: () => ({
    dts: '.d.ts'
  })
})
