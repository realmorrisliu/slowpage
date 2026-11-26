# Slowpage Website (`slowpage.app`)

Official landing page and interactive typesetting / marginalia preview for **Slowpage**, deployed to Cloudflare Workers at `https://slowpage.app`.

## Stack

- **UI Framework**: Solid 2.0 (`solid-js`, `@solidjs/web`, `@solidjs/vite-plugin`)
- **UI Primitives**: `@ui-sdk/core` + `@ui-sdk/solid`
- **Liquid Glass**: [`solid-liquid-glass`](https://github.com/realmorrisliu/solid-liquid-glass) v2.0 (WebGL2 refraction & cohesion with `prefers-reduced-transparency` fallback)
- **Styling & Build**: Tailwind CSS v4 (`@tailwindcss/vite`) + Vite 8
- **Static Prerender (SSG)**: `scripts/prerender.mjs` injects static Navbar + Hero HTML into `.cloudflare/output/v1/workers/default/assets/index.html` and `en/index.html` at build time
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

## Shared Conventions (Alan · M3 · Slowpage)

`slowpage.app` shares product-family conventions with `alanworks.app` (Alan) and `m3.thinksmart.systems` (M3):

- **Bilingual routes & CLI banner**: `/` serves `zh-CN` and `/en/` serves `en`, both mounting the same Solid app keyed off `<html lang>` with `hreflang` alternates (`zh-CN`, `en`, `x-default`). `worker/index.js` redirects `www.slowpage.app` -> `slowpage.app` (301) and `/en` -> `/en/` (301), and serves an ANSI terminal banner to `curl` / `wget` / `httpie` on `/` and `/en/` with `Vary: User-Agent`.
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
