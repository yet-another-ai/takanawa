import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'Takanawa',
  description: 'A Rust range-download library for resilient cross-platform downloads.',
  cleanUrls: true,
  lastUpdated: true,
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
          { text: 'Android', link: '/guide/android' },
          { text: 'Apple and SwiftPM', link: '/guide/apple' },
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
      { icon: 'github', link: 'https://github.com/yetanother.ai/takanawa' },
    ],
    footer: {
      message: 'Released under the MIT License.',
      copyright: 'Copyright (c) 2026 yetanother.ai',
    },
  },
})
