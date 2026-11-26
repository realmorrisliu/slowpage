import { bindings, defineConfig } from 'cf/config'

export default defineConfig({
  accountId: 'b96556986b5109d88eb943aef8a7fa6d',
  worker: {
    name: 'slowpage-website',
    entrypoint: './worker/index.js',
    compatibilityDate: '2026-11-24',
    env: {
      ASSETS: bindings.assets(),
    },
    assets: {
      runWorkerFirst: ['/'],
    },
    domains: ['slowpage.app', 'www.slowpage.app'],
  },
})
