import { presetWind4, transformerDirectives, transformerVariantGroup } from "unocss";
// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  compatibilityDate: '2025-07-15',
  devtools: { enabled: false },
  ssr: false,
  modules: [
    '@nuxt/eslint',
    '@nuxt/fonts',
    '@nuxt/icon',
    '@vueuse/nuxt',
    '@unocss/nuxt'
  ],
  devServer: {
    port: 3461
  },
  unocss: {
    nuxtLayers: true,
    presets: [presetWind4({
      preflights: {
        reset: true
      }
    })],
    shortcuts: {
      'flex-center': 'flex items-center justify-center',
    },
    transformers: [transformerDirectives(), transformerVariantGroup()],
  },
})