# NSH 0.6.1

The shell pins `nsh-core`, `nsh-base`, and `nsh-platform-macos` at exactly 0.6.1. Tag `v0.6.1` is commit `2f51d8611677c8a6bbad937b39756e8d0cc45350`, published 2026-11-20. It is a patch on the 0.6.0 line. 0.6.0 is tag `v0.6.0`, commit `554163758e30d2f62f814096a6327937dcb4fe49`, published 2026-11-18. The workspace `rust-version` is 1.97. This repo's toolchain is 1.98.1. The license is Apache-2.0. A nightly after `v0.6.1` is not the pin.

`nsh-platform-macos` is built with `default-features = false`. That leaves `updater`, `svg`, and `websocket` off. `nsh.json` does not set `updater` and this repo does not write `latest.json`. `nsh-engine` 0.6.1 still depends on the `nsh-updater` crate for non-wasm targets. The shell does not call it. An update check is not part of this app.

`nsh.json` does not declare `capabilities`, so the 0.6.0 rule that a declared list must include `application` and `homeDir` does not apply. The shell does not use `clip_chars`, NSH web-view downloads, `ContextMenu`, `Transition`, `ModalConfig`, or `RichEditor`. File open is still `Root::url_schemes(["file"])` and `next_open_files`, which returns `Answer<Vec<PathBuf>>`. A press still goes to one node. `pick_press` gives a tap to the innermost enabled view that listens for a tap. `Press::target` sends `Press` to that node.

## What 0.6.0 changed for this shell

The release notes on [nsh.dev](https://nsh.dev/releases/) and the `v0.6.0` GitHub release list every break. The ones this shell meets are these.

`AppOptions`, `TitleBarOptions`, and `KeyOptions` gained fields. Each still implements `Default`. The shell sets the fields it already set and fills the rest from `Default`. `TitleBarOptions` stays `non_exhaustive`. The hidden title bar and its height are unchanged. `for_focus` on a key binding defaults to false.

`Composition` still has `composing` and `text`. Card and new-note fields still read `composing` from `on_composition_change`.

SVG loading through `asset!` or `assets!` needs the `svg` feature in 0.6.1. This shell does not use those macros. Its icons are rasterized by its own `resvg`.

## What 0.5.1 was

The previous pin was NSH 0.5.1, published on 2026-11-14 as tag `v0.5.1`, commit `224d9a7`. 0.5.1 is a bug-fix release of 0.5.0 with no API change, so the 0.5.0 notes below still describe that shell.

The app stays a Rust root. `spikes/shell/nsh.json` names `SpikeApp`. Headless checks stay in `spikes/shell/shell.test.tsx`, run by `nsh test`.

## What 0.4.0 changed for this shell

The release notes on [nsh.dev](https://nsh.dev/releases/) list every break between 0.3 and 0.4. The ones this shell hits are these.

Hover handlers are `on_mouse_enter` and `on_mouse_leave`, and each one takes a mouse event. In 0.4, Rust `set_title_bar(style, height)` kept its two arguments. 0.5 removed that function. The TypeScript form in 0.4 became one object, `Window.setTitleBar({ style, height })`. 0.5 replaced the window API with `WindowRef`.

`set_frame` measures the origin from the top-left of the primary screen, with y growing down. This shell still places the window with AppKit frames.

`nsh test` compares values the way Jest does. The four shell tests still use `toContain`.

## What 0.5.0 changed for this shell

The release notes on [nsh.dev](https://nsh.dev/releases/) list every break between 0.4 and 0.5. The ones this shell hits are these.

`set_title_bar` is gone. The shell sets the main window with `current_window().set` and `TitleBarOptions`. The structs are `non_exhaustive`, so the shell starts from `Default` and sets the style to `TitleBarStyle::Hidden` and the height to `chrome::TOP`. Attributes left unset keep their values.

A hidden title bar with that height no longer installs an empty toolbar. NSH centers the traffic lights in the height. It stopped attaching the toolbar because macOS 26 gives a window with a toolbar larger corners than a window with only a title bar. The shell does not put the toolbar back.

`NshAppDelegate` implements `application:openURLs:` and sends the URL string to the core as a scheme event. The shell does not replace that method. `nsh.json` `fileTypes` is how NSH's generated root claims the `file` scheme. This shell's `main` is hand-written, so it sets `Root::url_schemes(["file"])` when `fileTypes` is present. A task waits on `next_open_files` and queues the absolute paths. The reader stage opens them.

`nsh test` on macOS starts the test host in the background and does not activate its window. Pass `--foreground` when a check needs the window in front.

`nsh-updater` is in this stable release. `nsh.json` `version` is `0.1.0-alpha.2`. This shell does not set `updater` and does not write `latest.json`. Production updates are not supported. What a later key still needs is in `docs/release.md`.

## What moved onto NSH

The main window's hidden title bar is `current_window().set` with `TitleBarStyle::Hidden` and height `chrome::TOP` (48 points).

0.4 installed an empty unified toolbar when the height was above 44 points, so the traffic lights sat in that row. 0.5 does not install that toolbar. It centers the traffic lights in the height. A height of `None` still installs no toolbar.

The hidden style makes the title bar transparent, hides the system title, and turns on a full-size content view.

In full screen, the title bar area is zero and AppKit reveals the bar when the pointer reaches the top of the screen. The shell does not add `window:willUseFullScreenPresentationOptions:` to NSH's window delegate, and it does not show or hide a toolbar itself.

The paper is Books-white `#ffffff`. There is no cream or yellow paper. It is the root view background. NSH copies an opaque view that fills the window onto the surface layer, so the title bar and the page are one white sheet.

The View menu is `set_application_menu`. The app, edit, and window menus are NSH menu roles, so Quit, Copy, and full screen stay the system actions. Each trim item calls the stage command `trim`. The check mark follows the trim id the stage publishes on `status`.

## What still needs objc2

The traffic lights fade by setting the alpha of `standardWindowButton`. NSH does not expose that.

The book title in the reading window is an `NSTextField` in the content view, and it fades with the lights. The hidden title bar does not draw that title. While an EPUB is open, the NSH title row is hidden, so this field is the title the reader sees.

`setTitlebarSeparatorStyle` clears the titlebar hairline. NSH does not. The hairline would sit on the paper.

`FullScreenPrimary` stays on the window's collection behavior. NSH sets collection behavior only from `set_level`, and that call replaces the behavior with `MoveToActiveSpace`.

Dragging the page mat uses `performWindowDragWithEvent` on the native stage view. NSH `window_drag` marks NSH views. The stage view has to keep the click so a drag on the page can select text.

The window is ordered front with `activateIgnoringOtherApps`. NSH `order_front` calls `activate`.

`SPIKE_FULLSCREEN` calls `toggleFullScreen` from the view timer, which is outside an NSH owner. The Window menu's full screen item is the NSH role.

The fitted window frame is AppKit `setFrame`. The fit math uses `visibleFrame`, whose origin is the bottom left. NSH frames use the other axis.

## Opened files

File > Open and a drop on the window are NSH. `pick_paths` is the open panel. `on_file_drop` is the drop. Both reach the stage command `open`. A drop on the Dock icon and `open -a` are `application:openURLs:`.

`NshAppDelegate` forwards each URL to the core. The core turns a `file:` URL into a path only when the root claims the `file` scheme, and it keeps that path until `next_open_files` is waiting. The shell checks the file. A book opens with the stage command `open`. A book that arrives before the stage exists is queued, and the stage opens that queue. A refused file is not queued. `opens.log` records `queued` plus the path.

`Info.plist` still lists EPUB, including the package form, and PDF. The package script writes those types, and `zh-Hans.lproj/InfoPlist.strings`, before the first `codesign`. The disk image is `Slowpage.app`. This release does not call `nsh package`. That switch is a later cleanup. `nsh package` would write `CFBundleDocumentTypes` from `fileTypes`, and it would not write the imported EPUB type that conforms to `com.apple.package`.

WKWebView, the `epub://` scheme handler, and the pdfium bitmap stay on objc2. They are the page, not the window chrome. PDF via pdfium-render is unchanged. EPUB in WKWebView is under re-evaluation. See `docs/design/north-star.md`.
