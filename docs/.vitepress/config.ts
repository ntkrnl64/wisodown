import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'Windows ISO Downloader',
  description: 'Download Windows 10 and Windows 11 ISOs directly from Microsoft — no browser required.',

  themeConfig: {
    nav: [
      { text: 'Home', link: '/' },
      { text: 'Guide', link: '/guide/quick-start' },
    ],

    sidebar: [
      {
        text: 'Guide',
        items: [
          { text: 'Quick Start', link: '/guide/quick-start' },
          { text: 'Usage', link: '/guide/usage' },
          { text: 'How It Works', link: '/guide/how-it-works' },
        ],
      },
    ],

    socialLinks: [
      { icon: 'github', link: 'https://github.com/ntkrnl64/win11_iso' },
    ],

    footer: {
      message: 'Released under the MIT License.',
    },
  },
})
