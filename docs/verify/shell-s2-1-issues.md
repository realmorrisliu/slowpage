# Shell S2.1 issue list

The 60d0dcf count is not acceptance. The corrected table is `docs/verify/shell-s2-1-evidence.md`.

Head this list was written against: `54d3529`, after the `1b33c34` run (82 pass, 31 fail, 11 pending). The Nov 11 summary (`ebad852`) is the last committed result on `885718f`. It predates the toolbar move and the alpha checks, so it is not a tip measurement.

`#53` is the library full form. The reading page stays as it was. S3 (reading form, annotation margin), S4 (immersive), and Alan stay out.

## Contents control

- Scope: the existing list glyph, 26pt past zoom, centered on that button (`docs/design/north-star.md`). Not a new icon set. Not S3.
- Group: `window-aqua`, `window-aqua-light`, `sidebar-icon`.
- Fact: `1b33c34` `aqua-dark.png` shows lights and the page, no contents icon. `icon_frame` is null because the hidden title bar does not draw the toolbar item. `54d3529` restored the hover title only.
- Unknown: whether `885718f` tip already hid it. The insertion function was not edited by this branch. That does not close the row.
- Change: place the same list glyph on the content view and report that frame.
- Evidence: `window-aqua` / `sidebar-icon` in the next run. The control has to be enabled while the lights are shown.
- Gate: still required. The title fix does not close it.

## Fullscreen paper, 24 samples

- Scope: the top of the fullscreen capture is `#ffffff`. The toolbar hides with the menu bar.
- Fact: the 24 samples are black `(0,0,0)` at the top-right rounded corner of `b-pride-fullscreen.png` (1164×1562), after a 156px skip of `bar_w` 78. `bar_w` is the traffic-light span, not a gray column. Same count on `35995b2` and `1b33c34`.
- Change: stop skipping that span; ignore near-black corner pixels only.
- Evidence: `chrome-b-fullscreen-paper`.
- Gate: script fix, not a baseline edit.

## Chinese rows (16)

- Scope: existing reader. Not a new typesetter.
- Fact: `zhaohua.epub` was absent (`UnreadableArchive`). `plain-zh.epub` was under 1500 characters, so the reader reported `no chapter text`. Pride trim, white, and justify passed. The 16 empty rows are that miss, not a measured 齐行 defect. 范愛農 and 客店 were not on a page.
- Unknown: the typesetting itself, until a legal book is opened.
- Change: use Gutenberg 25271 (the repo's existing public-domain source). Do not read a private library and do not write a fake 朝花 chapter. `plain-zh` is extracted from that file and must be at least 1500 characters. If the source is missing, these rows are not executed.
- Evidence: the trim, justify, and text rows after a book that contains 范愛農.
- Gate: not a product pass while the rows are not executed.

## Bold Word Starts, off

- Scope: leftover after the shell (`docs/roadmap.md`). The on-probe already passed (`bold-word-starts-on`).
- Fact: `/tmp/spike-shell-main-fdd8183` is absent, so the pixel compare did not run. Prefixes were `[]`.
- Change: the row is not executed. It is not a typesetting pass or fail.
- Gate: stays out of the product result until that binary exists.

## epub-turn blank

- Fact: `35995b2` read `2/10页`. `1b33c34` `epub-turned.png` has no ink in the middle band, and the OCR label is empty. The turn flag is unchanged by `1b33c34`.
- Hypothesis: webkit json can report page 2 before the view has ink, and the capture scored that frame.
- Change: wait for ink before accepting the shot. If it stays blank, the row stays failed.
- Gate: still open until the next run shows a painted page.

## Alpha

- `alpha-empty`: S1 removed the Open button and said not to put it back. The shelf window is the empty state. The assertion is updated. Open and turn of Pride already passed (`page 1`, then `2`).
- Reopen `page None`: a bare relaunch opens Full, which `docs/design/shell/spec.md` states as "Launch opens Full." The stored page is not discarded. An explicit file resumes it, and so does the 继续阅读 row. That row is not the S3 margin. `alpha-epub-reopen` still describes the pre-shell "launch again on that page" check; S1 did not name it in the removal list.
- PDF `page None`: the footer is painted. The check asked the accessibility tree for `N/M`. `reader.json` already records the pdf page. The check now reads that when the tree has no number. If the pdf never opens, the row still fails.

## Bold cache, slide frames

- Cache: the on-live sample had `break_source` null and `live` 0; a later off-live sample reported `live` 32. The probe path can report. This sample did not. No product edit without a tighter cause. The row stays failed.
- Slide: `reading-slide.mp4` exists and `frame_ms` is empty. Frames are posted by the page-turn animation (`settle(frames)`). The ready check returns on a settled layout, then the video runs. Empty `frame_ms` means that animation did not post during the capture. Not changed without a reproduced cause. The row stays failed.

## Already failed before this branch

- `reading-cold-open` and `reading-warm-open` failed on the Nov 11 summary. The `1b33c34` cold p95 was 3108.1 ms at load 25, median 604.9. Not a new defect. They stay failed.

## Pending, not this PR

Finder double-click, 继续阅读, library PDF open, ask, scroll column, comment note and UI, Alan, `superreader` JSON, SQLite annotations, tantivy. S3, S4, and the post-shell list.

## Not closed by a title string

The zoom click that missed fullscreen was not executed. Fullscreen in the suite uses `SPIKE_FULLSCREEN`, the existing `toggleFullScreen:` entry. A windowed frame is not a fullscreen result. The 32pt first-frame cause stands; the AppKit snapshot during the animation is still open.
