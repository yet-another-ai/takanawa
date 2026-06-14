import { readFileSync } from 'node:fs'
import { defineConfig } from 'vitepress'

const workspaceCargoToml = readFileSync(new URL('../../Cargo.toml', import.meta.url), 'utf8')
const takanawaVersion = workspaceCargoToml.match(
  /^\[workspace\.package\][\s\S]*?^version = "([^"]+)"/m,
)?.[1]

if (!takanawaVersion) {
  throw new Error('missing [workspace.package] version in Cargo.toml')
}

const replaceMarkdownVariables = (source: string) =>
  source.replace(/\{\{\s*takanawaVersion\s*\}\}/g, takanawaVersion)

export default defineConfig({
  title: 'Takanawa',
  description: 'A Rust range-download library for resilient cross-platform downloads.',
  cleanUrls: true,
  lastUpdated: true,
  markdown: {
    config(md) {
      md.core.ruler.before('normalize', 'takanawa-variables', (state) => {
        state.src = replaceMarkdownVariables(state.src)
      })
    },
  },
  themeConfig: {
    logo: '/logo.svg',
    nav: [
      { text: 'Guide', link: '/guide/getting-started' },
      { text: 'Targets', link: '/guide/platforms' },
      { text: 'Format', link: '/part-format' },
      { text: 'API Docs', link: '/api/' },
    ],
    sidebar: [
      {
        text: 'Guide',
        items: [
          { text: 'Overview', link: '/' },
          { text: 'Getting Started', link: '/guide/getting-started' },
        ],
      },
      {
        text: 'Targets',
        items: [
          { text: 'Target Matrix', link: '/guide/platforms' },
          { text: 'Rust', link: '/guide/rust' },
          { text: 'Node and Electron', link: '/guide/node' },
          { text: 'Capacitor', link: '/guide/capacitor' },
          { text: 'Tauri', link: '/guide/tauri' },
          { text: 'Godot GDExtension', link: '/guide/gdextension' },
          { text: 'Android', link: '/guide/android' },
          { text: 'Apple and SwiftPM', link: '/guide/apple' },
          { text: 'C# and NuGet', link: '/guide/csharp' },
          { text: 'C and C++', link: '/guide/c-cpp' },
        ],
      },
      {
        text: 'Internals',
        items: [
          { text: '.part Format', link: '/part-format' },
        ],
      },
      {
        text: 'API Docs',
        items: [
          { text: 'Overview', link: '/api/' },
        ],
      },
    ],
    search: {
      provider: 'local',
    },
    socialLinks: [
      { icon: 'github', link: 'https://github.com/yet-another-ai/takanawa' },
    ],
    footer: {
      message: 'Released under the MIT License.',
      copyright: 'Copyright (c) 2026 yetanother.ai',
    },
  },
})
