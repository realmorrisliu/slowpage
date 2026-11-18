# What 60d0dcf actually showed

Head under test: `60d0dcf`. The summary file is `docs/verify/runs/shell-s2-1-60d0dcf/summary.json` (85 pass, 12 fail, 27 pending). That count is not acceptance. `f3c34b3` only records the summary and this note's predecessor. It does not make the count true.

## Premise

Every failed check in that run shared one premise: asking for a state, or writing a file whose name is that state, means the state was entered.

## Counterevidence

- `b-pride-fullscreen.png` is the same bytes as `b-pride.png`, 1164×1562, the 582×781 window. `SPIKE_FULLSCREEN` was set. Chrome for that capture never had `fullscreen: true`. The footer, page, and paper rows still passed.
- `aqua-dark.png` is the same bytes as `b-pride-hover.png`. The detail says `NSAppearanceNameAqua`. The dark row passed.
- `docs/verify/alpha/epub.png` is the Open panel on `pride-se.epub`. `docs/verify/alpha/pdf.png` is the empty shelf. Open, turn, reopen, and the PDF page rows were scored as product failures.
- The suite started at 1-minute load 51.23 because `SR_VERIFY_MAX_LOAD=999` replaced the prescribed ceiling of 8. Cold p95, warm p95, and the slide frames were then treated as comparable. They are not.

## Control

These observations did enter the state they name.

- `frames/fullscreen-settled-chrome.json`: `fullscreen` true, window 1512×949, `content_top` 0. `form-fullscreen.png` is 3024×1898. Exit returns to 1440×855. That window is the library, not the reader.
- `b-pride-hover.png` on disk (188158 bytes, 17:00) shows the list glyph, the title Pride and Prejudice, and white paper. `window-aqua` reported gap 26.0 and dy 0.0 from the chrome frames. The file was not in `f3c34b3`, so a GitHub raw URL for it 404s. `f3c34b3` itself is on `origin/cursor/shell-form-tokens-d356`.
- `epub-pride.png` has ink. `epub-turned.png` has none. They are different processes. The turn capture never showed a loaded page.
- Bold on-probe returned the 15 prefixes, so that Pride process had loaded the book. The later cache sample was `source` null and `live` 0.

## Decision

A pass has to name the window, the flag, the native size, and the loaded book. A filename does not. A missing book does not fail Pride. Load at or above 8 does not start a run, and `SR_VERIFY_MAX_LOAD` cannot raise 8. Performance numbers taken above that ceiling are not executed.

## Corrected table

Valid observations, not a suite acceptance:

- Contents frame: gap 26.0, dy 0.0, lights shown, on the reader that had Pride loaded. Hover file archived with this change. Dark was not a second state.
- No gray column on `bar-window.png` (2200×1660, column width 0). `sidebar-fullscreen` reports no painted column. That is the bar measurement, not the false reader-fullscreen file.
- Shelf form entered and left fullscreen at the sizes above. `content_top` on those three frames is 0.
- Empty shelf has no Open button.

Void passes, state not entered:

- `chrome-b-fullscreen-footer`, `chrome-b-fullscreen-page`, `chrome-b-fullscreen-paper`.
- `window-aqua` (the dark row).
- `chrome-b-zhaohua-footer` and `flash-zhaohua` without an admitted Chinese book.

Not executed:

- 朝花 and plain-zh. Gutenberg 25271 returned 503. No sha is recorded. Nothing was fabricated.
- Bold off. `/tmp/spike-shell-main-fdd8183` is absent.
- Reader fullscreen title and paper.
- This run's alpha open, turn, reopen, and PDF pages. The panel or the empty shelf was still up.
- This run's `epub-turn`. The capture has no loaded page. `35995b2` read `2/10页` on a different head. This run does not prove the page is blank after a load.
- Cold open, warm open, and slide frames. Load was 51 to 95.

Real observations that stay open, and are not script artifacts:

- Packaged relaunch on `1b33c34` opened Pride at page 1, turned to 2, and the next launch stayed on the shelf (`page` None). `launches_full()` takes the shelf before `load_place`. `60d0dcf` did not repeat that, because the Open panel never closed. Landing on the shelf is the library form. 继续阅读 is not in this slice. It is not a false score, and it is not closed.
- Bold cache, after a loaded Pride: on-live `live` 32, then on-cached `source` null and `live` 0. The probe can report. This sample did not. Roadmap places the rest of Bold Word Starts after the shell. The file not changing does not erase the sample.
- The AppKit frames between fullscreen enter and settle were not snapshotted.

Future, not a #53 block:

Finder double-click, 继续阅读, library PDF open, ask, scroll column, comment note and UI, Alan, `superreader` JSON, SQLite annotations, tantivy. S3 is the reading form. S4 is immersive.

## Next run, when load is under 8

One `just verify`. No `SR_VERIFY_MAX_LOAD` above 8. A Chinese book is in the run only after `spikes/epub/fixtures/chinese-admission.json` has the file's sha256 and the chapter text meets 1500 characters and 范愛農. Do not start it while another project's process would have to be killed. Do not install to `/Applications`. Do not write the real profile.
