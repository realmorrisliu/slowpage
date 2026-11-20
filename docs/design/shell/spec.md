# App shell

This is the layout reference for the Slowpage window. Morris approved it on 2026-11-11. Every later shell change builds this form. The sequence of slices is in `docs/roadmap.md`.

The library form's right region is the chat panel. The reading form's right region is the annotation margin. Those are different regions.

## Pictures

![Approved library form, version B](approved-full-form.png)

`approved-full-form.png` is the approved library form. Morris picked this text-row sidebar and rejected the icon rail.

![Rejected icon rail](rejected-icon-rail.png)

`rejected-icon-rail.png` is the rejected alternative. Do not build it.

![Region sketch, full form](sketch-full.png)

`sketch-full.png` shows the full form's regions. The left column is the sidebar. The middle is the card. The right column is the chat panel.

![Region sketch, page and panel](sketch-page-panel.png)

`sketch-page-panel.png` shows the reading form's regions. The middle is the page. The right column is the place for the annotation margin. The sketch does not approve what goes inside that column.

![Region sketch, page only](sketch-page.png)

`sketch-page.png` shows immersive reading. The card meets the window edges.

![Floating card reference](floating-card-ref.png)

`floating-card-ref.png` is the floating-card reference. A light gray field holds one white card. No rule separates the side column from the card.

The older shots in this folder, `bar-window.png`, `bar-lights.png`, `bar-fullscreen.png`, and `bar-window-main.png`, record the 2026-11-10 bar. They are not the approved shell.

## Window

The base is a light gray field in the macOS vibrancy style. The gray has no approved hex value. Match the field in `floating-card-ref.png` and in `approved-full-form.png`.

The shell draws no divider between the sidebar and the card, between the card and the right region, or across those boundaries. A hairline there fails the shell.

## Card

One white card fills the area between the sidebar and the right region. The card is `#ffffff`. The corner radius is 12 points. The card is inset 6 points from the base on every side, including a side that meets the window because the sidebar or the right region is hidden. A soft shadow sits under the card. The shadow has no approved blur radius. Match the approved picture.

Immersive reading sets the inset to 0 and the corner radius to 0. The card meets every window edge. The gray field is not visible, including at the corners. Immersive reading draws no shadow.

The page, when a book is open, is the existing reader inside the card. The 6 point inset is the gap between the card and the base. The shell window stays the three-form window. S3 does not resize that window to the page ratio.

The page box keeps its trim, the footer on the text measure, and the absence of a divider above the footer. The 52 point side mats, the 48 point top mat, and the 88 percent window fit belong to the 2026-11-04 window, recorded in `docs/prototypes/window-chrome/README.md`. They are not the card inset.

## Forms

| Form | When | Sidebar | Card | Right region | Inset |
| --- | --- | --- | --- | --- | --- |
| Full | Launch, and any time no book is open | Shown | Library | Chat panel | 6 points |
| Page and panel | A book is open | Hidden | The page | Annotation margin | 6 points |
| Immersive | Immersive reading is on | Hidden | The page | Hidden | 0 |

Full is the launch form. The chat panel is open.

Page and panel is the reading form. Opening a book hides the sidebar and replaces the chat panel with the annotation margin. The page uses the reader the app already has.

Immersive reading hides the sidebar, the chat panel, and the annotation margin. Traffic lights still fade in at the top when the pointer is there. That fade is the reading-window rule in `docs/design/north-star.md`. The gray field stays hidden.

## Chat panel

The chat panel exists only in Full. It is 320 points wide and sits on the gray base, outside the card. It is open at launch.

The header shows an avatar, the name **Slowpage**, and the subtitle **你的阅读助手**. The body is the conversation. The bottom is a rounded field. The placeholder is **问问 Slowpage…**.

Shell slices build this frame and the empty state. They do not embed alan. The 2026-11-17 embed decision is in `docs/design/north-star.md`. Integration starts after S3 and S4.

Hiding the chat panel widens the card. The card stays inset 6 points from the base. Hiding the chat panel does not enter immersive reading.

## Annotation margin

The annotation margin exists only in Page and panel. It sits on the gray base, outside the card, in the reading form's right region. The chat panel is the library form's right region.

A highlight, a note, and an agent response each appear as a margin note. The note is anchored to the line it refers to and is vertically aligned with that line. The notes move when the page scrolls, and they move when the page turns. Asking the agent about a selection adds one anchored margin note.

Morris set the margin's contents on 2026-11-10. The same words are the section Annotation margin (hard requirement, 2026-11-10) in `docs/design/north-star.md`.

> The right panel is an annotation stream aligned with the text, not a chat interface.
>
> - Each annotation is a small card, vertically aligned with the line it highlights (like Google Docs or Notion comment margins). When cards crowd together they flow downward; when one is selected it snaps to its line in the text.
> - @alan's replies are a thread under that annotation, so the conversation hangs on the original passage instead of becoming a separate chat window detached from the book.
> - A single input box stays at the bottom of the panel for questions about the whole book or the current page that don't point to one sentence. Those conversations attach to "current page" and move with page turns.
> - Cards stay collapsed to one or two lines by default, showing only the annotator and the first sentence, and expand on hover or select. The panel should look like marginalia on a book page, not an IM.

Picked on 2026-11-10, in pull request 14.

- Annotation cards use the light card style.
- Annotation text is 13 to 14 points.
- The card-to-line connector has three states. At rest there is no connector. On hover the connector is faded. When the card is selected the connector is full.
- In a narrow window the margin becomes a drawer. Its cards stay aligned to their lines. When a card cannot stay at its line, the drawer scrolls that card to its line height.

Approved on 2026-11-20 at 15:49 Asia/Shanghai. Mochi conveyed Morris's choice after Morris looked at three design drafts. This paragraph records that conveyance.

- A wide window shows the annotation margin at 240 points. The chat panel's 320 points stay unused for this margin.
- A narrow window shows the margin as a drawer. The 16:03 approval states what that drawer does.
- The control that shows or hides the margin is the existing panel toggle. It does not open the chat panel, and it does not enter immersive reading. Its two seats are the 16:03 approval.

That choice approves these three items. S3, S4, and checklist item 10 stay open.

Approved on 2026-11-20 at 16:03 Asia/Shanghai. Mochi conveyed Morris's choice after Morris looked at the open drawer, the threshold, and the state frames. This paragraph records that conveyance.

- Below 818 points the open margin covers the right side. The page keeps the box it has when the margin is hidden. There is no mask. A click on the page does not close the margin. The margin does not take focus. The same control closes it.
- 818 points is the 578 point window minimum plus the 240 point margin. Crossing that width keeps the margin shown or hidden. At and above 818, a shown margin is the 240 point column and the page fits the card beside it. Below 818, showing or hiding the margin does not move or resize the page. A hidden margin has no column and no cover.
- Hidden, the control is at the top right of the window. Shown, the control is the header of the margin, at that same top right. The window does not show a second control.
- The control stays at full opacity when the lights and the book title fade. Those two keep their one alpha.
- During the 160 ms slide the footer stays on the departing page. It changes when the slide ends. A slide that is cancelled, or a book that changes, does not write that turn onto the book now open. A trackpad finger moves the current page by that gesture's points, clamped to the fitted page. The lift glides from that offset over the existing 160 ms, onward if it commits and back if it does not. A PDF does not rubber-band past the first or last page. Checklist item 3 is not done.
- A click in the left or right quarter of the page that is still visible turns one page. A click on the cover stays on the page.
- Escape stays on the path in Commands. The margin does not take it.

The 2026-11-10 list above stays that date's record. Lining a card up with its line, and scrolling a card to that line, stay in that list.

Mochi conveyed Morris's 2026-11-20 22:42 Asia/Shanghai widget answer: implement this set's card layout, connector, crowding, independent scroll, and page-turn update. The recording interface did not keep the widget. This paragraph records that conveyance. It does not claim the original widget log was read.

- The column stays 240 points. A card is 208 points wide, inset 16 points from each side, with an 8 point gap and a 12 point radius. The card is white with the existing 0.5 point ring. Annotation text stays 13 to 14 points.
- At rest a card is collapsed to the annotator and the first sentence, and it has no connector. Hover or selection expands it. The anchor is the first line of the highlight. The connector starts at that line's highlight end, or after the last glyph on that same line when unhighlighted words continue, and runs to the card's left edge. It does not use a later line. A hovered connector is the same 1 point ink at 0.28 alpha. A selected connector is solid. A selected card parked off its line uses that same ink as a short elbow.
- Crowded cards move down and keep the 8 point gap. A selected card returns to its line. Cards above it that would overlap move up.
- The card area scrolls on its own. Its top is the bottom of the 48 point header. Its bottom is the top of the input. The input sits outside that area, inset 18, radius 14, height 88, and it does not paint over card text. Scrolling reaches the body and the replies of a card taller than the area. Scrolling away from the anchor hides the connector. Scrolling back restores it. A line in the header parks the card at the top of the area. A line in the input band parks the card 8 points above the input.
- During a page turn or the immersive slide the cards stay on the departing page. They change together when the slide lands.

The local store is `annotations.sqlite` in the support directory. A card draws when that page has a stored note and a line. An EPUB note stays on the spine item it was taken from, so the same column number in another chapter does not show it. Its card is placed from a fresh measure of the stored scalar range in the current layout. A stored viewport box is not scaled into a new layout. A change of the webview frame, including its height or page zoom, runs the page reflow and posts that measure. The window height is not a separate layout. A message measured on the previous frame, zoom, or document is not drawn. Until a post for the frame now on screen arrives, the previous line boxes are not drawn. Return in the existing bottom input saves that note locally and clears the field. A second Return with no new typing does not write again. Typing the same note again does. A failed save keeps the draft and uses the existing notice line. The selection expires when the page, chapter, or layout changes, or while the page is turning. The draft stays. The next Return says 先选中原文, because the passage is already cleared. That line is not shown at the moment of leaving. A notes table that has no spine column gains one in place. Existing rows and replies stay, and the file is not replaced. Editing a card and replying in the margin are not in this slice, and the shell does not call an agent. S3, S4, and checklist item 10 stay open.

A shell change does not invent an unset item. Pull request 29's connector is not this approval.

## Commands

Launch opens Full. The sidebar is shown and the chat panel is open.

Opening a book from a cover or from **继续阅读** enters Page and panel. The sidebar hides. The chat panel is gone. The annotation margin is shown. The card shows the page at the 6 point inset.

Choosing **全部书籍** closes the book and returns to Full. The sidebar is shown, the chat panel is open, and the card shows the library.

Control-Command-I, written ⌃⌘I, the View menu item, and the window button toggle immersive reading while a book is open. They enter it when it is off, and they leave it when it is on. With no book open, the shortcut and the menu do nothing, and the button is not on screen. The button is a 28 point control, one slot to the left of the panel toggle. The pictures drew that toggle 6 points from the window edge. The live toggle's right inset is 16 points, so the button keeps a 6 point gap from the live toggle and does not move it. On a 1440 point window with the chrome top at 0, the button's left edge is 1362 and its top is 10, the same top as the panel toggle. The glyph is the IconMCP editor maximize outline at 16 points. The gray plate is the existing row wash at 0.07, and it is drawn only when the card has landed on the immersive frame. The seat does not move. `docs/nsh.md` keeps the toolbar off, and checklist item 8 says this row is not an AppKit toolbar, so the shell does not install that toolbar.

The card moves over 160 ms on the page glide's cosine, `(1 - cos(π · t / 160)) / 2`. The inset goes from 6 points to the window edge. The radius goes from 12 to 0. The native card fades the existing 0.5 point spread at 0.07 alpha, and that spread is gone at the immersive end. The two blur layers stay on the library card. A wide margin retreats with the card. A narrow margin slides off the right, and the page box does not shrink. The page keeps the frame and origin it left, and it refits once when the motion lands. A resize, fullscreen, or a cross of 818 during the motion keeps that frame and the layout key. The page refits when the motion lands. A return that finds the margin preference changed keeps the card on the rectangle it has reached, and the page keeps that departing frame until the return lands. The PDF bitmap is fitted to that frame and clipped to the moving card. The EPUB webview stays on that frame until the motion lands, so WebKit refits once. The webview is not masked to the shrinking card. The motion does not turn the page. A second toggle, or Escape, returns from the current rectangle. The return lasts the remaining distance times 160 ms, and it is never longer than 160 ms. Asking for the goal already in progress does not restart the clock. The command drops a held finger, a flying page, and the one queued page, and it does not write. A click or a swipe during the motion does not turn a page and does not write. An EPUB wheel uses that same gate. `spikeFollowEnd` answers `no` while the trip flag is set, and a lift already in flight does not seek after that flag clears. A key still turns. S4 is not done. The Mac window, `nsh test`, WebKit, and a real library were not run.

Escape leaves immersive reading and does not enter it. Leaving returns to Page and panel. The sidebar stays hidden and the annotation margin is shown. The 6 point inset returns when the card lands. While the card is still away from Page and panel, Escape keeps the book. After the card has landed there, Escape, Command-[, and the back button leave the book and return to Full. Full is the library card. That check is in `docs/design/reading-interactions.md`.

The sidebar toggle in Full hides or shows the sidebar. The card keeps the library. The chat panel does not change. The inset stays 6 points. Hiding the sidebar does not open a book.

The sidebar toggle in Page and panel hides or shows the sidebar. The page stays open. The annotation margin does not change. Showing the sidebar does not open the chat panel.

The sidebar toggle is absent in immersive reading. Its menu item does not leave immersive reading.

The panel toggle in Full hides or shows the chat panel.

The panel toggle in Page and panel hides or shows the annotation margin. It does not show the chat panel, and it does not enter immersive reading. The inset stays 6 points. Hidden, the control is at the top right of the window. Shown, the control is the header of the margin, at that same top right, and the window does not repeat it. That pair of seats is the 16:03 approval in the annotation margin section.

The panel toggle leaves with the margin. It is absent once the card has landed in immersive reading.

When the sidebar is shown, its toggle sits on the sidebar's top row, and the toolbar does not repeat it. When the sidebar is hidden and immersive reading is off, that toggle is the toolbar contents control. Insert it only after the traffic lights are shown, 26 points after the zoom button, centered on that button. Inserting it while the lights are faded parks it on the minimize button.

When the chat panel is hidden in Full, its toggle sits in the toolbar. When the panel is shown, the toggle sits on the panel header, and the toolbar does not repeat it.

## Sidebar

The sidebar is version B. It is 220 points wide. The width does not follow the window size. The rows are text rows. The rejected picture's icon rail is not a collapse state of this sidebar.

The top row holds the traffic lights and the sidebar toggle. The 2026-11-10 centering measurement sizes that row's left inset. The 220 point sidebar width stays fixed.

The inset is the zoom button's right edge plus the close button's left edge, after AppKit has converted those frames to window coordinates. The left gap is the close button's left edge. The right gap is the inset width minus the zoom button's right edge. Those two gaps differ by at most 0.5 points. Read the buttons after the title bar has laid them out. Measure again when the window is created, when the toolbar style changes, when immersive reading starts or ends, and when the window enters or leaves full screen. A resize does not change the inset. The toggle sits in the remaining part of the 220 point row.

Under that row is a search field. The placeholder is **搜索书名、作者、笔记**.

Then these groups, in this order. Each group label is small and gray.

- **书库** holds **全部书籍**, **正在读**, **想读**, **已读完**, and **PDF**.
- **笔记** holds **高亮与笔记**.
- **书单** holds the reader's lists. Each list has a colored dot. The three lists in the approved picture are examples, not a seeded catalog. An empty group shows the label and no rows.
- **来源** holds connected sources only, as peers. None is primary. The peers are Apple Books, calibre, and each connected folder. A folder row's label is the folder path. Each connected source shows a count. A source that is not connected is omitted. The last row is **+ 添加来源**, and it is always shown.

**设置** is pinned to the bottom of the sidebar, under the groups.

The selected row has a soft gray rounded highlight. Counts are right-aligned. **+ 添加来源** and **设置** have no count. These counts are the shell's row counts. They are separate from the calibre facet counts in the Library section of `docs/roadmap.md`.

Icons on the rows come from IconMCP. The parameters are outline, stroke 1.25, and radius 2. A selected row uses the solid variant. The colored dots on **书单** are dots, not icons.

## Library card

The card header shows the selected sidebar title, the count of books in that selection, a sort control, and a view control. The default sort is **最近阅读**. The view S2 ships is the cover grid. No other view is specified.

**继续阅读** sits under the header and above the grid. Each item shows the cover, the title, and the position the reader has already stored for that book. The row lists books the reader has opened. A book with no stored position does not get a fabricated percent.

The grid uses a cover that is 2 by 3. Under each cover, the title and the author are small and gray. A PDF cover shows a PDF mark, as in the approved picture. The books are the ready rows of `Catalog` in `spikes/library`. S2 reads `Book::title`, `Book::author`, and `Book::cover`. S2 does not add calibre write support.

An empty `Catalog` still uses this card. The card offers Apple Books, calibre, and adding a folder as peers. None is a primary button. That empty state replaces the centered shelf buttons.

Choosing **设置** shows settings in the card and stays in Full. The settings page is outside this spec.

## Icons

IconMCP is the only icon library. Outline, stroke 1.25, and radius 2. Solid is for a selected or active state only. The shell rasterizes each SVG at runtime with resvg, as `docs/design/north-star.md` describes.

The sidebar toggle uses the list glyph already shipped for the contents control.

## What S1 removes

S1 removes three temporary paths. It does not repair them.

- The Open button on an empty window.
- The `open -a` and Finder Open With checks. S1 does not make those checks pass.
- The bare shelf screen in `spikes/shell/src/onboarding.rs`. That screen is a white window with **Drop a book to open it**, **Import from Apple Books**, **calibre library**, and **Folder** centered in it.

`spikes/library` stays. `Catalog` is the store S2 draws. The checks in `.cursor/skills/verify-superreader/features/library-shelf.md` describe the bare shelf. S1 deletes those checks with the screen. It does not retarget them at the new card.

## Slices

Each slice is a thin cut of this form. The order is fixed.

1. S1 builds the gray base, the 220 point sidebar, the white card, and the chat panel frame, including empty states. It replaces the bare shelf window and removes the three paths above.
2. S2 fills the card with the cover grid and **继续阅读**, from `Catalog`.
3. S3 builds Page and panel with the existing reader. The sidebar hides, the page is in the card, and the right region is the annotation margin. S3 waits on the items the annotation margin section still marks unset. The interaction checks are in `docs/design/reading-interactions.md`.
4. S4 builds immersive reading.
5. S5 polishes motion. Night mode and a theme switch stay out, under the 2026-11-03 rule. Light and dark join S5 only if a later decision revises that rule.
