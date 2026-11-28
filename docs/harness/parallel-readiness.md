# Parallel readiness

Recorded against main `a8942eac8f74aced44419ce6635f1438b9525898` on 2026-11-19. That commit is the merge of pull request 63. The push CI for that commit is green at <https://github.com/thinksmart-systems/slowpage/actions/runs/44499726104>.

Open pull requests 55, 56, 57, and 58 are not the base of this change. None of them edit `spikes/library` or `docs/harness`.

## Apple Books import

`spikes/library` already reads a read-only copy of an Apple Books sqlite database and classifies each row. A fixture home on this VM, not a user Mac, showed two rows that were on disk and still labeled missing. A bare filename that exists only under `Library/Containers/com.apple.BKAgentService/Data/Documents/iBooks/Books` was missing when the iCloud Documents folder also existed, because the first root was returned without checking for the file. A `file:` URL of an EPUB inside an allowed folder was missing because the string was used as a literal path. Absolute paths, the newest sqlite, a row that exists only in the WAL, store books, audiobooks, iCloud placeholders, series omission, and an unchanged source file already worked.

The fix stays inside classification. A `file:` value is decoded with the existing `file_uri_path`. A relative name is taken from the first allowed root that already contains the file or its `.icloud` placeholder. Absolute paths still have to sit under an allowed root. A non-empty store id still returns before any disk read. `open`, `Scope`, `Import`, and `Book` keep their current signatures. The shell is unchanged.

This change does not write the library, import annotations, search, write Calibre, or open a shelf row in the reader. Annotation import needs a schema that is not confirmed. Opening a shelf row needs the shell.

## Alan

Alan issue 1078 is open. The roadmap says the Alan milestone waits for a launch flag in the Alan repository. This repository has no spawn command and no annotations database. No adapter is added here.

## This slice

A book that is on disk in the Apple Books folders is listed as openable when the database stores a bare filename in the second folder or a `file:` URL under an allowed folder. A store book and an audiobook stay unavailable and are not opened. The source sqlite bytes do not change. A second import matches the first.

This slice does not decrypt FairPlay, play audiobooks, write Calibre, change the reader, or talk to Alan.

It is done when `home_layout_opens_a_second_root_file_and_a_file_url` passes on the fixture home and the public catalog API is unchanged.

## File ownership

This branch owns `spikes/library/src/apple.rs`, `spikes/library/src/tests.rs`, and `docs/harness/parallel-readiness.md`.

The reading line owns `spikes/shell/**`, `docs/roadmap.md`, `.cursor/skills/verify-superreader/features/**`, `.github/workflows/**`, `Cargo.lock`, and the root `Cargo.toml`.

## Integration order

Both lines target `main` at `a8942eac8f74aced44419ce6635f1438b9525898`. This branch is not stacked on pull requests 55, 56, 57, or 58. The file sets do not overlap, so either line can merge first.

## Dalibre

Checked [djyde/dalibre](https://github.com/djyde/dalibre) at `aff3eb5e66b1bd0287bfc42b46e93b6fcc2d7151`. The [LICENSE](https://github.com/djyde/dalibre/blob/aff3eb5e66b1bd0287bfc42b46e93b6fcc2d7151/LICENSE) is GPL-3.0 or later. The README says `internal/calibre/metadata_sqlite.sql` is calibre's schema under GPL-3.0. No source is copied.

Dalibre reads and writes a Calibre library. A search of that commit for Apple Books, iBooks, and `ZBKLIBRARYASSET` returns nothing.

In [`internal/calibre/library.go`](https://github.com/djyde/dalibre/blob/aff3eb5e66b1bd0287bfc42b46e93b6fcc2d7151/internal/calibre/library.go) a book path is relative to one library root. `Dir` joins that root and the stored path. A format file is that folder plus the format name. `OpenReadOnly` opens the original `metadata.db` with `mode=ro`. [`internal/catalog/catalog.go`](https://github.com/djyde/dalibre/blob/aff3eb5e66b1bd0287bfc42b46e93b6fcc2d7151/internal/catalog/catalog.go) filters and sorts books that are already loaded. It does not find files.

The useful mechanism is already in this slice. A relative name belongs under the library root, and the openable file is the one that join names. Apple Books has two roots, so this slice takes the first root that already has the file or its `.icloud` placeholder. `home_layout_opens_a_second_root_file_and_a_file_url` is the acceptance. Using Dalibre's single join would mark the second-root file missing again.

Not taken. Opening the live database with `mode=ro` instead of a snapshot. Apple Books keeps its database open, and this crate already copies the sqlite, wal, and shm before a read-only open. Search, filters, metadata edits, OPF writes, plugins, and MCP are outside this slice. GPL-3.0 also rules out copying their source. This check changes no Rust.

## Verification

Ran on this cloud VM. `just` is not installed here, so the `test` recipe in the justfile was run as its cargo commands. The home-layout fixture is not Apple Books on a Mac.

`cargo fmt --manifest-path spikes/Cargo.toml -p spike-report -p spike-bold-word-starts -p spike-library -- --check` passed.

`cargo clippy --manifest-path spikes/Cargo.toml -p spike-library --all-targets --features fixture -- -D warnings` finished with no warnings.

`cargo test --manifest-path spikes/Cargo.toml -p spike-library` passed 28 tests, including `home_layout_opens_a_second_root_file_and_a_file_url`.

`cargo test --manifest-path spikes/Cargo.toml -p spike-report --lib` passed 4 tests. `cargo test --manifest-path spikes/Cargo.toml -p spike-bold-word-starts` passed 12 tests. `cargo test --manifest-path spikes/Cargo.toml -p spike-pdf --lib` passed 3 tests.

`cargo test --manifest-path spikes/Cargo.toml -p spike-epub --lib` passed 6 tests and failed `fixtures_open_on_chapter_text` because `/tmp/superreader-fixtures/pride-se.epub` is not on this VM. That test is outside this diff.

Apple Books on a Mac, the user's library, `just verify`, the shell GUI, an Alan process, `cargo deny`, and a shell build were not run. The chrono advisory in `deny.toml` is still an ignore. CI for this pull request was not polled.

## Cloud and series rows

Recorded against main `6ad10945cd3932b77257c26ddbfcfea78e0c9b42`, the squash of pull request 64. Open pull request 65 owns `spikes/shell/**`, `docs/roadmap.md`, the shell verify feature, `.github/workflows/**`, `Cargo.lock`, and the root `Cargo.toml`. This slice is based on `6ad10945`, not on pull request 65.

A cloud book with no file was labeled Missing. An unowned series row with no file was listed as a missing book. [BookMinder's Apple Books notes](https://github.com/palimondo/BookMinder/blob/master/docs/apple_books.md) record `ZSTATE` 3 as a cloud book, 6 as a cloud sample that is not downloaded, and 5 as a series or unowned title. That note is one observed library, not an Apple document. Databases without `ZSTATE` keep the previous labels.

A file that is already an EPUB or PDF still opens, including when `ZSTATE` is 3 or 5. A store id still returns before any disk read. An audiobook stays an audiobook. The source sqlite bytes do not change. A second import matches the first. `open`, `Scope`, `Import`, and `Book` stay as they are.

This slice does not decrypt FairPlay, play audiobooks, search, sort, filter, write Calibre, import annotations, or open a shelf row in the reader. Search and filters need the shell. Annotation import still has no confirmed schema. Alan issue 1078 is still open.

Dalibre at `aff3eb5e66b1bd0287bfc42b46e93b6fcc2d7151` remains GPL-3.0 or later. No source is copied. Its catalog filters books that are already loaded, and this slice does not add that. The part used here is the same rule as last time. A catalog row is not an openable file until the file is on disk.

This branch owns `spikes/library/src/apple.rs`, `spikes/library/src/tests.rs`, and `docs/harness/parallel-readiness.md`. Pull request 65 does not edit those paths. Either line can merge to `main` first.

It is done when `cloud_and_series_rows_do_not_look_like_missing_files` passes on a fixture folder and the public catalog API is unchanged. That fixture is not Apple Books on a Mac.

## Local copy over an iCloud placeholder

Recorded against main `576195310bf90d6e20c5c730121d9dfce017b83c`, the squash of pull request 66. Open pull request 65 still owns the shell, the roadmap, the shell verify feature, workflows, the lockfile, and the root manifest. This slice is based on `5761953`, not on that pull request.

A relative name that exists as a real file in the Books folder was labeled not downloaded when the iCloud Documents folder held only `.{name}.icloud`. The first root was returned as soon as a placeholder existed. A real file in the iCloud folder still wins when it is there. A name that is only a placeholder stays not downloaded. A store id still returns before any disk read. The source sqlite bytes do not change.

`open`, `Scope`, `Import`, and `Book` stay as they are. This slice does not download placeholders, decrypt FairPlay, play audiobooks, search, or open a shelf row in the reader.

Dalibre at `aff3eb5` is still GPL-3.0 or later. No source is copied. It has one library root, so it does not choose between a placeholder and a second folder. The rule kept from that check is the same. The openable book is the file on disk.

This branch owns `spikes/library/src/apple.rs`, `spikes/library/src/tests.rs`, and `docs/harness/parallel-readiness.md`. Pull request 65 does not edit those paths. Either line can merge to `main` first.

It is done when `a_local_copy_opens_when_icloud_only_has_the_placeholder` passes on a fixture home. That home is not a user Mac.

## Shelf title and author search

Recorded against main `5805eb2525e6a568bb4c84340ee7de2e11d8929b`, the squash of pull request 67. Open pull request 65 still owns `spikes/shell/src/chrome.rs`, `settings.rs`, `stage.rs`, `visit.rs`, `web.rs`, `docs/roadmap.md`, `.cursor/skills/verify-superreader/features/shell.md`, `.github/workflows/ci.yml`, `Cargo.lock`, and the root `Cargo.toml`. This slice does not edit those paths. `spikes/shell/src/lib.rs` is not in that list and is not edited here. The shelf projection already runs there. The query is a `Signal<String>` created in `form::mount`, and `shelf::searched` filters the `LibraryIn` that projection already built.

The shelf field finds an imported book whose title or author contains the text. Blank text, including spaces, shows the same shelf again. A miss leaves the grid empty, keeps the library counts and sources, and does not switch the card to the empty import state. The matching card keeps its open path. The check is a Unicode lowercase substring. It is not a search service and not an index. Notes, reading position, and source files stay as they are.

Dalibre at `aff3eb5` is still GPL-3.0 or later. No source is copied. Its catalog filters books already loaded. This slice does the same kind of filter on the shelf cards, for title and author only.

This branch owns `spikes/shell/src/shelf.rs`, `spikes/shell/src/form.rs`, and `docs/harness/parallel-readiness.md`.

Morris stated this for Slowpage on 2026-11-19 18:41 Asia/Shanghai. An established slice is implemented, reviewed on its own, checked with the cloud checks that apply, merged in the normal way, and the next slice starts. It does not wait for a separate approval on every pull request. A new scope, a new visual design, or an architecture tradeoff still waits for Morris. Security controls stay in place. A check that did not run on a Mac is not a pass. This note does not apply to another project.

It is done when `a_title_or_author_query_filters_the_shelf_and_clears` passes on the form fixture. That fixture is not a user Mac. The shell release build is the macOS compile of `spike-shell`. A passing compile is not a GUI pass.
