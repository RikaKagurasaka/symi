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
    preset: 'cloudflare-module',
    // 2. 禁用 SourceMap
    // Cloudflare 部署通常不需要 SourceMap，这能节省大量内存
    sourceMap: false,
    // 3. 开启 WASM 支持
    // 这能解决 shiki/onig.wasm 的加载警告，防止它回退到低效模式
    experimental: {
      wasm: true
    },
    // 4. 针对 Shiki 的特定 Rollup 优化 (可选，如果上面不行再加这个)
    rollupConfig: {
      output: {
        // 避免过度拆分 chunk
        manualChunks: undefined,
      }
    },
    prerender: {
      routes: [
        '/'
      ],
      ignore: [
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
        title: 'LLM',
        description: 'Machine-friendly endpoints for AI tools.',
        links: [
          {
            title: 'LLM Guide',
            description: 'How to use llms.txt, llms-full.txt.',
            href: '/llm'
          }
        ]
      }
    ],
    notes: [
      'For machine consumption, prefer /llms.txt for index-level context and /llms-full.txt for detailed content.',
    ]
  },


})
