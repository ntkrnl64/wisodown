import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'Windows ISO Downloader',
  description: 'Download Windows 10 and Windows 11 ISOs directly from Microsoft — no browser required.',

  head: [
    ['link', { rel: 'icon', type: 'image/svg+xml', href: '/ic_fluent_arrow_square_32_color.svg' }],
    ['meta', { name: 'theme-color', content: '#1a1a1a' }],
    ['meta', { property: 'og:type', content: 'website' }],
    ['meta', { property: 'og:title', content: 'wisodown — Windows ISO Downloader' }],
    ['meta', { property: 'og:description', content: 'Download Windows 10 and Windows 11 ISOs directly from Microsoft\'s servers.' }],
    ['meta', { property: 'og:image', content: '/ic_fluent_arrow_square_32_color.svg' }],
  ],

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
      { icon: 'github', link: 'https://github.com/ntkrnl64/wisodown' },
    ],

    footer: {
      message: 'Released under the GNU General Public License v3.0.',
    },
  },
})
