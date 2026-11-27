# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html) once it ships.

## [Unreleased]

### Added

- macOS spike shell (`spike-shell`) on NSH 0.4.0: PDF via pdfium-render, EPUB via WKWebView, Knuth–Plass justification with on-disk line-break cache, trim presets, window chrome prototypes.
- Rust workspace crates: `spike-report`, `spike-epub`, `spike-pdf`, `spike-measure`, `spike-shell`.
- `scripts/verify` UI verification harness and `docs/verify` records.
- Open-source docs: README, CONTRIBUTING, SECURITY, CI, Dependabot, issue forms (Phase 1; no edits under `spikes/` or `scripts/verify`).
- Dual license **MIT OR Apache-2.0** (`LICENSE-MIT`, `LICENSE-APACHE`; workspace `license` already on `spikes/Cargo.toml` on `main`).

### Changed

- Pin the macOS shell to NSH 0.5.0.
- EPUB direction under evaluation toward an owned pure-Rust typesetter (WKWebView remains default until then).
- `SPIKE_TYPESET=1` paints the owned typesetter's glyph runs. The default EPUB page stays WKWebView.
- `SPIKE_TYPESET=1` keeps the 160 ms page slide, the sentence, contents, in-book links, annotations, bookmarks, in-book find, live reflow, and Bold Word Starts. The stored sentence stays the WKWebView body offset, and the glyph-run page translates it. The default EPUB page stays WKWebView.
- `SPIKE_TYPESET=1` follows the finger and jumps to an in-book footnote named with `id` or `name`. A margin note's start and end stay body Unicode scalars, the numbers WKWebView already stores. The selection anchor stays a UTF-16 body offset. A 2026-11-27 Phase 1 crate run on EB Garamond and Noto Serif CJK SC did not include the WKWebView census, so the default EPUB page stays WKWebView.
- The Phase 1 Iowan Old Style and Songti SC census runs on the official `macos` CI job (`scripts/typeset-census.sh`, GitHub-hosted macOS). The record is `docs/design/typeset-gate/ci/census.json`. The 2026-11-10 baseline is not a pass for this tree. The default EPUB page stays WKWebView until that census records `phase1_pass` true.
