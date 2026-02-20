export default defineAppConfig({
  ui: {
    colors: {
      primary: 'purple',
      neutral: 'slate'
    },
    footer: {
      slots: {
        root: 'border-t border-default',
        left: 'text-sm text-muted'
      }
    }
  },
  seo: {
    siteName: 'Symi 编辑器文档',
  },
  header: {
    title: 'Symi 编辑器',
    to: '/',
    logo: {
      alt: '',
      light: '/logo-light.svg',
      dark: '/logo-dark.svg'
    },
    search: true,
    colorMode: true,
    links: [{
      'icon': 'i-simple-icons-discord',
      'to': 'https://discord.gg/pyZYtqXjeB',
      'target': '_blank',
      'aria-label': 'Discord'
    }, {
      'icon': 'i-simple-icons-github',
      'to': 'https://github.com/RikaKagurasaka/symi',
      'target': '_blank',
      'aria-label': 'GitHub'
    }]
  },
  footer: {
    credits: `Built with Nuxt UI • © ${new Date().getFullYear()}`,
    colorMode: false,
    // links: [{
    //   'icon': 'i-simple-icons-discord',
    //   'to': 'https://discord.gg/pyZYtqXjeB',
    //   'target': '_blank',
    //   'aria-label': 'Symi on Discord'
    // }, {
    //   'icon': 'i-simple-icons-github',
    //   'to': 'https://github.com/RikaKagurasaka/symi',
    //   'target': '_blank',
    //   'aria-label': 'Symi on GitHub'
    // }]
  },
  toc: {
    title: '目录',
    // bottom: {
    //   title: 'Community',
    //   edit: 'https://github.com/nuxt-ui-templates/docs/edit/main/content',
    //   links: [{
    //     icon: 'i-lucide-star',
    //     label: 'Star on GitHub',
    //     to: 'https://github.com/RikaKagurasaka/symi',
    //     target: '_blank'
    //   }, {
    //     icon: 'i-lucide-book-open',
    //     label: 'Symi docs',
    //     to: 'https://symi-docs.vercel.app/',
    //     target: '_blank'
    //   }]
    // }
  }
})
