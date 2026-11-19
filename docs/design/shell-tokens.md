# Full-form tokens

The approved B mockup stylesheet is the source for these numbers. `spikes/shell/src/tokens.rs` is the only place the shell reads them. The window in that stylesheet is 1440 by 900 points.

The card in the stylesheet starts at the sidebar's right edge. The shell keeps a 6 point inset on every side of the card. `shell-card-inset` measures that gap, and `b-rows.png` shows the stylesheet's flush card. The inset stays.

## Window

| CSS | Value | Rust |
| --- | --- | --- |
| `html, body` width / height | 1440px, 900px | `WINDOW_W`, `WINDOW_H` |
| `body` font-family | system stack, SF Pro and PingFang SC | `FONT_STACK` |
| `body` font-size, color, tracking | 13px, `#1d1d1f`, -0.005em | `BODY_SIZE`, `INK`, `BODY_TRACK_EM` |
| `.win` background | `#EEEEF0` to `#E9E9EC` | `FIELD_TOP`, `FIELD_BOTTOM` |
| `.side` width | 220px | `SIDEBAR_W` |
| `.agent` width | 320px | `CHAT_W` |
| `.card` top / bottom | 6px | `INSET` |
| `.card` radius | 12px | `CARD_RADIUS` |
| `.card` fill | `#fff` | `CARD` |
| `.card` shadow | 0.5px ring, 1px/2px, 6px/20px | `card_shadows` |

## Sidebar

| CSS | Value | Rust |
| --- | --- | --- |
| `.side` padding | 52px 10px 12px | `SIDE_PAD_TOP` (stylesheet only), `SIDE_PAD_X`, `SIDE_PAD_BOTTOM` is `WINDOW_CORNER` (18) |
| `.side-top` height | 52px in the stylesheet | `TOP_ROW_H` is `CHROME_H`, 48 |
| `.tb` | 28px, radius 7px | `TOGGLE_SIZE`, `TOGGLE_RADIUS` |
| sidebar glyph | 18px, IconMCP `sidebar-left` | `SIDEBAR_ICON` |
| `.sfield` | height 28, radius 8, pad 8, gap 6, 12.5px, `#8E8E93` | `SEARCH_H`, `SEARCH_RADIUS`, `SEARCH_PAD_X`, `SEARCH_GAP`, `SEARCH_SIZE`, `QUIET` |
| `.sfield` fill | `rgba(0,0,0,.05)` | `SEARCH_ALPHA` |
| `.slabel` | 11px / 600, tracking 0.04em, margin 14 0 4 | `LABEL_SIZE`, `LABEL_WEIGHT`, `LABEL_TRACK_EM`, `LABEL_MT`, `LABEL_MB` |
| `.row` | height 28, radius 7, pad 8/10, gap 8, 13px | `ROW_H`, `ROW_RADIUS`, `ROW_PAD_L`, `ROW_PAD_R`, `ROW_GAP`, `BODY_SIZE` |
| `.row.on` | `rgba(0,0,0,.07)`, label 600 | `ROW_ON_ALPHA`, `ROW_WEIGHT_ON` |
| `.row .rn` | 12px `#8E8E93` | `COUNT_ROW_SIZE`, `QUIET` |
| `.row.add` | `#8E8E93`, plus 14px | `QUIET`, `ADD_ICON` |
| `.dot` | 9px | `DOT` |
| `.srcic` | 16px, radius 4 | `SOURCE_BADGE`, `SOURCE_BADGE_RADIUS` |
| Apple Books badge | `#FFA53D` to `#F27A1A` | `APPLE_FROM`, `APPLE_TO` |
| calibre badge | `#6B5A4A` to `#4A3C30` | `CALIBRE_FROM`, `CALIBRE_TO` |
| `.side-foot` | height 28, gap 8, 13px `#6E6E73` | `FOOT_H`, `FOOT_GAP`, `BODY_SIZE`, `ICON` |
| settings glyph | IconMCP `sliders` | row icon `sliders` |

An empty **书单** group still shows its label. The headless check `shell-sidebar` reads that label. The fixture fills the three lists from the mockup when `SUPERREADER_FORM_FIXTURE=1`.

## Card header and shelf

| CSS | Value | Rust |
| --- | --- | --- |
| `.card-tb` | height 40 in the stylesheet, pad 12, gap 6 | `TOOLBAR_H` is `CHROME_H - 2 * INSET` (36), `TOOLBAR_PX`, `TOOLBAR_GAP` |
| `.ctl` | height 28, pad 8, gap 5, 12.5px `#6E6E73` | `TOOL_H`, `TOOL_PX`, `TOOL_GAP`, `TOOL_SIZE`, `ICON` |
| sort / chevron | 15px, 11px | `SORT_ICON`, `CHEVRON_ICON` |
| `.vseg` | pad 2, radius 8, button 28 by 22, radius 6 | `SEG_PAD`, `SEG_RADIUS`, `SEG_BTN_W`, `SEG_BTN_H`, `SEG_BTN_RADIUS` |
| `.vseg` track | `rgba(0,0,0,.05)` | `SEG_ALPHA` |
| `.vseg .on` shadow | 0.5px ring and 1px/2px | `seg_shadows` |
| `.card-body` padding | 4px 40px 0 | `CARD_PAD_TOP`, `CARD_PAD_X` |
| `.title-row` | height 36, gap 10 | `TITLE_ROW_H`, `TITLE_GAP` |
| `h1` | 26px / 700, tracking -0.01em | `H1_SIZE`, `H1_WEIGHT`, `H1_TRACK_EM` |
| `.h1-sub` | 13px / 400 `#8E8E93` | `COUNT_SIZE`, `COUNT_WEIGHT`, `QUIET` |
| `.sec` | 15px / 600, margin 28 0 12, gap 8 | `SECTION_SIZE`, `SECTION_WEIGHT`, `SECTION_MT`, `SECTION_MB`, `SECTION_GAP` |
| `.sec-n` | 12px / 400 `#8E8E93` | `SECTION_NOTE_SIZE`, `BODY_WEIGHT`, `QUIET` |
| `.cr-row` gap | 16px, at most 3 cards | `CONTINUE_GAP`, `CONTINUE_MAX` |
| `.cr` | pad 12, radius 12, gap 14, `#F5F5F7` | `CONTINUE_PAD`, `CONTINUE_RADIUS`, `CONTINUE_INNER_GAP`, `CONTINUE_BG` |
| continue cover | 52 by 78 | `CONTINUE_COVER_W`, `CONTINUE_COVER_H` |
| `.cr-t` | 13px / 600 | `CONTINUE_TITLE_SIZE`, `CONTINUE_TITLE_WEIGHT` |
| `.cr-a` | 12px `#8E8E93`, margin-top 2 | `CONTINUE_META_SIZE`, `QUIET`, `CONTINUE_META_MT` |
| `.bar` | height 3, radius 2, track `rgba(0,0,0,.08)`, fill `#1d1d1f` | `BAR_H`, `BAR_RADIUS`, `BAR_TRACK_ALPHA`, `INK` |
| `.cr-p` | 11px, gap 8 | `PROGRESS_SIZE`, `BAR_GAP` |
| `.cr-w` | 11px `#A1A1A6` | `TIME_SIZE`, `FAINT` |
| `.grid` row gap | 28px | `SHELF_ROW_GAP` |
| shelf cover | 128 by 192, radius 3 | `SHELF_COVER_W`, `SHELF_COVER_H`, `SHELF_RADIUS` |
| column count | continuous rule; 5 at content width 808 | `shelf_grid` |
| cover shadow | 0.5px ring, 1px/2px, 6px/14px spread -4 | `cover_shadows` |
| `.b-t` | 12.5px / 500, margin-top 10 | `SHELF_TITLE_SIZE`, `SHELF_TITLE_WEIGHT`, `SHELF_TITLE_MT` |
| `.b-a` | 11.5px `#8E8E93`, gap 6, margin-top 1 | `SHELF_META_SIZE`, `QUIET`, `SHELF_META_GAP`, `SHELF_META_MT` |
| `.pct` | 11px `#8E8E93`, on the author line | `PROGRESS_SIZE` |
| `.tag` | 9px / 600, radius 3, pad 3, 0.5px border | `TAG_SIZE`, `TAG_WEIGHT`, `TAG_RADIUS`, `TAG_PX`, `TAG_BORDER` |
| `.tag.sm` | 8.5px | `TAG_SM_SIZE` |

The shelf is one continuous rule, not a second breakpoint. `columns = floor((content + 42) / 154)`, and the cover width is the share of that content clamped to 112..168 points. The height is `width * 3 / 2`. The column gap stays 42 and the row gap stays 28. At the mockup content width of 808 the rule is five columns of 128 by 192. A content width of 648 is four columns of 130.5 by 195.75. A content width of 266, the two-column minimum, is two columns of 112 by 168. A cover never grows past 168, and the row never runs past the content width.

The continue-reading row keeps the 52 by 78 cover. A card needs 103 points before it can sit in the row. The row takes the largest count from 1 to 3 that fits, then stretches those cards across the content width. At 808 that is three cards. At 266 it is two.

The window minimum is 578 by 528. The width is the 220-point sidebar, the 6-point inset, the 40-point card padding, a 266-point two-column shelf, the padding again, and the inset. The height is the card stack: the 36-point toolbar, the title, the continue row, one full shelf row, both section labels, a 12-point corner clearance so the caption is not clipped by the card radius, and the 6-point inset above and below. The sidebar list scrolls in the space above the pinned settings row. The chat panel is 320 points and does not shrink. It is shown only when the content width beside it is still at least 266, which is a 898-point window with the sidebar open. Below that the library form hides the panel. The preference is kept, and the toolbar toggle shows it again once the window is wide enough. This is not the reading-form annotation-margin drawer.

## Alignment grid

NSH centers the traffic lights in the hidden title bar. That bar is `CHROME_H` (48), so the chrome center is `CHROME_H / 2` (24) at every window size. `chrome_center` and `chrome_top` are the only vertical origins for that row.

`chrome_row` takes one live input: whether the three traffic-light frames are visible, the measured bar, and the content view's top inset. A button counts as visible only when it is shown and its frame sits in the title-bar band. A fullscreen window parks the buttons outside that band, so the lights are hidden. The toggle's left edge is then `SIDE_PAD_X`, the same pad as the sidebar rows. While the lights are visible and the bar is wider than 8 points, the toggle's left edge is the measured bar. The header height is the inset plus the pad, the toggle, and the pad again. `SIDE_PAD_X` equals `TOGGLE_TOP`, so a hidden-light header stays 48 points tall when the inset is zero. A real inset in the title-bar band, and only when the layout is as tall as the window, moves the sidebar header, the library card, and the chat panel down together. `layout.json` records `lights`, `traffic_bar`, and `content_top`.

The same center is the vertical middle of the sidebar top row (`TOP_ROW_H = CHROME_H`), the sidebar toggle (`TOGGLE_TOP = chrome_top(TOGGLE_SIZE)`), the chat header (`CHAT_HEAD_H = CHROME_H`), and the card toolbar. The card starts at `INSET` (6), so the toolbar height is `CHROME_H - 2 * INSET` (36) and its middle is `INSET + TOOLBAR_H / 2`, which is the light center. The stylesheet's 52-point rows and 40-point toolbar are the mockup; the shell does not use them, because they sit 2 points below the lights at every size.

Horizontal edges share one pad:

| Edge | Token |
| --- | --- |
| Sidebar rows, group labels, and the settings row | `SIDE_PAD_X` (10). A group label does not add `LABEL_PX` on top of that pad. |
| Card content: shelf header, section labels, continue cards, shelf grid | `CARD_PAD_X` (40), measured from the card's left |
| Chat header contents | `CHAT_PAD_X` (16), measured from the chat panel's left |
| Card to the sidebar, the chat panel, and the window | `INSET` (6) |

An icon and a label in one row share that row's vertical center. A pair on one baseline (the shelf title and its count, the chat name and its subtitle) shares the bottom of the line box. Repeated gaps stay on their tokens: the shelf column gap is 42, the shelf row gap is `SHELF_ROW_GAP` (28), and the continue gap is `CONTINUE_GAP` (16). The source menu clears the settings stack `SIDE_PAD_BOTTOM + ROW_H`, not a copy of the old 52-point row.

`layout.json` records `chrome_center`, `inset`, `sidebar_pad_x`, `card_pad_x`, `chat_pad_x`, `window_corner`, `composer_inset`, `composer_pad`, `lights`, `traffic_bar`, and `content_top` with the frames. `shell-clip` checks those centers, edges, and gaps within 0.5 points at the minimum window, at 1440 by 900, and at full screen. While `lights` is false, the sidebar toggle's left edge is `sidebar_pad_x` and no chrome control sits on `traffic_bar`. The search field, the shelf title, and the first cover stay inside the visible window. Consecutive fullscreen frames of the same window size must not move those boxes, and the windowed frames after a return must match the frames from before. Measured traffic-light frames from `chrome.json` are included when that file is the same window.

`WINDOW_CORNER` is `CARD_RADIUS + INSET` (18). It is the macOS window corner. The gray sidebar and the gray chat panel are the window base, so they meet the window edge. The pinned settings row and the composer card stay outside that corner. The library card keeps the 6-point inset on every side. Its own 12-point radius keeps the painted corner inside the window.

## Chat

| CSS | Value | Rust |
| --- | --- | --- |
| `.ag-head` | height 52 in the stylesheet, gap 10, pad from `.agent` 16 | `CHAT_HEAD_H` is `CHROME_H` (48), `CHAT_HEAD_GAP`, `CHAT_PAD_X` |
| `.avatar` | 26px, `#F0B267` to `#D2693C`, mark 13px / 700 | `AVATAR`, `AVATAR_FROM`, `AVATAR_TO`, `AVATAR_MARK_SIZE`, `AVATAR_MARK_WEIGHT` |
| `.ag-name` / `.ag-sub` | 13px / 600 and 11.5px / 400, gap 6 | `NAME_SIZE`, `NAME_WEIGHT`, `SUB_SIZE`, `NAME_GAP` |
| header icons | compose `edit`, panel `sidebar-right`, 18px | `PANEL_ICON` |
| `.conv` | gap 14, padding-top 12 | `CONV_GAP`, `CONV_PT` |
| `.ctx-line` | 11px `#A1A1A6` | `TIME_RULE_SIZE`, `FAINT` |
| `.msg-u` | radius 14, pad 8 12, max 250, white | `BUBBLE_RADIUS`, `BUBBLE_PY`, `BUBBLE_PX`, `BUBBLE_MAX`, `CARD` |
| bubble shadow | 0.5px ring, 1px/2px | `bubble_shadows` |
| `.msg-a` | 13px `#1d1d1f` | `MSG_SIZE`, `INK` |
| `.chip` | 12px, pad 4 10, radius 999 | `CHIP_SIZE`, `CHIP_PY`, `CHIP_PX`, `CHIP_RADIUS` |
| `.composer` | stylesheet inset 12, radius 14, pad 12 10 8 14 | `COMPOSER_INSET` is `WINDOW_CORNER` (18), `COMPOSER_RADIUS`, `COMPOSER_PAD` (12) on every side |
| composer shadow | 0.5px ring, 2px/10px | `composer_shadows` |
| `.ph` | 13px `#A1A1A6` | `BODY_SIZE`, `FAINT` |
| `.cmp-row` | margin-top 20, gap 6 | `COMPOSER_ROW_MT`, `COMPOSER_ROW_GAP` |
| `.scope` | stylesheet height 22, radius 6, `#F2F2F4`, 11.5px | `SCOPE_H` is `COMPOSER_CONTROL` (26), `SCOPE_RADIUS`, `SCOPE_BG`, `SUB_SIZE`. The chip hugs its text. |
| `.send` | 26px, `#D8D8DD` | `SEND` is `COMPOSER_CONTROL`, `SEND_BG` |
| paperclip / send glyph | 16px, 14px | `ATTACH_ICON`, `SEND_ICON` |

With no stored chat lines the thread is empty. The placeholder **问问 Slowpage…** stays on the composer. The fixture writes two exchanges when `SUPERREADER_FORM_FIXTURE=1`.

The composer is one card. Its outer gap is `COMPOSER_INSET` on the left, the right, and the bottom, and that gap is the window corner, so the radius is fully inside the window. Its inner pad is `COMPOSER_PAD` on every side. The placeholder and the paperclip share that left edge. The paperclip is the 16-point glyph in a control of height `COMPOSER_CONTROL`, not the 28-point sidebar toggle. The book chip is the same height as the send button and as wide as its icon, its text, and `SCOPE_PX`. The free space in the row sits between the chip and the send button. The chip does not grow.

## Icons

IconMCP only. Outline, stroke 1.25, radius 2. Solid is the selected row. `assets/icons/manifest.toml` lists them. `scripts/icons-fetch.sh` (the `just icons-fetch` recipe) writes the SVGs.

| Mockup glyph | IconMCP name | File |
| --- | --- | --- |
| sidebar toggle | `sidebar-left` | `sidebar-left.svg` |
| search | `search` | `search.svg` |
| 全部书籍 | `library` | `library.svg`, `library-solid.svg` |
| 正在读 | `book-open` | `book-open.svg`, `book-open-solid.svg` |
| 想读 | `bookmark` | `bookmark.svg`, `bookmark-solid.svg` |
| 已读完 | `check` | `check.svg`, `check-solid.svg` |
| PDF | `file` | `file.svg`, `file-solid.svg` |
| 高亮与笔记 | `note` | `note.svg`, `note-solid.svg` |
| Apple Books badge | `book-open` | `book-open.svg` |
| calibre badge | `library` | `library.svg` |
| folder source | `folder` | `folder.svg`, `folder-solid.svg` |
| 添加来源 | `plus` | `plus.svg` |
| 设置 | `sliders` | `sliders.svg`, `sliders-solid.svg` |
| 最近阅读 | `sort-desc` | `sort-desc.svg` |
| chevron | `chevron-down` | `chevron-down.svg` |
| grid | `grid` | `grid.svg` |
| list | `list` | `list.svg` |
| new chat | `edit` | `edit.svg` |
| panel | `sidebar-right` | `sidebar-right.svg` |
| attach | `paperclip` | `paperclip.svg` |
| scope chip | `book-open` | `book-open.svg` |
| send | `arrow-up` | `arrow-up.svg` |

The reading toolbar still uses `list` for the contents control. The sidebar toggle does not.
