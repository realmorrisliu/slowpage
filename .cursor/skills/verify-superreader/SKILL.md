---
name: verify-superreader
description: Drive the SuperReader spike on macOS and score the walkthroughs plus the bars in docs/design/north-star.md. Use when a reader change needs a pass or fail, when the user says verify-superreader, or before calling a SuperReader PR done.
---

# Verify SuperReader

The product Morris reads is still the spike shell. `scripts/verify` is the pass or fail. A check that the spike does not implement is `pending`. Pending does not pass, and it does not fail the exit code. A failing check exits nonzero.

Read [features/README.md](features/README.md) before driving one feature by hand. The entry command already runs every check in that map.

## Launch

From the repo root:

```sh
just verify --out docs/verify/runs/$RUN_ID
```

`just verify` takes `/tmp/superreader-verify.lock` with `mkdir`, waiting 30 seconds between tries, and releases it with `rmdir` when the run exits. It then reads `sysctl -n vm.loadavg`. The prescribed ceiling is 8. `SR_VERIFY_MAX_LOAD` cannot raise it. If the 1-minute load is not under the ceiling, the run does not start. Do not start a second window run while the lock is held.

The script builds `spike-shell` and `spike-measure` in release, then starts each drive as its own process. A PDF drive is ready when Accessibility shows the paper title and a `N pages left` footer. An EPUB drive is ready when the footer shows `N/M` with more than one page and `page left in this chapter` or `pages left in this chapter`. There is no shared server and no seeded database.

Build the two binaries in separate `cargo` invocations. `nsh-core` 0.5.0 compiles when taffy's `calc` feature is also on. The script still builds them separately.

Fixtures live outside the repo, where the spike already looks:

- `$SPIKE_GPT3` or `/tmp/superreader-fixtures/gpt3.pdf`
- `$SPIKE_PRIDE` or `/tmp/superreader-fixtures/pride-se.epub`
- `$SPIKE_PDFIUM` or `/tmp/pdfium/lib`, the directory that holds `libpdfium.dylib`. A path that already ends at the dylib file is accepted too.

If doctor says one is missing, run `spikes/fetch-fixtures.sh`. That script downloads into `/tmp`. Do not invent a second fixture path.

## Doctor

```sh
scripts/verify --doctor
```

This is read-only. It checks `nsh` is 0.6.1, the shell crate pins `nsh-core`, `nsh-base`, and `nsh-platform-macos` at `=0.6.1`, the fixtures exist, Cargo.lock has neither the `epub` crate nor `mupdf`, the machine is macOS, and no `spike-shell` process is already running. A running shell belongs to someone else. Do not kill it.

## Drive

Two drivers. Do not swap them.

Headless `nsh test` reads the NSH tree. The test file is `spikes/shell/shell.test.tsx`. It opens the Rust root `SpikeApp` from `spikes/shell/nsh.json`. The texts it can see are the toolbar and the footer slots. It asserts the tree contains no `SuperReader spike`, no `night`, no `theme`, and no `spread`.

`nsh test` does not see pixels inside a native `NSView`. pdfium bitmaps and WKWebView are invisible to it. The headless runner also does not construct `reader.stage`, so it never receives the footer strings the stage publishes.

The window driver launches the release `spike-shell` from the target directory `cargo metadata` reports. It does not assume `spikes/target/release`, and it does not set `CARGO_TARGET_DIR`. It captures that process's window with `screencapture -l` after `CGWindowListCopyWindowInfo` finds the window id. It does not click the user's other apps and it does not call `screencapture` on a rectangle of the desktop. PDF footer strings come from Accessibility. WKWebView hides those NSH texts from Accessibility, so the EPUB footer is read with Vision from the captured PNG.

The env vars are the same commands the keys send. The reading window hides the traffic lights and the book title until the pointer is at the top. `just verify --subset reading-chrome` scores that fade from one library session. The blur file and the status log exist only while `SUPERREADER_VERIFY_SESSION` is set on that process. The structure rows read status and AppKit values. The shot rows pass only for a PNG, and that pass does not score pixels. The GPUI title row and the visual row stay `blocked`. `nsh test` does not see either. `--self-test` scores the sample log and does not launch the app. A `blocked` row is not a pass. `SPIKE_DRAG=1` is the two-line drag. `SPIKE_SCALE=zoom-2x` is Command plus. `SPIKE_MODE=webkit` is the EPUB page. `SPIKE_SELECT=1` with `SPIKE_RELOAD=1` marks `truth universally acknowledged` and reloads. `SPIKE_TURN=1` turns one page. `SPIKE_PLAY=1` turns three times so a recording can see the slide. `SPIKE_BOOK` selects `pride`, `zhaohua`, `styled`, `plain-en`, `plain-zh`, or `fixed`. The trim is the View menu. A real launch saves it in `~/Library/Application Support/app.slowpage.reader/settings.json`. `scripts/verify` sets `SUPERREADER_SUPPORT_DIR` to a fresh temp directory for that run, and the shell reads settings and the line-break cache from there. An empty override is the real directory. The footer is `N/M页` and `本章还剩X页`.

Perf numbers come from `spike-measure pdf pdfium retina-1x`, five release runs. The open-to-first floor is under 100 ms. The later-page median floor is at or under 16.7 ms. The median is the lower middle sample, the same rule as `spike-report`. The timed work is `Document::raster`. Each sample checks the bitmap length, the error count, and the two title phrases.

## Evidence

The run directory holds `summary.json`, the screenshots, the Accessibility dumps, `nsh-junit.xml`, and `measure.json`. `summary.json` is the result. The run replaces any previous summary with a `running` file before the checks, and that file is not reusable. `ok` is true only when no check has status `fail` or `blocked`. A finished file records `run_id`, `head`, and `session`. `status` `failed` is not reusable. Its `head` field is `git rev-parse HEAD`. Each check has `id`, `feature`, `status` of `pass`, `fail`, `pending`, or `blocked`, `driver`, and `detail`.

A proof shows the action and the resulting state. The PDF shots are the highlight at retina 1x and at zoom 2x. The EPUB shot is Pride with the footer and the mark. The highlight check counts yellow paper pixels and dark ink pixels inside that band. A yellow band with no ink fails.

Copy a sample only when someone asks to keep one. The committed sample is `docs/verify/summary.json` plus the screenshots next to it. Later runs go under `docs/verify/runs/`, which is gitignored.

## Cleanup

The script terminates the `spike-shell` pid it started. It does not signal a process by name. Screenshots and `summary.json` stay in the run directory. The spike itself still writes `/tmp/superreader-spike/webkit.json` and `drag.txt`. The script copies those into the run directory and does not delete the `/tmp` originals.

One verify at a time. A second run is refused while a shell is up, because both would write the same `/tmp` anchor file and both would own a window.

## Helpers

```sh
scripts/verify --self-test
scripts/verify --doctor
scripts/verify --out docs/verify/runs/manual
```

## Preflight

Before the full suite, prove the shell form on its own. This builds the release shell, launches the fixture library in an isolated temp support directory, and scores only that form. It walks 1440, the 578 by 528 minimum, and fullscreen enter, settled, and exit, and it requires the accessibility titles and a shelf wheel that moves pixels. It does not install to `/Applications` and it does not set `CARGO_TARGET_DIR`.

```sh
just verify --subset preflight --out /tmp/superreader-preflight-s2-1-9
```

Do not set `SR_VERIFY_MAX_LOAD` above 8. A load at or above 8 means this command does not start.

`shell-form-fixture`, `shell-shelf-scroll`, and `shell-clip` have to pass. `frames/` has to contain `1440`, `minimum`, `fullscreen-enter`, `fullscreen-settled`, and `fullscreen-exit`. `form-minimum.png`, `form-fullscreen.png`, and `form-restored.png` have to exist, and the fullscreen capture has to differ from the windowed one. The wheel before and after captures have to differ. A pass here is the gate for the full suite. The full suite uses a new run directory and still scores every other row.

`--self-test` checks the median, the footer patterns, and the highlight pixel rule. It does not launch the app.

Keep the map honest with `/maintain-verification-skill` when the reader gains a real feature or a check starts lying.
