// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  runtimeConfig: {
    githubToken: process.env.GITHUB_TOKEN,
    public: {
      githubRepo: process.env.NUXT_PUBLIC_GITHUB_REPO || 'RikaKagurasaka/symi'
    }
  },


  modules: [
    '@nuxt/eslint',
    '@nuxt/image',
    '@nuxt/ui',
    '@nuxt/content',
    'nuxt-og-image',
    'nuxt-llms',
    '@nuxtjs/mcp-toolkit'
  ],

  devtools: {
    enabled: true
  },

  css: ['~/assets/css/main.css'],

  content: {
    build: {
      markdown: {
        toc: {
          searchDepth: 1
        }
      },
    },
  },

  experimental: {
    asyncContext: true
  },

  compatibilityDate: '2024-07-11',

  devServer: {
    port: 3462
  },

  nitro: {
    prerender: {
      routes: [
        '/'
      ],
      crawlLinks: true,
      autoSubfolderIndex: false
    }
  },

  eslint: {
    config: {
      stylistic: {
        commaDangle: 'never',
        braceStyle: '1tbs'
      }
    }
  },

  icon: {
    provider: 'iconify'
  },

  llms: {
    domain: 'https://symi.rika.link',
    title: 'Symi Documentation',
    description: 'Documentation for the Symi, a music notation language for microtonal music.',
    full: {
      title: 'Symi Documentation - Full Version',
      description: 'The full version of the Symi documentation, including all details and examples.'
    },
    sections: [
      {
        title: 'Getting Started',
        description: 'Install Symi Editor and learn the core syntax basics.',
        links: [
          {
            title: 'Introduction',
            description: 'What Symi is and why it exists.',
            href: '/getting-started'
          },
          {
            title: 'Installation',
            description: 'Download and install Symi Editor.',
            href: '/getting-started/installation'
          },
          {
            title: 'Usage',
            description: 'Write, preview and export your music.',
            href: '/getting-started/usage'
          }
        ]
      },
      {
        title: 'Grammar Reference',
        description: 'Control statements, pitch syntax, timing and macros.',
        links: [
          {
            title: 'Overview',
            description: 'Core grammar concepts and structure.',
            href: '/grammars'
          },
          {
            title: 'Control',
            description: 'BPM, time signature and other global controls.',
            href: '/grammars/control'
          },
          {
            title: 'Pitch',
            description: 'Pitch notation and microtonal expressions.',
            href: '/grammars/pitch'
          },
          {
            title: 'Time',
            description: 'Duration and rhythm notation.',
            href: '/grammars/time'
          },
          {
            title: 'Macro',
            description: 'Reusable syntax blocks and expansion.',
            href: '/grammars/macro'
          }
        ]
      },
      {
        title: 'LLM & MCP',
        description: 'Machine-friendly endpoints and MCP integration for AI tools.',
        links: [
          {
            title: 'LLM & MCP Guide',
            description: 'How to use llms.txt, llms-full.txt and MCP endpoints.',
            href: '/llm'
          }
        ]
      }
    ],
    notes: [
      'For machine consumption, prefer /llms.txt for index-level context and /llms-full.txt for detailed content.',
      'MCP tools are available at /mcp and include list-pages and get-page for documentation retrieval.'
    ]
  },

  mcp: {
    name: 'Symi Documentation MCP',
    version: '1.0.0',
    route: '/mcp',
    browserRedirect: '/llm',
    dir: 'mcp'
  }
})
