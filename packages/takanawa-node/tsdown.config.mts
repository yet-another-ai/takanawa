import { defineConfig } from 'tsdown'

export default defineConfig({
  entry: {
    index: 'src/index.ts'
  },
  deps: {
    neverBundle: ['node:module']
  },
  format: ['esm', 'cjs'],
  dts: true,
  sourcemap: true,
  outExtensions: () => ({
    dts: '.d.ts'
  })
})
