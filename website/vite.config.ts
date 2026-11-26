import { cloudflare } from '@cloudflare/vite-plugin'
import solid from '@solidjs/vite-plugin'
import tailwindcss from '@tailwindcss/vite'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { defineConfig } from 'vite'

const __dirname = path.dirname(fileURLToPath(import.meta.url))

export default defineConfig({
  plugins: [solid(), tailwindcss(), cloudflare()],
  resolve: {
    dedupe: ['solid-js', '@solidjs/web', '@ui-sdk/core', '@ui-sdk/solid'],
  },
  environments: {
    client: {
      build: {
        rollupOptions: {
          // `/` is Chinese, `/en/` is English. Both mount the same app; copy keys off <html lang>.
          input: {
            main: path.resolve(__dirname, 'index.html'),
            en: path.resolve(__dirname, 'en/index.html'),
          },
        },
      },
    },
  },
  server: {
    fs: {
      allow: [__dirname, path.resolve(__dirname, '..')],
    },
  },
})
