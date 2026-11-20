# Window chrome

The reading window is the page. The traffic lights and the book title fade in together when the pointer is near the top. There is no `SPIKE_CHROME` switch.

## Sub-features

- `chrome-b-aspect` opens Pride at paperback. The window matches the page plus the side mats and the top mat, within 2 percent and 8 points.
- `chrome-b-clear` keeps the page box below the traffic lights.
- `chrome-b-footer` shows `N/M页` and `本章还剩X页`.
- `footer-no-divider` counts pixels in the band from the last body line to the top of that footer text. Paper passes. A glyph edge within two pixels of ink is not a divider. Any other pixel fails.
- `chrome-b-fill` keeps the gap under the last line of a full page smaller than one line.
- `chrome-b-measure` keeps the footer row on the same left and right edges as the text measure.
- `chrome-b-trim` switches the saved trim to US Trade and the window follows.
- `chrome-b-zhaohua-footer` shows the same footer on 朝花夕拾.
- `chrome-b-fullscreen-footer` keeps that footer in full screen.
- `chrome-b-fullscreen-page` scales the page to the screen height, centers it in the area to the right of the sidebar, keeps the footer on the measure, and leaves the CSS page at the windowed design width.
- `chrome-b-fullscreen-paper` keeps the top 60 px of the full-screen capture Books-white, `#ffffff`, to the right of the sidebar. The toolbar hides with the menu bar.
- `chrome-b-hover` shows the traffic lights when the pointer is at the top of the window.
- `reading-chrome-entry-status` reads the first session `chrome-status.jsonl` line with `reading` true. `title_opacity` is 0 and `chrome_shared` is true. That line is the status cell. It is not the rendered GPUI title.
- `reading-chrome-entry-lights` reads `button_alpha` from the close button's `alphaValue` within 80 ms after that line. The value is at most 0.02.
- `reading-chrome-entry-title` stays `blocked`. The shell title is a GPUI row. This run does not read that renderer. An absent or present `NSTextField` does not pass the row, and `title_opacity` does not stand in for it.
- `reading-chrome-enter`, `reading-chrome-short-leave`, and `reading-chrome-leave` read one session's `chrome.json` samples. Enter ends at button alpha of at least 0.9. A leave shorter than 400 ms does not drop the alpha. A leave that stays away holds for 400 ms, then steps by 0.25 through 0.75, 0.5, and 0.25 to 0. A poll gap over 120 ms blocks that sampled row. The other rows keep their own results. `summary.json` is not ok while the gapped row is `blocked`.
- `reading-chrome-resize` requires a real change of `window_w` or `window_h`, a higher `generation`, the pointer outside the top band, the same 400 ms hold, and close and zoom centers within 0.5 points.
- `reading-chrome-blur` requires `focused` false and button alpha at most 0.02 on that sample. The driver writes `chrome-blur`, `shell-frame`, and `bar-fullscreen` inside `/tmp/superreader-spike/verify-<session>/` and sets `SUPERREADER_VERIFY_SESSION` on that process. Without a valid session those names have no path, so a root `shell-frame` or `bar-fullscreen` does not resize or enter full screen, and a leftover blur file does not call `resignKeyWindow`. Staying key is a fail.
- `reading-chrome-fullscreen-align` requires `fullscreen` true, a window larger than 578 by 528, and the same button-center rule. Missing button frames are `blocked`.
- `reading-chrome-minimum-align` requires the window within 2 points of 578 by 528 and the same button-center rule.
- `reading-chrome-shot-entry`, `reading-chrome-shot-shown`, `reading-chrome-shot-hidden`, `reading-chrome-shot-fullscreen`, and `reading-chrome-shot-minimum` record a `screencapture -l` file. The row passes when the file decodes as a PNG. That pass is capture integrity. A missing file is `blocked`. A file that is not a PNG fails. `reading-chrome-visual` stays `blocked`. It is the pixel acceptance, and a PNG capture does not pass it.
- `window-aqua` puts the Mac in Dark Mode, then opens Pride. The window's effective appearance is aqua, the paper in the top band is `#ffffff`, and the toolbar title and contents icon stay dark. The contents item's leading edge is 26 pt past the zoom button, within half a point, and its frame misses the traffic lights. `window-aqua-light` repeats that frame check in Light Mode. The previous appearance is restored.
- `flash-pride` and `flash-zhaohua` record the open, a corner drag, and the trim change that animates the window frame. A region that is neither white nor ink fails the run when its area is at least 20 by 20, including a strip a few pixels wide that runs the height of the page. A glyph's gray edge, the pixels within two of the ink, is not a flash. The white cutoff stays at 242.
- `book-body-background` reads the computed body background. Pride and 朝花夕拾 are both `#ffffff`. The settled shots have no block of any other color. A selection highlight is not a flash.
- `page-bands-white` samples the header band, the footer band, both side margins, and the text block of Pride and of 朝花夕拾 范愛農. Every paper pixel is `#ffffff` ±1. Glyphs are skipped. A cream band fails even when the text block stays white.

## How to get to it (user POV)

- Launch with `SPIKE_MODE=webkit`.
- At rest the window is paper. The lights and the title are hidden.
- Move the pointer to the top edge. The lights and the book title fade in.
- Choose a trim from the View menu. The window resizes and keeps the top edge.
- Enter full screen. The page grows to the screen height and stays centered in the area to the right of the sidebar. The type grows with the page.

## Driving it with scripts/verify

Preconditions:

- `scripts/verify --doctor` passes.
- The settings file is restored after the chrome shots.

- **Paperback.** Write `paperback`, launch Pride, and wait until `chrome.json` reports a page and `webkit.json` reports the chapter. The window from `CGWindowListCopyWindowInfo` matches `chrome_plan`. The page top clears the lights. Vision reads the footer. `fill_gap` is under `line_h`. The footer edges match the measure. The band from `last_bottom` to `footer_text_top` has no divider pixels.
- **Trim.** Write `trade` while that process is up. The window ratio follows 140 by 216.
- **朝花夕拾.** Launch with `SPIKE_BOOK=zhaohua` and `SPIKE_AT=3`.
- **Hover.** Park the pointer away, then move it to the top of the window. `lights` in `chrome.json` reaches 1.
- **Reading chrome.** `just verify --subset reading-chrome` launches one form-fixture library, presses Pride and Prejudice, and samples that process. The shell receives `SUPERREADER_VERIFY_SESSION`, and the blur file, the pin, the fullscreen flag, and `chrome-status.jsonl` stay in that session directory. `scripts/verify --self-test` scores a recorded sample log and does not launch the app. `nsh test` is not used. A host that is not Darwin, a locked console, or a missing press blocks the session. A coarse poll blocks the sampled row.
- **Full screen.** Launch with `SPIKE_FULLSCREEN=1`. `fullscreen` is true, the footer is in the picture, and the page fills the height under the top mat.
- **White flash.** Launch Pride, then 朝花夕拾. Record from the first window id through the open, drag the bottom-right corner outward, and switch the saved trim so the frame animates. A rectangle that is neither `#ffffff` nor ink fails when it is at least 20 by 20.
- **Publisher background.** The computed body background on Pride and on 朝花夕拾 is `rgb(255, 255, 255)`. The settled shots have no block of any other color. A selection highlight is not a flash.
- **Proof.** The rows are `pass`. The pictures are copied to `docs/prototypes/window-chrome/`.

## Gotchas

- `child_env` strips `SPIKE_FULLSCREEN`.
- A resting shot parks the pointer at the bottom of the screen so the lights stay hidden.
- A reading-chrome shot row passes only when the file decodes as a PNG. That pass does not score pixels, and it does not pass `reading-chrome-entry-status`, the fade rows, or `reading-chrome-visual`.
- `chrome-b-hover` reaching `lights` 1 is not the 400 ms sequence.
- Full screen uses the whole display for a few seconds.
- The window id is the largest layer-0 window for the process. A fullscreen animation can list a short sliver first, and `screencapture -l` of that id fails.
- The page edge used to ask a tagged `NSColor` for `CGColor` and aborted on this OS. It now builds the color with `CGColorCreateSRGB`.
- The 2026-11-11 shell puts this page inside the white card. These checks still score the current window until S3. The card checks are in `shell.md`.
