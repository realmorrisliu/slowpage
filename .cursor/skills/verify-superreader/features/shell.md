# Shell

The approved window is the gray base, one white card, the version B sidebar, and a right region that changes with the form. The library form's right region is the chat panel. The reading form's right region is the annotation margin. The layout reference is `docs/design/shell/spec.md`. None of these checks exist in the spike yet. Each stays `pending` until the slice that owns it passes. Pending does not fail the exit code.

## Sub-features

- `shell-card-inset` is S1. The card is inset 6 points from the base on every side in Full, with the sidebar and the chat panel open.
- `shell-no-dividers` is S1. The gaps between the sidebar, the card, and the chat panel contain no divider pixels.
- `shell-sidebar` is S1. The sidebar is 220 points wide and lists **书库**, **笔记**, **书单**, **来源**, and **设置** in that order, with the rows named in the spec.
- `shell-panel-open` is S1. Launch shows the chat panel open, with the placeholder **问问 Slowpage…**.
- `shell-cover-grid` is S2. The card shows the cover grid, the title and the author under each cover, and a **继续阅读** row. The books come from `Catalog`. At the 1440 point mockup width the grid is five columns.
- `shell-form-fixture` launches the twelve-title form fixture at 1440 by 900. The card inset stays 6 points. **继续阅读** lists Pride and Prejudice, then 三体, then The Art of Computer Programming.
- `shell-form-parity` writes `side-by-side.png` and `overlay-sidebar.png`, `overlay-header.png`, `overlay-continue.png`, `overlay-shelf.png`, and `overlay-chat.png` against `docs/design/shell/b-rows.png`. The detail is the mean channel delta of each region. A region past its max fails. The maxes are sidebar 12%, header 12%, continue 28%, shelf 42%, chat 18%.
- `shell-shelf-geometry` reads `geometry.json` written by the view. At the 1440-point mockup the cover is 128 by 192, the column gap is 42, the row gap is 28, and the caption is 42, each within 1 point. The column count is 5. Other window sizes must match the same continuous rule at that launch's content width.
- `shell-clip` reads `layout.json` at the minimum window (578 by 528), at 1440 by 900, and at full screen. The view writes every text, cover, button, sidebar row, toolbar item, shelf header, and chat header frame with its container and intrinsic size, plus the traffic-light frames measured in `chrome.json` when that file is the same window. A frame that leaves its container or the window fails. Two frames that cross, with neither inside the other, fail as well. Scroll children may pass the viewport on Y only. Text wider than its frame fails unless `ellipsis` is true. Toolbar items must share the traffic lights' vertical center, recorded as `chrome_center`, within 0.5 points. Sidebar, shelf, and chat edges must sit on the pads in `docs/design/shell-tokens.md`. An icon and its label share a row center. A title and its count, and the chat name and subtitle, share a baseline. Repeated column gaps and continue-card gaps are equal, within 0.5 points. A coverless plate carries title and author frames inside the cover, and the capture must show ink there and the plate's sRGB color on the face. A wheel over the shelf grid must move pixels. The same launch also walks the accessibility tree and fails when a named element's horizontal frame leaves the window. The three captures are `form-fixture.png`, `form-minimum.png`, and `form-fullscreen.png`. Each size also writes 4x crops: `form-fixture-toolbar-4x.png`, `form-fixture-sidebar-4x.png`, `form-fixture-header-4x.png`, `form-fixture-sidebar-header-4x.png`, `form-fixture-search-4x.png`, `form-fixture-shelf-title-4x.png`, `form-fixture-composer-4x.png`, `form-fixture-sidebar-bottom-4x.png`, and `form-fixture-chat-bottom-4x.png`, and the same names for `form-minimum` and `form-fullscreen` when that region is on screen. While `lights` is false the toggle's left edge is the sidebar pad, not `traffic_bar`. The fullscreen launch records consecutive `layout.json` frames and fails a jump, a clipped search or shelf title, or a windowed row that does not return to its first position. The composer card and the settings row stay at least `window_corner` from the window edge. The placeholder and the paperclip share a left edge. The book chip hugs its text and shares the send button's center and height. Later slices run this same check.
- `shell-shelf-scroll` requires `scrolls` true and the last shelf title in the accessibility tree, so a row below the 855 point window can be brought on screen.
- `smoke-shell-click` stays pending. A cover click leaves the library and shows the existing reader chrome. Escape or Command-[ returns while editing is idle and immersive is off. This window check has not been driven. The margin form still waits on the mockup. The reader-forcing launch is `SPIKE_BOOK`.
- `shell-reading-hides-sidebar` is S3. Opening a book hides the sidebar and keeps the page in the card. The right region is the annotation margin.
- `shell-reading-margin` is S3, and it has no pixel bar yet. The exact gap, the horizontal position, the margin width, and the toggle placement wait for Morris's mockup. Do not invent those pixels in the check.
- `shell-immersive-no-gray` is S4. Immersive reading shows no gray field. The card meets the window edges.

## How to get to it (user POV)

- Launch the app. Full shows the sidebar, the library card, and the chat panel.
- Open a book from a cover or from **继续阅读**. The sidebar hides. The page is in the card. The right region is the annotation margin.
- Press Control-Command-I, or use the View menu item, or use the toolbar button. The card goes edge to edge. Escape leaves that form.
- Choose **全部书籍** to return to the library and the chat panel.

S3's reading checks wait until Morris approves the margin mockup. Do not drive a substitute margin.

## Driving it with scripts/verify

Preconditions:

- `scripts/verify --doctor` passes.
- No other `spike-shell` is running.
- The ids below are not in `scripts/verify` yet. The slice that builds the form adds its ids. Until then the summary has no row, and a missing row is not a pass.

- **Inset.** In Full, measure the card against the base on all four sides. Each gap is 6 points, within 0.5 points. `shell-card-inset` stays `pending` until that measurement passes.
- **Dividers.** Sample the bands between the sidebar and the card, and between the card and the chat panel. A divider pixel fails. `shell-no-dividers` stays `pending` until a clean sample passes.
- **Sidebar.** The sidebar width is 220 points, within 0.5 points. The group labels are **书库**, **笔记**, **书单**, **来源**, and pinned **设置**. `shell-sidebar` stays `pending` until that reading passes.
- **Chat panel.** At launch the chat panel is open and the field shows **问问 Slowpage…**. `shell-panel-open` stays `pending` until that reading passes.
- **Grid.** With a non-empty `Catalog`, the card shows the cover grid and **继续阅读**. `shell-cover-grid` stays `pending` until that reading passes.
- **Form fixture.** `SUPERREADER_FORM_FIXTURE=1` with the `library-fixture` `form` tree and `SPIKE_WINDOW=1440x900` opens the twelve plates, three reading rows, and the two chat exchanges. `shell-form-fixture` passes when the inset is 6 points and the continue row is in that order.
- **Parity.** The same window is captured as `form-fixture.png`. `shell-form-parity` writes the side-by-side and the five overlay diffs. A missing image fails. A region whose mean channel delta is past its max fails. The maxes are sidebar 12%, header 12%, continue 28%, shelf 42%, chat 18%.
- **Geometry.** `geometry.json` comes from the view, the same way `fonts.json` does. `shell-shelf-geometry` passes when the 1440-point launch is a 128 by 192 cover, a column gap of 42, a row gap of 28, a caption of 42, each within 1 point, and 5 columns. `content_w` is recorded so another size can be checked against the rule instead of these mockup numbers.
- **Clipping.** `layout.json` lists every text, cover, button, sidebar row, toolbar item, shelf header, and chat header with its frame, its container, its intrinsic size, and whether the text is ellipsized. `shell-clip` fails when a frame extends outside its container or the window, when two frames overlap and neither contains the other, or when text is wider than its frame and `ellipsis` is not set. It also fails when a chrome control's vertical center is more than 0.5 points from `chrome_center`, when a sidebar, shelf, or chat edge leaves its pad, when an icon and its label do not share a row center, when a baseline pair does not share a bottom, or when a repeated gap is uneven. Scroll children may extend past the viewport on Y. The check runs at 578 by 528, at 1440 by 900, and at full screen, and each size also asserts `geometry.json` against the grid rule and writes the 4x toolbar, sidebar, shelf-header, sidebar-header, search, shelf-title, composer, sidebar-bottom, and chat-bottom crops. While the traffic lights are hidden the sidebar toggle sits on the sidebar pad, and consecutive fullscreen frames must not jump or clip the search field or the shelf title. A composer chip wider than its text fails. A settings row or a composer card inside `window_corner` of the window edge fails. Later slices call the same check.
- **Scroll.** The shelf body scrolls. `shell-shelf-scroll` passes when `scrolls` is true and the last title is in the accessibility tree.
- **Composer.** `shell-form-fixture` accepts **问问 Slowpage…** from `fonts.json` when it is not an accessibility name.
- **Smoke click.** `smoke-shell-click` stays pending. A cover click leaves the library and shows the existing reader chrome. Escape or Command-[ returns while editing is idle and immersive is off. This window check has not been driven. The margin form still waits on the mockup. `SPIKE_BOOK` is the reader-forcing launch. Do not paint a page to make the click pass.
- **Reading.** Open a book. The sidebar is absent and the page is inside the card. `shell-reading-hides-sidebar` stays `pending` until that reading passes. `shell-reading-margin` stays `pending` with no pixel bar until the mockup lands and a later check names the pixels.
- **Immersive.** Enter immersive reading. The gray field is absent. `shell-immersive-no-gray` stays `pending` until that sample passes.
- **Proof.** A passing row names the check id and attaches the window PNG. A pending row is not `pass`.

## Gotchas

- `sidebar-centered` still scores the 2026-11-10 measured-width bar. Do not pass `shell-sidebar` from it. The top-row inset keeps that check's 0.5 point gap.
- `reading-page-box` still centers the page beside the old bar. Do not pass `shell-card-inset` from it.
- `shelf-empty` scores the bare shelf buttons. Do not pass `shell-cover-grid` or the empty card from it. S1 deletes those shelf checks.
- Pull request 39 insets a page sleeve on the old bar. Do not pass `shell-card-inset` from that branch.
- Hiding the chat panel is not immersive reading, and it is not the annotation margin. `shell-immersive-no-gray` requires the gray field to be gone.
- Do not pass `shell-panel-open` while a book is open. That check is the library chat panel at launch. The reading form's bottom input belongs to the annotation margin, and it is not this check.
- S5 adds no night-mode check. The 2026-11-03 rule still forbids night mode and a theme switch.
- Full screen is the style mask. `shell-clip` fails when `chrome.json` says `fullscreen` is false, when the window is still the windowed size, or when `form-fullscreen.png` is the same file as `form-fixture.png`. The run archives `frames/fullscreen-enter`, `fullscreen-settled`, and `fullscreen-exit`, plus `form-restored.png`.
- A continue gap counts `continue.0`, `continue.1`, and so on. `continue.section` is the label, not a card.
- The minimum window is 578 by 528. The extra 12 points are the card corner, so the shelf caption is not clipped.
