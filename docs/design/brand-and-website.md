# Brand visual system and official website

Recorded on 2026-11-24. This document defines Slowpage's brand visual system and official website architecture in `website/`, aligned with `docs/design/north-star.md`, `docs/design/shell/spec.md`, `docs/design/shell-tokens.md`, and `.cursor/skills/slowpage-design/SKILL.md`.

## Two-material contrast

Slowpage's visual identity is built on the contrast between **still, opaque reading paper** and **light, refractive floating chrome**:

1. **Content & Paper Layer (0% Glass)**:
   - Books-white `#FFFFFF` page and card surfaces (`tokens::CARD`, `reading::PAPER`).
   - Warm black reading ink `#1c1915`, `EB Garamond` / `Iowan Old Style` for Latin and `Noto Serif SC` / `Songti SC` for CJK, `tabular-nums` footer without a dividing rule.
   - Never apply backdrop blur, translucency, or glass refraction behind body prose.
2. **Floating Chrome & Interactive Controls (Liquid Glass)**:
   - macOS App (S5 / post-S5): `NSVisualEffectView` (`BehindWindow`, `NSAppearanceNameAqua`) for the light-gray window base, plus macOS 26 `NSGlassEffectView` / `NSGlassEffectContainerView` on floating reading HUD controls (`contents_button`, `hover_title`, `immersive_button`, `margin_toggle`), `source_menu`, and the `< 818pt` covering drawer backdrop (keeping inner `208pt` margin cards and composer opaque `#FFFFFF`, with pre-Tahoe `NSVisualEffectView` fallback and `Reduce Transparency` / `Reduce Motion` support).
   - Official Website (`website/`): Powered by [`solid-liquid-glass`](https://github.com/realmorrisliu/solid-liquid-glass) v2.0 (a standalone MIT-licensed Solid 2.0 + `@ui-sdk/solid` port of [`bilalgursen/liquid-glass`](https://github.com/bilalgursen/liquid-glass) with Cohesion, Pull, Press Light, `GlassMenu`, and `prefers-reduced-transparency` fallbacks), applied strictly to floating navigation pills, interactive trim/mode tabs, popovers, menus, and showcase HUDs.

## Tokens & assets

- **Brand mark**: Sculptural Liquid Glass "S"-Wave turning-page mark inside a cool alabaster macOS squircle (`assets/icon/Slowpage.png`, `1024×1024` transparent macOS HIG squircle from Concept C: matte white curled page paired with a frosted warm-amber glass ribbon `#FACE9A -> #EC7922 -> #B93E07`; tight web squircle cutout at `website/public/slowpage-icon.png` plus `website/public/favicon-32.png` and `website/public/apple-touch-icon.png`).
- **Light-only palette**:
  - Window field base: `#EEEEF0 -> #E9E9EC`
  - Paper & floating cards: `#FFFFFF` with crisp `0.5px` ring (`rgba(0,0,0,0.07)`) and subtle elevation (`0 6px 20px rgba(0,0,0,0.045)`)
  - UI neutral ink scale: `#1D1D1F` (primary), `#3A3A3F` (chip), `#55555B` (cite), `#6E6E73` (secondary/icons), `#8E8E93` (quiet), `#A1A1A6` (faint)
  - Marginalia semantic list hues: `#E0773A`, `#5B8DEF`, `#7D8C6A`
- **Icons**: IconMCP only (`assets/icons/`, outline `stroke-width="1.25"`, `radius 2`). No emoji icons.
- **Motion**: `160 ms` cosine curve `(1 - cos(π · t / 160)) / 2` for page turns and shell form transitions (`docs/design/shell/spec.md`, `spikes/shell/src/ease.rs`).
- **Guardrails**: No dark mode or theme switch, no purple/decorative gradients, no SaaS card soup, no all-caps eyebrows, and no hairlines separating shell regions or reading footers.

## Website stack (`website/`)

- **Framework**: Solid 2.0 (`solid-js@^2.0.0`, `@solidjs/web@^2.0.0`, `@solidjs/vite-plugin@^3.0.1`, Vite 8)
- **Behaviors**: Vercel Labs UI SDK (`@ui-sdk/core`, `@ui-sdk/solid`)
- **Liquid Glass**: `solid-liquid-glass` v2.0 (`github:realmorrisliu/solid-liquid-glass`, MIT, preserving `bilalgursen/liquid-glass` 2.0 WebGL2 refraction/cohesion engine + `prefers-reduced-transparency` fallback)
- **Styling & Deployment**: Tailwind CSS v4 (`@tailwindcss/vite`), Cloudflare Vite plugin (`@cloudflare/vite-plugin`), Worker entrypoint (`website/worker/index.js`), and `cf` CLI deployment (`website/cloudflare.config.ts`) serving `https://slowpage.app` and `https://www.slowpage.app`.

## Page structure (2026-11-26 refinement)

Approved by Morris on 2026-11-26. The page runs hero, typography, marginalia, library, closing, footer. Backgrounds alternate white and `#F5F5F7` so no two neighbors share a field. The centered template appears only in the hero and the closing; the middle sections are a full-width comparison, a left-aligned two-column header, and a split row.

- **Hero**: headline 「读得慢一点，懂得多一点。」 / "Read slower. Understand more." in one color. The first mention of @alan reads 「内置 AI 伴读 @alan」 with a hover and focus explanation. One primary button (download); the browser trial is a text link.
- **Typography**: the signature section. The same English or Chinese text is set twice side by side. The English Slowpage column runs the reader's own line breaker: `website/src/typeset/knuthPlass.ts` loads `spikes/justify/vendor/tex-linebreak/layout.js` and the `hyphenated-en-us` patterns with the `BREAKS`, hyphen cost and hyphenation limits of `spikes/justify/justify.js`. When no break set fits the cap, the paragraph falls back to the browser, as the reader does. The Chinese columns compare `line-break: anywhere` with the reader's kinsoku, punctuation trim and mixed-script spacing.
- **Marginalia**: the interactive reading window, with a "Powered by Alan" link to `https://alanworks.app`.
- **Library**: source card on the left, copy on the right. The Liquid Glass material card was removed; glass stays on controls only.
- The three-column highlights band and the bento were removed as SaaS card soup.

Accent text uses `#B4531F` (`--sp-accent-text`), which keeps 5:1 contrast on white and `#F5F5F7`. The brand gradient stays on the icon.

## Locales, analytics and release label

- `/` is Chinese and `/en/` is English. Both HTML entries mount the same app; `website/src/i18n.ts` keys the marketing copy off `<html lang>`. The product demos mirror the Chinese app UI on both.
- Analytics are anonymous PostHog events shared by name with alanworks.app and M3: `$pageview`, `download_click`, `github_click`, `scroll_75`. `/ph.js` loads after `load` when the main thread is idle, with session recording off and no person profiles. The footer says so.
- The release label comes from the GitHub latest release at runtime, with the build-time label as the fallback. Download links point at `releases/latest`.
- Social cards are `website/public/og.png` and `og-en.png`, 1200 × 630.
