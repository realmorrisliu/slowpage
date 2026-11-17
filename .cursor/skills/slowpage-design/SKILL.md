---
name: slowpage-design
description: Records the approved Slowpage interface direction and requires agents to follow it before any UI change. Use when building or reviewing the shell, library, reading view, or typesetter page, and when the user says slowpage-design.
---

# Slowpage design

Follow the design docs. This skill records the direction that is already approved. It does not invent a new look.

The interface authority is the [Apple Human Interface Guidelines](https://developer.apple.com/design/human-interface-guidelines/). The product look is Apple Books, flat and minimal. The window base stays the light gray macOS vibrancy field in `docs/design/shell/spec.md`. Sidebar group labels stay small and gray, as that spec says. The card stays opaque white.

## Read these first

Read these files before you change UI.

- `docs/design/north-star.md`
- `docs/design/shell/spec.md`
- `docs/design/shell/approved-full-form.png`
- `docs/design/shell-tokens.md`
- `docs/design/reading-interactions.md`
- `CONTRIBUTING.md` Icons section
- `.cursor/skills/apple-design/SKILL.md`

There is no `DESIGN.md` in this repo. Do not add one to satisfy this skill.

`docs/design/shell-tokens.md` is landing in [pull request 53](https://github.com/realmorrisliu/slowpage/pull/53). `docs/design/reading-interactions.md` is landing in [pull request 55](https://github.com/realmorrisliu/slowpage/pull/55). Neither file is on main yet. When a file is absent, use the docs that are on your branch. Do not invent the missing tokens or checklist items.

## Fonts

Chrome uses the system font. On macOS that stack is SF Pro and PingFang SC, as `docs/design/shell-tokens.md` records. Do not add a distinctive display face. The window title is `title_face` in `spikes/shell/src/chrome.rs`. A title with a CJK character uses Songti SC. Any other title uses Iowan Old Style.

Book text uses the publication's own face. The reader fallbacks already in `docs/design/north-star.md` are Iowan Old Style for Latin and Songti SC for Chinese and Japanese. The typesetter's existing faces are EB Garamond and Noto Serif CJK SC. Do not add another face.

## Do not add

Icons come only from IconMCP. The parameters are outline, stroke 1.25, and radius 2. `CONTRIBUTING.md` and `docs/design/north-star.md` state that rule. Do not use emoji as icons.

Do not add any of these.

- A purple gradient, or another decorative gradient, that the design docs and the approved picture do not show.
- SaaS card soup. That is a grid of identical rounded cards.
- Decorative shadows.
- All-caps eyebrows or all-caps labels.
- A theme switch, a dark mode, or a night mode.

`docs/design/north-star.md` already forbids theme switching and night mode.

Keep a shadow or a gradient that a design doc or `docs/design/shell/approved-full-form.png` already shows. Do not add one those sources do not show. The window field in that picture is lighter at the top than at the bottom.

## Adaptive layout

No control, label, or page may clip or overflow its container.

Do not size the window for one resolution only. Check the minimum that the design doc names for that form, a 1440 by 900 window, and full screen. `docs/design/shell-tokens.md` names the library minimum as 578 by 520. That number is not the reading window's minimum.

A measure that a design doc fixes stays fixed. The sidebar width, the chat panel width, and the card inset are examples. Content that the docs say adapts must adapt.

## Alignment

Toolbar items share the traffic lights' vertical center. Sidebar, shelf, and chat edges and paddings sit on one grid. The design tokens define that grid. Icons and labels in a row share a vertical center or a text baseline. Repeated spacings are equal.

The sidebar top-row inset is the traffic-light measurement in `docs/design/shell/spec.md`. Read it again when that spec says to. It is not a token. The other edges and paddings sit on the token grid.

The tolerance is 0.5 points. Automated bounding-box checks enforce it at the minimum window, at 1440 by 900, and at full screen. Visual review uses zoomed-in crops.

Fix a miss in the tokens or the layout. Do not nudge one element into place.

## Reading interactions

Follow `docs/design/reading-interactions.md`. That checklist is the rule. Do not replace it.

Each item needs both proofs in that file. Headless `nsh test` drives the input and checks the result. A Mac run records the screen, or it captures consecutive frames. One screenshot does not prove the item. The capture has no blank frame, no flicker, no position jump, and no clipping.

A trackpad swipe follows the finger, and the reader can cancel that swipe. Arrow keys, Space, Shift-Space, and an edge click play the 160 ms slide in `docs/design/north-star.md`. After a reopen, the book returns to the same sentence. The page stays Books-white.

## Process

1. Write the plan before you write UI code. Name each layout choice and each token. Name the design doc it comes from.
2. Implement that plan.
3. Compare real screenshots of the result with the approved mockup before you claim the change is done. The library mockup is `docs/design/shell/approved-full-form.png`. A reading interaction still needs both proofs above.

## Borrowed rules, and the override

This file keeps two ideas from Anthropic's frontend-design skill. That skill is Apache-2.0. The upstream file is https://github.com/anthropics/skills/blob/main/skills/frontend-design/SKILL.md. The anti-slop list in `Do not add` borrows four bans from that skill. The bans are identical rounded cards, decorative gradients, decorative shadows, and all-caps eyebrows. Process borrows the other idea. Plan first, then compare screenshots with the approved mockup. The IconMCP rule, the night-mode ban, and the system-font rule are Slowpage's.

That same skill tells you to be distinctive, to avoid system fonts, and to take a bold aesthetic risk. Do not follow those three instructions. Slowpage uses system fonts and a quiet Apple-native design. The design docs win.

`.cursor/skills/apple-design/SKILL.md` is vendored for its principles. Use the principles. Do not copy its web or CSS snippets. Slowpage is native AppKit through NSH, in Rust. When that skill and this file disagree, this file and the design docs win. A shadow, a gradient, or a night mode that this file, the design docs, and the approved picture do not show stays out.
