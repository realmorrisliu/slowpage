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
  server: {
    fs: {
      allow: [__dirname, path.resolve(__dirname, '..')],
    },
  },
})
