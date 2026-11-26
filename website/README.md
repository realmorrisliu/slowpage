# Slowpage Website (`slowpage.app`)

Official landing page and interactive typesetting / marginalia preview for **Slowpage**, deployed to Cloudflare Workers at `https://slowpage.app`.

## Stack

- **UI Framework**: Solid 2.0 (`solid-js`, `@solidjs/web`, `@solidjs/vite-plugin`)
- **UI Primitives**: `@ui-sdk/core` + `@ui-sdk/solid`
- **Liquid Glass**: [`solid-liquid-glass`](https://github.com/realmorrisliu/solid-liquid-glass) v2.0 (WebGL2 refraction & cohesion with `prefers-reduced-transparency` fallback)
- **Styling & Build**: Tailwind CSS v4 (`@tailwindcss/vite`) + Vite 8 (single client entry `index.html`)
- **Static Prerender (SSG)**: `scripts/prerender.mjs` inlines compiled CSS and injects static Navbar + Hero HTML into `.cloudflare/output/v1/workers/default/assets/index.html` (`zh-CN`) and `.cloudflare/output/v1/workers/default/assets/_i18n/en.html` (`en`) at build time
- **Edge Hosting**: `@cloudflare/vite-plugin`, custom Worker (`worker/index.js`), and the Cloudflare `cf` CLI (`cloudflare.config.ts`)

## Local Development

Run commands from `website/`:

```bash
pnpm install
pnpm dev        # Start local Vite dev server
pnpm typecheck  # Run TypeScript check (tsc --noEmit)
pnpm build      # Typecheck + Vite build + build-time hero prerender
pnpm preview    # Preview the built Cloudflare Worker + assets locally
```

## Single-URL i18n & Shared Conventions (Alan · M3 · Slowpage)

`slowpage.app` shares product-family conventions with `alanworks.app` (Alan) and `m3.thinksmart.systems` (M3):

- **Single-URL edge language negotiation & in-place switching**: `https://slowpage.app/` is the single canonical URL for both Chinese (`zh-CN`) and English (`en`).
  - `scripts/prerender.mjs` generates `assets/index.html` (`zh-CN`) and `assets/_i18n/en.html` (`en`) from the same Vite build output.
  - `cloudflare.config.ts` sets `assets.runWorkerFirst: ['/', '/en', '/en/', '/_i18n/*']`.
  - `worker/index.js` redirects `www.slowpage.app` -> `https://slowpage.app` (301), returns `404` for direct `/_i18n/*` requests, redirects legacy `/en` and `/en/` to `/` (302) with `Set-Cookie: sp_lang=en; Path=/; Max-Age=31536000; SameSite=Lax`, serves an ANSI banner to `curl` / `wget` / `httpie` on `/`, and negotiates browser locale on `/` (`sp_lang` cookie > `Accept-Language` q-value comparison of `en` vs `zh` > default `zh`) with `Vary: Accept-Language, Cookie, User-Agent`, `Content-Language`, and `Cache-Control: private, no-cache`.
  - In the browser, `src/i18n.ts` exposes a reactive `locale` signal and `setLocale()` so the Navbar button switches between Chinese and English in place without reloading, updating the `sp_lang` cookie, `<html lang>`, `document.title`, and PostHog `locale`.
- **Anonymous PostHog analytics**: `public/ph.js` and `src/analytics.ts` load PostHog lazily after `load` via `requestIdleCallback` with `disable_session_recording: true` and `person_profiles: 'identified_only'`. Shared event names across the family: `$pageview`, `download_click`, `github_click`, `scroll_75`.
- **Social cards**: `1200×630` Open Graph and Twitter summary cards at `public/og.png` (`zh-CN`) and `public/og-en.png` (`en`).
- **Family footer**: `src/sections/Footer.tsx` cross-links Alan and M3 under "同一作者" / "Also by the author" alongside the privacy disclosure and MIT / Apache-2.0 credits.

## Cloudflare Deployment

### Automatic deploy on `main`

[`.github/workflows/website.yml`](../.github/workflows/website.yml) runs `build` on pull requests and pushes affecting `website/**`, `spikes/justify/vendor/**`, or the workflow file, and runs `deploy` after `build` on `main` (or manual `workflow_dispatch`):

1. Create a Cloudflare API token with **Workers Scripts: Edit** permission for the account in `cloudflare.config.ts`.
2. Add the token to GitHub repository secrets as `CLOUDFLARE_API_TOKEN` (Repository **Settings → Secrets and variables → Actions**, or `gh secret set CLOUDFLARE_API_TOKEN`).
3. Without `CLOUDFLARE_API_TOKEN` set, the `deploy` job logs `"CLOUDFLARE_API_TOKEN is not configured; skipping automatic Cloudflare deploy."` and exits `0` without failing CI.

### Manual deploy

```bash
cd website
pnpm install --frozen-lockfile
pnpm build
pnpm exec cf deploy
```
