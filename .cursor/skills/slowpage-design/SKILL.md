---
name: slowpage-design
description: Records the approved Slowpage interface direction and requires agents to follow it before any UI change. Use when building or reviewing the shell, library, reading view, or typesetter page, and when the user says slowpage-design.
---

# Slowpage design

Follow the design docs. This skill records the direction that is already approved. It does not invent a new look.

The interface authority is the [Apple Human Interface Guidelines](https://developer.apple.com/design/human-interface-guidelines/). The product look is Apple Books, flat and minimal.

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

Chrome uses the system font. On macOS that font is the SF family the system provides. Do not add a distinctive display face.

Book text uses the publication's own face. The reader fallbacks already in `docs/design/north-star.md` are Iowan Old Style for Latin and Songti SC for Chinese and Japanese. The typesetter's existing faces are EB Garamond and Noto Serif CJK SC. Do not add another face.

## Do not add

Icons come only from IconMCP. The parameters are outline, stroke 1.25, and radius 2. `CONTRIBUTING.md` and `docs/design/north-star.md` state that rule. Do not use emoji as icons.

Do not add any of these.

- Purple gradients or other decorative gradients.
- SaaS card soup. That is a grid of identical rounded cards.
- Decorative shadows.
- All-caps eyebrows or all-caps labels.
- A theme switch, a dark mode, or a night mode.

`docs/design/north-star.md` already forbids theme switching and night mode.

The library card's one soft shadow is specified in `docs/design/shell/spec.md`. Match the approved picture. Do not add a second shadow.

## Adaptive layout

No control, label, or page may clip or overflow its container.

Do not size the window for one resolution only. Check the minimum window, a 1440 by 900 window, and full screen.

A measure that a design doc fixes stays fixed. The sidebar width, the chat panel width, and the card inset are examples. Content that the docs say adapts must adapt.

## Reading interactions

Follow `docs/design/reading-interactions.md`.

A gesture animation follows the finger, and the reader can cancel it. The frame does not jump, flicker, or show a blank or white frame. Reading progress stays on the same sentence.

Prove an interaction with a screen recording or with consecutive frames. One screenshot does not prove it.

## Process

1. Write the plan before you write UI code. Name each layout choice and each token. Name the design doc it comes from.
2. Implement that plan.
3. Compare real screenshots of the result with the approved mockup before you claim the change is done. The library mockup is `docs/design/shell/approved-full-form.png`. A reading interaction still needs the recording above.

## Borrowed rules, and the override

This file keeps two ideas from Anthropic's frontend-design skill. That skill is Apache-2.0. The upstream file is https://github.com/anthropics/skills/blob/main/skills/frontend-design/SKILL.md. The anti-slop list in `Do not add` borrows four bans from that skill. The bans are identical rounded cards, decorative gradients, decorative shadows, and all-caps eyebrows. Process borrows the other idea. Plan first, then compare screenshots with the approved mockup. The IconMCP rule, the night-mode ban, and the system-font rule are Slowpage's.

That same skill tells you to be distinctive, to avoid system fonts, and to take a bold aesthetic risk. Do not follow those three instructions. Slowpage uses system fonts and a quiet Apple-native design. The design docs win.

`.cursor/skills/apple-design/SKILL.md` is vendored for its principles. Use the principles. Do not copy its web or CSS snippets. Slowpage is native AppKit through NSH, in Rust. Where that skill mentions colors that adapt to light and dark, night mode stays out.
