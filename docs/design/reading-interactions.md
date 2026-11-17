# Reading interactions

This is the interaction rule for the reading form. Morris confirmed the rule, the checklist, and the performance floors on 2026-11-17. The reading form is shell slices S3 and S4 in `docs/design/shell/spec.md`. S3, S4, and every slice after them follow this file.

## Rule

S3, S4, and every slice after them get every interaction detail right. The interaction has to work while the reader uses it.

Each item in the checklist needs both proofs before that item merges.

- An automated interaction test. Headless `nsh test` drives the input and asserts the resulting state.
- A verify run on a Mac. The proof is a screen recording, or a run of consecutive frames. One screenshot does not prove the item. The capture has no blank frame, no flicker, no position jump, and no clipping.

Every item runs once for an EPUB and once for a PDF.

One item, or a small group of items, is one pull request. The pull request merges after both proofs pass.

Add a new item to this list, with Morris's approval, before building that item.

## Checklist

The ten items are these.

1. Shelf to reading and back. One click on a cover enters the reading form. While immersive reading is off, the back button, Command-[, and Escape leave the book and return to the shelf. While immersive reading is on, Escape leaves immersive reading and stays on the book. The enter and leave transition is animated. Morris picks that animation from two or three recorded prototypes before anyone builds it. The shelf keeps its scroll position on return. **继续阅读** updates immediately.
2. Finder and Dock open. Opening a file on a cold start, opening a file while Slowpage is already running, and dropping a file on the Dock icon each opens that book directly. Opening a file while another book is open switches to the new book. That switch is the fix for the alpha-1 bug, where a second PDF opened on a blank page. An unsupported file or a broken file shows a clear message.
3. Page turns. The left arrow and the right arrow turn one page. Space moves forward, and Shift-Space moves back. A horizontal trackpad swipe turns one page. The animation follows the finger, and the reader can cancel that turn before it completes. A click on the left edge or the right edge turns one page. A turn blocked at the first page or the last page gives feedback. The footer page number matches the page on screen. The arrow keys, Space, Shift-Space, and an edge click play the 160 ms slide recorded in the PDF reading section of `docs/design/north-star.md`.
4. Chapter and location jumps. A table-of-contents click, a bookmark, a search result, an in-book link, and a footnote each lands on that exact location. The reader can return to the location from before the jump.
5. Progress memory. The position is saved immediately after each page turn. Closing the book, quitting Slowpage, and a force-quit followed by a reopen all return to the same sentence. A reopen on the right page and the wrong sentence fails this item.
6. Window resize. An EPUB reflows live while the reader drags the window. The sentence being read stays on screen. Further resizes stay on that sentence. A PDF page keeps its aspect and scales uniformly inside the white card, per the PDF reading section of `docs/design/north-star.md`. Nothing clips at the minimum window. That bar is the adaptive layout rule in [pull request 53](https://github.com/realmorrisliu/slowpage/pull/53). This checklist does not copy that rule.
7. Full screen and immersive reading. System full screen and immersive reading work in any combination. Control-Command-I, the View menu item, and the toolbar button enter immersive reading. Escape leaves immersive reading. The reading position stays the same across every switch.
8. Traffic lights and toolbar. The traffic lights and the toolbar appear when the pointer reaches the top. After a delay they hide, with no flicker. A keyboard page turn leaves them hidden. The traffic-light position and the sidebar centering measurement stay as written in `docs/design/north-star.md` and in the Sidebar section of `docs/design/shell/spec.md`.
9. Rapid repeated input. The cases are a held arrow key and a turn that starts during an animation. They also include repeated table-of-contents clicks, entering and leaving a book in quick succession, and resizing during a turn. The process keeps running. The window keeps taking input. The position stays the one those inputs asked for. The book ends on the correct page.
10. EPUB and PDF. A PDF is paged by the PDF reading section of `docs/design/north-star.md`. The whole page fits inside the white card at a uniform scale. Continuous scroll is out. Reflow is out. PDF page turns match the EPUB page turns in this checklist. Items 1 through 9 pass for an EPUB and pass again for a PDF.

## Performance floors

The page-turn animation holds 60 fps on the Mac. The consecutive-frame capture for that animation shows no dropped frame.

The time from a cover click to visible body text stays under 300 ms. That interval is separate from the open `page_ready_ms` bar in `docs/roadmap.md`.

## Out of scope

Text selection, highlights, and notes belong to the S3 annotation margin. That margin has its own mockup and its own checklist. This file does not specify them.

## Real Apple Books library

Once the reading form has landed, run one read-only round on a real Apple Books library.

Copy the Apple Books library to a temporary directory. Put Slowpage's data in a temporary `SUPERREADER_SUPPORT_DIR`.

Screenshots and logs from that round can contain real book titles. Keep those files on the Mac. Never commit them.

`docs/library/apple-books.md` describes the catalog snapshot. This round is the reading pass on that snapshot.
