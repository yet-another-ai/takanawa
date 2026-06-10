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
      { text: 'Format', link: '/part-format' },
      { text: 'API Docs', link: '/api/' },
    ],
    sidebar: [
      {
        text: 'Guide',
        items: [
          { text: 'Overview', link: '/' },
          { text: 'Getting Started', link: '/guide/getting-started' },
          { text: 'Platforms', link: '/guide/platforms' },
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
