# Roadmap

## 1. Feasibility spike

Done on 2026-11-03. PDF is pdfium. EPUB is the system WKWebView. UNDER RE-EVALUATION (2026-11-10). Morris wants a fully native, pure-Rust EPUB implementation on NSH, for cross-platform reach and full control of layout. The first step is `crates/superreader-typeset`. The reader is still WKWebView. PDF via pdfium-render is unchanged. The comparison, including why hayro lost on text, is in `docs/spike/report.md`. No product features in this step.

## 2. Verification skill

Add the verify-superreader skill with `/create-verification-skill`, so later changes are checked against the goals and the walkthroughs.

## Next tracer bullets

The shell comes first. Morris decided on 2026-11-11 to finish the app shell before any other feature. The layout reference is `docs/design/shell/spec.md`. The final bar does not move. The rule is How we work in `docs/design/north-star.md`.

Each slice below is a thin cut of that one form.

- S1. The gray base, the 220 point version B sidebar, the white card, and the library chat panel are in the library form. The bare shelf window is gone. The GUI is not verified.
- S2. The cover grid and **继续阅读** are in that card. The GUI is not verified.
- S3. The reading form, using the existing reader. The sidebar hides, the page is in the card, and the right region is the annotation margin. The card, the 240 point column, the narrow cover, the top-right control, and PDF fit-page with the existing 160 ms slide are in the shell. The margin draws no note cards, because the shell has no annotation store. The gap that lines a card up with its line stays unset in `docs/design/shell/spec.md`. S3 is not done. The margin's contents are the 2026-11-10 hard requirement in `docs/design/north-star.md`. Interaction detail for S3, S4, and every slice after them is `docs/design/reading-interactions.md`. One checklist item, or a small group, is one pull request. It merges on the necessary cloud checks and an independent review.

The checklist document records the rule. It does not build an item, the margin mockup, or S4, and it does not start the alan embed. The document merges on the necessary cloud checks and an independent review. A green run does not pass the ten items. Not executed, and not a blocker for this document: the ten items, their `nsh test` runs, their Mac recordings, the real Apple Books round, and the shell GUI, performance, image, and animation checks. `deny.toml` still ignores RUSTSEC-2026-0543 (chrono 0.4.45 via nsh-base 0.5.1 and pdfium-render 0.9.5). That ignore is not a clearance. The spikes bump to rbook 0.7.13, sha2 0.11.1, resvg 0.50.0, and rusqlite 0.43.0 leaves chrono 0.4.45 in the lock. Those four crates declare rust-version 1.89 or older, and the toolchain is 1.98.1.

- S4. The immersive form. Control-Command-I, the menu item, and the toolbar button enter it. Escape leaves it.
- S5. Motion, and the fit of the three forms. Night mode and a theme switch stay out. Light and dark join this slice only if a later decision revises the 2026-11-03 rule.

## After the shell

These wait until S1 through S5 have landed. Do not start them ahead of the shell.

- [Pull request 39](https://github.com/realmorrisliu/slowpage/pull/39), the 6 point reading card on the old bar. The spec replaces that card. Leave the pull request until the shell slices land.
- [Pull request 45](https://github.com/realmorrisliu/slowpage/pull/45), PDF pages in the EPUB view. One PDF page fit inside the white card, with arrow keys turning pages. Still missing after that: animation, trackpad, edge click, and the footer page number.
- [Pull request 47](https://github.com/realmorrisliu/slowpage/pull/47), per-PR verify selection and a single `spike-shell` session.
- Repo cleanup, including worktree cleanup in `scripts/cleanup-worktrees.sh`.
- Calibre compatibility beyond the catalog `spikes/library` already reads. Writes, schema user_version 28, search, and the rest of the Library checklist below.
- [Pull request 14](https://github.com/realmorrisliu/slowpage/pull/14). The shell spec replaces its split into A1, A2, A3, B1, and B2. The margin mockup replaces B1 and B2 as the annotation next step, and that mockup is an approval, not a code start.
- Reading chrome leftovers from [pull request 8](https://github.com/realmorrisliu/slowpage/pull/8). The shell reading window now shares one fade for the traffic lights and the book title. The dev spike path is unchanged. Checklist item 8 is not done. Do not put the AppKit toolbar back. The 朝花夕拾 narrow-window two-column bug and the page scrubber stay here too.
- Owned typesetter follow-ups from [pull request 13](https://github.com/realmorrisliu/slowpage/pull/13). Still missing: passing gate B, CJK, and switching the default off WKWebView.
- The rest of the Books toolbar icons. S1 uses the list glyph the contents control already ships.
- Bold Word Starts, [pull request 21](https://github.com/realmorrisliu/slowpage/pull/21). Still missing: the WebKit rendering, the breaker and its cache key, and the owned typesetter.
- Books-white, [pull request 15](https://github.com/realmorrisliu/slowpage/pull/15), is already merged. Nothing for white is still missing.

## Core

The library, SQLite annotations, tantivy, and the `superreader` command with JSON output. Writes from that command are applied by the running app, because alan's Seatbelt sandbox denies writes outside mounted directories.

The calibre and [dalibre](https://github.com/djyde/dalibre) targets are in Library below. They wait until the shell slices land. S2 draws the `Catalog` the shelf slice already reads. S2 does not start those targets.

## Library

The library has to reach Calibre, then pass [dalibre](https://github.com/djyde/dalibre) (Go, native UI, no webview). Read on 2026-11-10 from dalibre's README, changelog, and `docs/mcp` and `docs/plugins`. Dalibre publishes no memory, import-speed, or search-latency figures. None are recorded below. Every number under "Where we must be better" is a target, not a measurement.

License. Dalibre's repository root declares no license. Its README says `internal/calibre/metadata_sqlite.sql` comes from calibre (GPL-3.0). SuperReader is MIT OR Apache-2.0. This section is a design reference only. Never copy code, SQL, or schema files from dalibre or calibre. Implement calibre compatibility ourselves from the publicly documented formats, calibre's user manual and the OPF specification, described in our own words.

Pure Rust. The library is Rust only. That covers reading and writing a calibre library (`metadata.db`, the OPF sidecar, and the naming rules), metadata fetch, the plugin and book-source system including its sandbox, the MCP server, and auto-update. None of that may depend on Go, Python, or a calibre install, at runtime or at build time.

Checklist. Each item is what we match, then where we have to be better.

- Calibre library, read and write. Match: open and write a library whose `metadata.db` is schema user_version 28; supply the SQL functions calibre's triggers call, including title sort and uuid generation; follow calibre's directory and file naming, ASCII transliteration included, and move the folder when the title or the first author changes; on edit, write the OPF sidecar and mark the book metadata-dirtied so calibre refreshes it. Better: the metadata fields keep the original CJK. We do not vendor calibre's schema script.
- Search, filter, sort, sidebar counts. Match: query, author, series, tag, publisher, language, format, recently added, sort, and a count per facet. Better: tantivy searches the books, not the catalogue fields alone. Target, not a measurement: a title or author query over 10,000 books returns in under 50 ms at the median, release build.
- Cover thumbnails. Match: thumbnails stored on disk for the shelf. Better: covers and motion at Apple Books' level. Target, not a measurement: once those thumbnails are on disk, scrolling a shelf of 1,000 covers stays at 60 Hz.
- Metadata plugins. Match: Google Books and NeoDB lookups. The reader accepts a candidate before it fills title, authors, identifiers, or the cover. Better: CJK names, series, and publishers round-trip with the original characters, not an ASCII fold.
- Book-source plugins. Match: Project Gutenberg built in, and further sources as JavaScript plugins in a sandbox. A plugin searches and points at a file; we download it and add the book. The sandbox cannot be GPL or AGPL. Candidates to evaluate later, no choice yet, and only ones we can embed from Rust under a permissive license: Boa (MIT), a JavaScript engine written in Rust, and rquickjs (MIT), Rust bindings to QuickJS-NG (MIT). rquickjs still links that C engine, so it has to clear the pure-Rust rule above before it can be chosen.
- MCP. Match: an external agent can search the open library, edit metadata, and add books. Ours is already the plan in `docs/design/north-star.md`. The `superreader` command prints JSON. The running app applies writes, because alan's Seatbelt sandbox denies writes outside mounted directories. The MCP server is the thin wrapper for clients that cannot run a terminal. It exposes the same search, edit, and add jobs as that command, and it does not open the database itself.
- Auto-update. Match: updates come from signed GitHub releases. Dalibre's changelog 0.3.0 says it checks every hour. Target, not a measurement: a signed check on launch and at most once an hour, and an unsigned payload is refused.

Where we must be better:

- Apple Books-level UI. The shell spec is that shelf and that reading window. S1 and S2 build the shelf. S5 is the motion. The calibre targets on this list still wait.
- Agent-native features. Ask, summarize, cross-book search, and notes, over the same MCP wrapper. Dalibre's MCP edits the catalogue. Ours also reaches the annotations and the full-text index.
- Memory footprint. Dalibre publishes none. Target, not a measurement: idle RSS under 200 MB on macOS with a 10,000-book library, release build.
- Import speed. Dalibre publishes none. Target, not a measurement: 1,000 EPUBs into a calibre-compatible library in under 60 seconds, covers included, release build, on the machine that runs verify.
- Search latency. The 50 ms median target above. It is a target, not a measurement.
- CJK metadata. File names written into a calibre library stay in calibre's ASCII form so calibre can find them. Display, search, sort, and the OPF text keep the original characters. A Chinese author and title survive our edit and still open in calibre.

## Reading

Done on 2026-11-03. A single-page PDF and EPUB reader on NSH v0.4.0. PDF pages are pdfium through pdfium-render, and that choice is unchanged. EPUB pages are WKWebView. UNDER RE-EVALUATION (2026-11-10). The owned typesetter's first step is in the tree. The page is still the WebKit page. The publication's type and layout win. The page box uses the saved trim, or the book's own page size when it declares one. The default trim is paperback, 129 by 198 mm, picked by Morris. 32开 and US Trade stay as the other choices. Pinch zoom and Command zoom re-render the page. Highlights hang off the spike's anchor. No spread, and no night mode.

The reading window landed on 2026-11-04. The lights and the book title fade in at the top. The footer is in the page margin. Full screen scales the page and the type together. The 2026-11-11 shell puts that page inside the card. See `docs/design/shell/spec.md`.

The page and the card are Books-white `#ffffff`. There is no cream or yellow paper. The shell base is the light gray field in `docs/design/shell/spec.md`. Until S1 the spike still paints the chrome the same white as the page.

The Knuth-Plass pass is in the EPUB page. It chooses line breaks and the space between words. Font, size, line height, indent, and alignment stay. `SPIKE_JUSTIFY=0` leaves every paragraph native. A paragraph with CJK stays native. 朝花夕拾 and the plain Chinese fixture report `owned` 0 and `forbidden_starts` 0 at paperback, 32开, and US Trade.

`crates/superreader-typeset` is the headless first step of the native engine. It does not replace the WKWebView page. Phase 1 passed on this Mac on 2026-11-10. The run is commit 64fa573, 1-minute load 7.68, release build. Pride chapter 1 at column 343.47, Iowan Old Style 22px, scored crate p95 1.978 and max 3.9429 against the reader's 3.4286 and 5.4286. At column 383.78 the crate scored 1.7174 and 2.1754 against the reader's 3.1429 and 4.5714. 朝花夕拾 范愛農, Songti SC 20px, column 372.06, has no scored Latin gaps, so that p95 compare is skipped. Kinsoku was 0, overflow was 0, and the warm chapter medians were 5.585 ms, 5.189 ms, and 6.527 ms. Phase 1 of the typesetter gate replaces the word-gap p95 ≤ 1.37 bar and the owned-line-only set. One gap is the drawn inter-word space divided by that font's natural space, sampled per space, nearest-rank p95. The sample is every justified line in the chapter. A paragraph's last line is out, and so is a line with fewer than two spaces, a CJK line, and a line that is not justified. The four conditions are: the crate's all-lines p95 is no worse than the reader's all-lines p95 measured on the same build, fonts, and width; zero CJK kinsoku violations; zero overflow; and at most 50 ms per chapter. The recorded baseline, which is not the bar, is Pride chapter 1, paperback, Iowan 22px, column 343.47 CSS px, where the reader scored p95 3.4286 and max 5.4286 (owned lines 1.2478/1.4799, WebKit-fallback lines 3.5714/5.4286), measured by bc-498e04f9 on cfe3404 at load 8.9. The gate runs on strict Knuth-Plass. `--font-expand` and `--letter-space` stay off by default, and a run with either on does not pass. Tighten later. Latin breaking is TeX's three passes held to the 0.75 cap, then a minimum-demerit set when a paragraph still cannot fit. The breaker matches the JavaScript breaker on the same items. The page images are Apple Books white, #FFFFFF. Still missing from the final bar is the Mac glyph-run view behind a debug switch, then the switch that opens that view, then that view becoming the page in place of WKWebView. The shell slices build the shelf UI. The library targets that beat dalibre stay after those slices.

`scripts/verify` scored one release run on 2026-11-04. The record is `docs/verify/summary.json`. The run is `docs/verify/runs/20261103T183652Z`. With the pass off, the Pride opening was loose. Paperback `gap_p95` was 3.108 on 10 justified lines, 8 of them over 1.5, with 1 river. 32开 `gap_p95` was 3.599 on 10 justified lines, all 10 over 1.5, with 1 river. `docs/design/justification/before.json` stores that census. The breaker is given the column's content width: the page width minus the padding in the page sheet, `max(14%, (100% - 48ch) / 2)`. On the settled paperback that width is 383.78 px, and a DOM measure of "acknowledged" matched the canvas measure at 138.4 px. `box_err` is the distance between that box and the box the paragraph actually laid out in. It was 0 at every English trim. Each non-last owned line meets both edges within 0.25 px. An earlier pair of screenshots compared the opening frame, about 505 by 775 px and 9 pages, with the settled frame, 534 by 819 px and fewer pages. The pass was the only thing still invalidating the view after the title fade, so only the fitted shot reached the settled frame. Both shots are the settled frame now. At that frame native Pride is 8 pages and the pass is 7, because the breaks fill the same measure more evenly. Pride, whole first chapter: paperback p95 1.359 max 1.491, 0 of 59 loose, 6 of 59 hyphenated, flush 0.23. 32开 p95 1.386 max 1.489, 0 of 62 loose, 6 of 62 hyphenated, flush 0.25. US Trade p95 1.379 max 1.482, 0 of 60 loose, 7 of 60 hyphenated, flush 0.25. Rivers and overflow were 0. The unstyled English fixture, also the whole chapter: paperback 9 of 76 hyphenated, p95 1.379. 32开 5 of 57, p95 1.355. US Trade 9 of 67, p95 1.377. The widest hyphen share is 13.4 percent. The 15 percent check stays. A line may start with an intact `——` or `……`. A comma, a period, or a split pair still counts.

`OWN_PUBLISHER` stays false. A separate run, with that constant temporarily true, opened the Literata fixture whose own sheet sets justify. Paperback failed at p95 3.421 and max 3.421, with 4 of 12 lines loose. US Trade failed at p95 2.105 and max 2.368, with 3 of 12 lines loose. 32开 passed at p95 0.789, with 0 of 10 loose. `docs/design/justification/publisher-own.json` stores that run. The verify summary still expects `owned` 0, because the constant in the tree is false.

A later open reads line breaks from `~/Library/Application Support/app.slowpage.reader/breaks/<book>.json`. The key is the book hash, the chapter href, the paragraph index, the content width, the first-line width, the font, the indent, the language, and the incoming hyphen run. The breaker waits until the web view is within 1.5 px of the settled page width, so a reopen is not keyed off a frame from the window animation. Paragraphs the breaker leaves to WebKit are stored too, or the next open would run `breakLines` again for them. `reading-warm-cached` requires `break_source` `cached` and `live_calls` 0. It is not the 300 ms bar.

There is one web view. `SPIKE_NEXT=1` loads the next chapter into that view after the open chapter's layout is done, and `chapter_ms` is that load. `scripts/verify` sets `SUPERREADER_SUPPORT_DIR` to a temp directory for that run.

On the release run in `docs/verify/summary.json`, the one-minute load at start was 9.7, with no other `spike-shell`. During the opens it was 15 to 35. Cold open max 1076.6 ms, median 952.7 ms. Native on that interleave was 881 to 990 ms. Warm open max 1093.3 ms, median 928.0 ms, and all five fitted samples were `cached`. Native on that interleave was 883 to 1070 ms. The cached reopen in `warm_phases` spent 3 ms applying breaks, 355 ms to finish navigation, and 555 ms after navigation before the page was ready. `page_ready_ms` was 910. The 300 ms bar is that whole `page_ready_ms`. Applying the stored breaks is 3 ms. Navigation finishes in 355 ms, and the page is ready 555 ms after that. The same web view then loads the next chapter. Fitted `chapter_ms` max 39.1 ms, median 35.7 ms, at load 14.8. `summary.json` `ok` is false because cold open and warm open stay over 300 ms.

WKWebView remains the EPUB page until an owned pure-Rust typesetter passes. This run keeps the line-break cache.

Pretext, chenglou/pretext, waits until figure placement.

## Icons

Morris picked `generic` / `list` for the contents control. That glyph is on main. S1 uses it as the sidebar toggle. The remaining toolbar icons wait until the shell slices land. [Pull request 14](https://github.com/realmorrisliu/slowpage/pull/14) is not the shell plan. The layout reference is `docs/design/shell/spec.md`.

## PDF paged reading

Queued after the shell slices, with [pull request 45](https://github.com/realmorrisliu/slowpage/pull/45), and before the calibre targets in Library.

The spike still scrolls PDFs in one column. Replace that with fit-page inside the white reading card, EPUB's page-turn interaction, and pdfium with adjacent-page cache. The hard requirements are in `docs/design/north-star.md`, PDF reading. EPUB and PDF both run the checklist in `docs/design/reading-interactions.md`. Item 10 of that checklist is this paged PDF rule.

The implementing pull request must attach prototype screenshots for Morris's approval before the code change: fit-page in the card at two window sizes, and a mid-turn frame.

Verify:

- The page fits inside the card without cropping.
- Aspect ratio is preserved within 0.5%.
- Turn latency stays within the reading bars in `docs/design/north-star.md`.
- The footer page number is correct.
- A highlight survives a page turn and a window resize.

## Comments

The reading form shows a highlight, a note, and an agent response in the annotation margin. `docs/design/shell/spec.md` is that decision. Which margin facts are approved is the annotation margin section of `docs/design/shell/spec.md`. The shell draws that column, cover, and control, and no note cards. The gap that section still marks unset stays unset. A highlight can carry the reader's note. `@alan` in that note runs the handoff below.

## @alan

Picked on 2026-11-17. This decision supersedes the 2026-11-03 one-shot process. The full decision is in `docs/design/north-star.md`. SuperReader embeds alan in-process through `alan_os_host::embed` in [alan pull request 1109](https://github.com/realmorrisliu/alan/pull/1109). A reply is `EmbedEvent::TextDelta` from `EmbedEventStream::next_event`. SuperReader's SQLite owns the thread. Each ask sends that thread as context.

Integration starts after S3 and S4. This repository does not start it now.

## Distribution

WKWebView _setBackgroundColor: SPI, revisit before any Mac App Store build.

## S3 shelf return

Goal. A cover or **继续阅读** opens the existing reader. Escape or Command-[ returns to the library while editing is idle and immersive is off. The shelf scroll and the reading page stay. **继续阅读** lists the book just opened first.

Scope. The visit is library or reading. The library form unmounts while reading, and the existing title and footer show. Shelf scroll is the offset the shelf already records, restored when the form mounts again. The page stays in settings history.

Non-goals. No margin layout, no new motion, no PDF page architecture, and no immersive enter path.

Observable. The EPUB fixture Plain Essay and the PDF fixture Field Guide keep scroll and page across leave. **继续阅读** follows `opened_at`. Editing focus and immersive Escape do not leave the book. A wheel on the open reader reaches the page. A wheel on the library moves the shelf.

Evidence. `spikes/shell/src/visit.rs` tests. The headless `shell.test.tsx` case leaves the reader between the two opens. macOS CI runs the visit tests. `smoke-shell-click` stays pending because no window was driven.

Not run. Mac GUI, smoothness, frames, animation, and PDF runtime loading.

## S3 file open

Goal. A file opened from Finder or the Dock, at launch or while Slowpage is already running, opens that book. A second file replaces the book on screen. A file that is not an EPUB or a PDF, or whose bytes do not match, stays on the current book and shows 打不开这个文件.

Scope. `next_open_files` and a drop on the window use one decision. An EPUB starts with `PK`. A PDF starts with `%PDF`. The exploded EPUB directory still counts. The visit switches to reading and the stage receives `open`. Cold start from a command-line path stays the launch target it already is.

Non-goals. No new animation, no margin, no new type, and no new PDF renderer. A file whose header matches and whose pages still fail to load is outside this slice.

Done. Plain Essay and Field Guide switch the visit and keep the shelf offset. A bad file leaves the open book in place and is not queued for the stage. A handled arrival does not open again when the reader leaves the book. The refusal is the sentence above.

Evidence. `spikes/shell/src/visit.rs` reads those two fixtures. macOS CI runs that test. `nsh test` does not deliver `openFiles` on this machine.

Not run. A Mac window of Finder, the Dock, and a second real PDF loading in pdfium. Smoothness, frames, and animation.

## S3 page turn

Goal. While a book is open, the right arrow and Space move one page forward. The left arrow and Shift-Space move one page back. The first page and the last page stay put. The footer shows that page. On the library those keys do nothing.

Scope. The keys use the turn the reader already has. An EPUB keeps the 160 ms slide. A PDF keeps the existing page offset. The product PDF footer uses the same line the EPUB footer already uses. The visit page follows the page the stage reports.

Non-goals. No new slide, no edge target, and no finger-following swipe. The EPUB swipe that is already in the page stays. A page whose header matches and whose bytes still fail to load is outside this slice.

Done. Page 4 of 10 goes to 5, then back to 4. Page 1 does not go back. Page 10 does not go forward. The library form stays. A direct reader launch, whose visit still starts on the library surface, still turns. The product footer for page 2 of 5 is 2/5页 and 本章还剩3页.

Evidence. `spikes/shell/src/visit.rs` tests the key and the stop. The CI `test` job runs that test. The Release `macos` job compiles, packages, signs, and notarizes. It does not run this test.

Not run. A Mac window, `nsh test`, the key event itself, pdfium drawing the turned page, smoothness, frames, and animation.

## S3 held key

Goal. Holding the right arrow, or the left arrow, keeps turning one page for each repeat the system delivers. The book stops on the page that last repeat asked for. A repeat at the first or last page stays on that page.

Scope. The page's key listener delivers a repeated arrow, Space, or Shift-Space to the turn it already uses. A repeat that arrives while a slide is painting takes the one queue slot. A second repeat replaces that slot. It does not add another page. The posts from those turns are the place `handle_page` writes.

Non-goals. This does not complete checklist item 9. A repeated contents click, leaving and reopening a book, and a resize during a turn stay in the swipe slice. No new animation, no margin, and no PDF fit-page.

Done. On a 400 point column, one right arrow and two repeats while that slide is painting answer `ok`, then `queued`, then `queued`. The column ends at 800, which is two pages on, and the file reopens on the last of those turn posts. Two left arrows at the first page, the second a repeat, stay at scroll 0.

Evidence. `a_key_during_the_release_still_turns` fires those key events on the page script, then writes the turn posts through `handle_page` and `SettingsBook` and opens the file again. The CI `test` job already names that test. The Release `macos` job compiles, packages, signs, and notarizes. It does not run the test.

Not run. A Mac window, the key repeat itself, WebKit painting the slides, `nsh test`, and the feel of the hold.

## S3 edge turn

Goal. While a book is open, a click on the left of the page moves one page back and a click on the right moves one page forward. A click in the middle does not turn. The first page and the last page stay put.

Scope. The click uses the turn the reader already has. An EPUB keeps the 160 ms slide. A PDF keeps the existing page offset. The outer quarter of the page rectangle is the edge. The middle stays so a selection click there still selects.

Non-goals. No new slide and no finger-following swipe. This slice does not draw the blocked-turn ease-back. The EPUB swipe that is already in the page stays. A click on the footer does not turn. The library ignores the click.

Done. On a page rectangle from x 100 to 500, a click at x 199 goes back, a click at x 401 goes forward, and a click at x 200 stays. A click outside the rectangle stays. A move of 3 points is a drag and does not turn. A point inside the footer rectangle does not turn. Page 1 does not go back. Page 10 does not go forward.

Evidence. `spikes/shell/src/visit.rs` tests the click rectangle. The CI `test` job runs that test. The Release `macos` job compiles, packages, signs, and notarizes. It does not run this test. The EPUB page script copies the same fraction. That copy is script generation, not a delivered click.

Not run. A Mac window, `nsh test`, the click event itself, pdfium drawing the turned page, and the 160 ms slide.

## S3 sentence

Goal. After a page turn, closing the book and opening it again shows the same sentence. The page number alone is not enough. A reopen on the right page and the wrong sentence fails.

Scope. The sentence is the character offset the page already measures near the top of the column. That offset is stored with the page. Opening the book uses the existing reveal to put that character back on screen. The same page keeps a stored sentence when a later sample is empty. A new page replaces it.

Non-goals. No new slide and no finger-following swipe. A table of contents, a bookmark, a search result, and an in-book link stay out. A fitted PDF page already shows the whole page, so this slice does not add a second position inside that page. The blocked-turn ease-back is the S3 blocked turn slice.

Done. Page 4 with offset 800 stays 4 and 800 when the same page reports an empty sample. The same page with offset 860 becomes 860. Page 5 with offset 20 becomes page 5 offset 20. A stored file keeps 800 across an open that does not record a new sentence, and a later turn stores 20. The opening page is not written while a reveal is still pending, and a nonzero sample from that page is not the reveal report. The report is the pinned offset. A missed target does not replace the stored sentence.

Evidence. `spikes/shell/src/visit.rs` tests the sentence rule. `spikes/shell/src/settings.rs` writes the offset and reads it back. The CI `test` job runs those tests. The Release `macos` job compiles, packages, signs, and notarizes. It does not run these tests. The page script reports the offset. That report is script generation, not a quit and a reopen.

Not run. A Mac window, `nsh test`, quitting the app, the reveal on screen, and pdfium.

## S3 chapter

Goal. While a book is open, the contents control lists that book's chapters. Choosing one lands on that chapter. **返回** goes back to the sentence the reader was on.

Scope. The list is the spine the reader already opened. One chapter uses the book title. The jump uses the existing reveal. A different chapter loads that spine item, then reveals its start. The place before the jump is kept for one return. A row that is not in the book does not move. The stored place keeps that spine item, and opening the book loads it before the sentence. A chapter choice drops a leftover opening-page turn.

Non-goals. No new slide, no finger-following swipe, and no margin. A bookmark, a search result, an in-book link, and a footnote stay out. A PDF outline stays out. The blocked-turn ease-back is the S3 blocked turn slice.

Done. Plain Essay is the fixture file. Choosing Netherfield in that chapter reveals that character and keeps page 4 offset 800. Return reveals 800. Choosing the second spine item loads that item at its start and keeps page 4. A missing row stays. A stored sentence of offset 0 is kept when the reveal asked for the chapter start. A pin from an earlier reveal does not replace it. An empty-href row for page 3 goes to page 3, and return goes to page 1. The same page and offset in a different spine item is a new place. An unknown spine item opens the first one. A chapter choice clears an opening target of page 4, and a choice that stays keeps it. A chapter command clears a leftover resume offset. A second choice before the first poll stores the return at that chapter's jump offset. A stored file keeps `/OEBPS/next.xhtml` across an open, and a later progress write replaces it. The shell shows the existing contents button while the book is open and hides it on the library.

Evidence. `spikes/shell/src/visit.rs` tests the choice and the return. The CI `test` job runs that test. The Release `macos` job compiles, packages, signs, and notarizes. It does not run this test. The menu is AppKit. That wiring is not a delivered click.

Not run. A Mac window, `nsh test`, the contents click itself, the reveal on screen, and pdfium.

## S3 swipe

Goal. On an EPUB page, a horizontal trackpad swipe follows the finger. Letting go before the page commits returns to the same page. Letting go after it commits turns one page. Progress changes only for that committed page.

Scope. The finger and its inertia are AppKit `phase` and `momentumPhase` on the scroll-wheel monitor the shelf already uses. The page script paints the offset. A commit seeks one column. The commit distance is the existing 48 points. The return and the rest of the slide use the existing 160 ms. Keys, Space, Shift-Space, and an edge click keep that slide. A key, a contents choice, a return, a resize, a reopen, or another book drops the gesture that was in progress.

Non-goals. A PDF keeps its scroll offset. No fit-page paging. The follow does not move past the first or last page. A release that asks to go past that page eases back in the S3 blocked turn slice. The stored place does not change. No margin, no S4, and no toolbar.

Done. Ten points returns to page 4. A reversal back under 48 points returns. A reversal past 48 points turns back one page. Forty-eight points turns one page, and the inertia after that lift does not turn another. A second gesture turns one more page. Dropping the gesture for a key, a chapter, or a resize makes the lift a no-op. Space, an arrow, or an edge click during the 160 ms release waits for that committed page and then turns once. Shift-Space after that release still turns. A second lift during the commit, still past 48 points and not ignored by a key, queues one page and that page posts its own token. A key that arrives while that second finger is still down ignores the lift, so the lift does not turn, and a page key takes the one queue slot. An unrelated key does not take the slot. The first commit is not dropped. A cancelled second gesture does not turn. Dropping the gesture during the release does not seek. A key during the return still turns. A same-chapter reveal, a chapter jump, a return, a resize, a reopen, or another book during the release drops that glide, so it does not seek or post over the new place. Each commit the page accepts carries a token, either the glide it is running or the one queued page. A token the page refuses is not kept. The EPUB place lives in one reader. A message is parsed once. A shared apply writes `settings.json` only after that decision, and only then does the reader move. A failed write, including an empty book or page 0 where `record_progress` returns `Ok` and stores nothing, leaves the reader where it was. An accepted swipe, a key, or an edge (`kind: "turn"`) saves with `next_sentence` and ends the one resume walk. That walk lives on the reader. It ends when the shown column reaches it, and when a key, an edge, or a swipe is saved. A resize does not end it. The poll's walk posts `chase: true`. Those frames do not save. A layout sample does not save. A restore is not queued behind a commit. A key, an edge, or a swipe that has posted saves that column and ends the walk. One that has not posted returns to the last settled column, and the walk stays. A refused restore leaves the walk and does not hold the poll latch. A second lift's token stays queued. Shell left, right, Space, and Shift-Space, and a shell edge, eval the user command. The page script's own key and edge do too. `SPIKE_TURN` and `SPIKE_PLAY` are restores. A same-page sentence of 0 keeps the stored offset. A layout box does not save on its own. While a resume is pending it returns the reveal `poll_chapter` already returns. A pin that matches the awaited offset saves and clears it. A miss does not save, clears the await, and holds that page so a later box does not record the miss. A `{hold}` message does not write. A late pin after the await is already empty does not write. A dropped token is retired and a page or pin in that same body is ignored. Opening is leave, then the resume row. An offset above 0 stays pending. Leave cancels a gesture that has not posted and does not flush. A resize reads the caret. A confirmed miss, or a second timeout, moves the box to the new size and does not walk the saved page number. The draws after that do not cancel a gesture and do not schedule a chase. That loss does not measure the old page number at the new width. The first timeout keeps the old frame and reads the same column once more. A waiting draw does not cancel. The read is the first glyph in the column still on screen, and the scroll returns there. An image post reaches the draw without painting the footer. The element is kept for the next resize and is not stored as a character. A user turn or a new load drops it. Two lookups that miss the element, with no find between them, move the frame and do not write a caret. A find between them clears that count. A late sample from an older read is not the current character. A failed anchor write keeps that character and does not walk the page number. A hit stores that character and reveals it, and a later resize reveals the same character instead of measuring the old column at the new width. That reveal saves the new column. Character 0 is stored on the row. An old row with only a page and offset 0 still walks. While that read is in flight the poll does not chase. A commit that has not posted can be dropped. A commit that has posted is saved. Chapter and return use the reader's place. The footer page is paint. A PDF page stays on its own saved page and offset. An older token does not move the saved page backward. A post with no href still cannot tell an old document's pin from a new chapter at the same offset. That boundary is unchanged. A delayed reveal retires the finger and those tokens, so the lift and a post that already left the page do not replace the saved sentence. Retiring the post does not move the column back. A drop before the queued page finishes stops that page from posting; the reveal then seeks to the sentence. A resize drops it even when the fitted frame and zoom do not change. A key during the release still waits for the committed page. A direct file open does not engage the shelf, and the wheel monitor is installed anyway. Page 1 does not move backward. Page 10 does not move forward. A vertical wheel is not this gesture. A mouse-wheel notch under 48 points does not turn, and 48 points turns once. A backward notch on page 1 does not turn.

Evidence. `spikes/shell/src/visit.rs` runs the gesture the monitor calls, including the release that stays open, the delayed reveal, and a key during a second finger. The page script runs the key, Space, edge, drop, return, reveal, and commit-token sequences. `a_finished_swipe_is_the_place_a_later_open_reads` writes a real temporary `settings.json` through `SettingsBook` and `handle_page`, leaves, and opens again at page 2 offset 50. It also covers a swipe that wins over a pending resume, a chapter jump, a refused token, a late pin, a key or edge (`kind: "turn"`), a `frames` sample that does not save, a box that only reveals, a miss, a swipe body that also carries `missed` and `pinned`, a failed write, and a drop. `a_page_resume_does_not_store_the_columns_it_crosses` resumes page 10 offset 0, including an empty href, through `resume_walk` and `resume_turn`. Each step evals `turn_command` and `handle_page`. Leaving before page 10, and arriving there, both keep page 10 offset 0. A key during the walk saves that page. `a_resume_walk_ends_when_the_reader_moves` continues polling after that key and after a step back from page 10. Neither poll chases. `poll_resume` is that test. A restore during a commit is refused, the remaining column matches the saved page, and a second lift's token is still posted. `a_taken_turn_is_the_page_the_resume_stops_on` walks an unattended page to 10 and keeps 10/0. A commit with an empty queue, a commit that already holds a token, a shell turn, a page key, and a swipe each leave the screen on the saved page, and later polls do not chase. A drop during a restore does not post that restore. A resize drops an animation that has not posted. It does not retire a commit that already posted, and it does not end the walk. A caret miss posts no offset. `layout_frame` is the draw `place_page` applies. After the miss it moves the box, the following draws cancel nothing and schedule nothing, and the walk stays. The file stays page 10 offset 0. A poll does not walk that page number. A later width moves the box and does not measure page 10. A key that those draws do not cancel posts `kind: "turn"`. A sample that arrives after that read was retired is not stored. The file staying put is not the text. A caret miss on the old frame reads the first glyph in that column. At width 400, page 10 is character 360, and the scroll returns to the column that was showing. Width 250 reveals character 360 on page 15. Width 180 reveals it on page 21. A caret that hits is kept. The file does not take page 10 at the new width, and it does not take the start of the text node. While the reveal frames are queued, a poll at a column past the old page does not clear the walk. A key that has not posted returns to the settled column. Columns crossed before the anchor stay page 10 offset 0. A failed anchor write keeps character 360, leaves the file at page 10 offset 0, and the poll does not walk page 10. The same post then saves. Character 0 is the first glyph of the visible column. Its reveal lands on page 1, writes offset 0 with the caret mark, and a new reader opens that character and keeps it on the next resize. A page-only row with offset 0 and no caret mark still walks. A user post marks the page. Reader sets one ack when it accepts or refuses that post. The next restore carries the ack and clears the mark. A restore built before the post does not seek. A refused write leaves the saved page and the walk unchanged, and the following acked restore still seeks. A restore or a drop from an older chapter document does not run. A user key does not carry that document id. Shell left, right, Space, Shift-Space, and both edges, plus the page script's key and edge, each save on their own. `a_second_commit_and_a_long_chapter_post_every_token` feeds the Node driver's own post objects, including `glide`, `queued`, and 128 commits, through that same `handle_page`. `a_finished_page_is_saved_before_chapter_or_resize` still calls `deliver_page`. It is not this wiring. The CI `test` job names `a_trackpad_swipe_follows_cancels_and_commits_once`, `a_key_during_the_release_still_turns`, `a_direct_file_arms_the_wheel_without_the_shelf`, `a_release_stays_open_until_chapter_or_resize_cancels_it`, `a_reveal_during_the_release_does_not_post`, `a_delayed_reveal_does_not_let_the_lift_replace_the_pin`, `a_second_finger_key_keeps_the_committed_page`, `a_commit_post_names_its_token_and_a_second_finger_key_does_not_drop_it`, `a_long_chapter_commit_is_never_dropped`, `a_second_commit_and_a_long_chapter_post_every_token`, `a_finished_page_is_saved_before_chapter_or_resize`, `a_finished_swipe_is_the_place_a_later_open_reads`, and `a_page_resume_does_not_store_the_columns_it_crosses`, `a_resume_walk_ends_when_the_reader_moves`, `a_taken_turn_is_the_page_the_resume_stops_on`, `a_resize_keeps_the_saved_page_until_the_anchor_lands`, and `a_hold_reads_the_column_character`. `a_direct_file_arms_the_wheel_without_the_shelf` runs the route functions. It does not install the monitor or run `sync`. The Release `macos` job compiles, packages, signs, and notarizes. It does not run these tests. WebKit did not run them either. This round did not change how `settings.json` is parsed or how a write reports its own I/O error.

Not run. A Mac window, the trackpad, WebKit painting the follow, `nsh test`, the real library, frames, and animation. A post with no href against a new chapter at the same offset was not given a document generation and was not claimed fixed. The document id was not claimed to cover a user key. WebKit did not run the resize. The node harness ran the column cases: character 360, a caret hit at 368, a blank skipped to 363, a zero rect still 360, page 15 at width 250, page 21 at width 180, the same image after the width change, and a user turn during the reveal. `a_hold_reads_the_column_character` routes the page's img, svg, late probe, pageless figure, two missed aligns after the wait has ended, a find between two misses, user turn, and load through `layout_frame`. That test calls the mail predicate and does not call `place_page`. Clearing the wait and the mailbox in `place_page` was read in source and was not executed. That test was not executed on Linux. WebKit's caret and reflow did not run. The 128-commit canvas script was not re-run. Linux did not compile `spike-shell`.

## S3 blocked turn

Goal. An EPUB turn that asks to leave the first or last page of the open chapter eases back to that same column. The page number and the stored sentence stay.

Scope. The left arrow, the right arrow, Space, Shift-Space, an edge click, and a trackpad release use the page script's existing turn. The open document's column decides the block. The ease is at most 24 CSS pixels and at most 6 percent of the column width, and it uses the 160 ms turn. A contents choice, an in-book link, a return, a resize, leaving the book, and opening another book cancel that ease. A turn that still has a column to enter plays the existing slide.

Non-goals. Checklist item 3 is not done. S3 is not done. No new control, no PDF fit-page, no margin, no S4, and no change to the return place or a pending fragment. The page script has no reduce-motion switch, and this slice does not add one. `nsh` stays at 0.5.1.

Done. On a 400 CSS pixel column, a backward key, Shift-Space, a left-edge click, and a blocked trackpad release at page 1 move the column to `translate3d(24px,0,0)` at 80 ms and back to `none` at 160 ms. A trackpad lift past 48 points at the first or last page returns `Ease` with that direction and token 0. A lift under 48 points, a cancelled finger, and a mouse-wheel notch at that edge stay `Cancel` and pass delta 0. Scroll stays 0. A forward key, Space, and a right-edge click on that page seek to 400 and do not ease. The last page eases the other way, `translate3d(-24px,0,0)`, and scroll stays on that column. A one-column chapter eases in both directions. A second blocked key does not queue a slide. A forward key during the ease seeks once and the old frame does not paint again. An acked restore during the ease seeks once and the old frame does not paint again. `spikeFollowDrop` and `spikeReveal` clear the transform, and a later frame stays clear. A new `__spikeLoad` makes the old frame return. The footer and the document element do not take that transform.

Evidence. `a_blocked_page_eases_back_to_the_same_column` runs the production page script under Node's clock. `a_blocked_ease_leaves_the_stored_sentence` writes a temporary `settings.json` through `SettingsBook` and `handle_page`. The blocked `kind: "turn"` leaves page 4 offset 800, the return place, the pending fragment, and the gesture token. A contents choice of the next spine item still returns `Chapter`. The CI `test` job names both tests. The Release `macos` job compiles, packages, signs, and notarizes. It does not run them.

Not run. A Mac window, WebKit painting the ease, `nsh test`, and the feel of the motion. Linux did not compile `spike-shell`.

## S3 reading chrome

Goal. On the shell reading window, the traffic lights and the book title appear together when the pointer reaches the top, hold, then hide, without a flicker and without a key or a gesture showing them.

Scope. One alpha, written by `step_chrome` from the pointer band `near_top` already uses (56 points) and from whether the window is key. The shell reading path in `sync` paints that alpha. The status `title_opacity` copies it, and the shell title row does not write a second opacity. Opening a book sets the reading identity before the first status and cuts the fade. A publish key whose width, height, or bar changed clears the pending delay on the next pump. A key that changes only because the lights crossed half alpha still publishes and leaves that delay alone. The library, `SPIKE_LIGHTS`, and a product window that is not reading keep the lights on. The dev spike still fades toward the pointer on its own path. Button frames stay `align_traffic_centers` and `measure_traffic_bar`. Fullscreen reparent stays `9 + index * 22` at `chrome_center`. NSH stays 0.5.1. The AppKit toolbar stays off.

Non-goals. Checklist item 8 is not done. No new control, no layout token change, no S3 margin, no S4, no PDF fit-page, and no engine change.

Done. From hidden, four pointer steps reach alpha 1. A 16 ms exit and a return stay at 1, with the delay cleared. A 399 ms exit stays at 1, and the next millisecond steps to 0.75, then to 0. Ten quick exits and returns stay at 1. Twenty steps with the pointer away stay at 0, which is the keyboard, Space, trackpad, and edge case: those inputs are not on this clock. Losing focus goes to 0 and clears the delay. After the 400 ms hold, four pumps go 1, 0.75, 0.5, 0.25, 0. The pump that reaches 0.5 publishes a lights-only key and does not bump the generation, so the next pump continues. A wider window does bump it, and the next pump keeps alpha 1 with the delay restarted at 50 ms. A book cut goes to 0, and a cut with the pointer at the top starts at 0.25. The shelf's first status, with the surface already Reading and the pump not yet run, publishes title opacity 0 and the shared flag. Leaving the reading flag false publishes the open-time title at 1.

Evidence. `reading_chrome_follows_the_pointer_and_drops_a_stale_hide` runs that pump order through `step_chrome`, `light_alpha_shown`, `lights_shown`, `shell_publish_key`, and `publish_key_effect`, then the next step. It also runs `reader_shown` and `enter_reading_chrome` for the shelf's first status. The CI `test` job names that test. The Release `macos` job compiles, packages, signs, and notarizes. It does not run the test.

Not run. A Mac window, the pointer, the fade on screen, WebKit, `nsh test`, and the feel of the hold. `reading-chrome-entry-title` and `reading-chrome-visual` stay `blocked` without a renderer read and without a pixel score. `scripts/verify --self-test` scores a sample log and does not launch the app. `just verify --subset reading-chrome` is the one-session console run and was not run. Linux ran `verify_session` with `rustc --test`. `cargo test -p spike-shell` did not compile here: `objc2` only works on Apple platforms, so the named shell test was not executed on Linux.

## S3 link

Goal. A primary click on an in-book link lands on that text. The sentence before the click can be returned to while the book stays open. A second return stays. The same route covers a footnote whose href stays inside the book.

Scope. The page posts `kind: "link"` with the raw href, the document load, and, when the target is in the open document, the character offset of its first locatable character. `handle_link_message` percent-decodes that href, resolves it against the spine, and calls `handle_chapter`. A fragment in another spine item is not written yet. The view href moves to that chapter before the sentence does. That view is not a sentence and is not the return. That document posts the fragment, its own load, and the measured character. A page message during that wait does not write and does not drop the original sentence. A click that cannot move, including an unmeasured fragment, leaves that wait in place. A layout or turn from a document that is neither the stored sentence nor the jump does not write. A contents choice for a chapter that is not on screen loads that chapter. It does not reveal on the document that is showing. The stored sentence stays the return. The file stores the destination sentence through `SettingsBook`. `Reader.back` is the return place for this session. Reopen reads the destination sentence and has no return place.

Non-goals. The return place is not written. Reopen keeps the destination sentence and has no return stack. No footnote popup. No external open for `http`, `https`, `javascript`, or `mailto`. A path that leaves the book, an unknown spine item, or a missing fragment does not change the stored place. `#` is not a spine item. PDF links, the margin, S4, and a new control stay out. Checklist item 4 is not done. S3 is not done. A bookmark and a search result stay out. The blocked-turn ease-back is the S3 blocked turn slice. This slice does not draw it.

Evidence. `a_book_link_lands_on_the_sentence_and_returns` writes `settings.json` through `handle_link_message`, `handle_page`, and `handle_chapter`, then opens that file again. `a_pending_fragment_keeps_the_origin_sentence` keeps that file on the original sentence when a `kind: "turn"` arrives before the fragment is measured. The second click, the contents choice of another chapter, and the contents choice of the chapter already on screen pass that chapter's href as the shown address. The file and the one return stay the original sentence until the new sentence is written. It also covers a percent-encoded spine href and an encoded path that leaves the book. `a_page_link_click_posts_the_character_offset` fires the page click. The CI `test` job runs them. The Release `macos` job compiles, packages, signs, and notarizes. It does not run them.

Not run. A Mac window, WebKit, `nsh test`, and a real EPUB click. The node harness accepts a sized character that sits outside the column and above the page, and it rejects a character whose ancestor is `visibility: hidden`. WebKit did not run that rule.
