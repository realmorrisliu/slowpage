# Parallel readiness

Recorded against main `a8942eac8f74aced44419ce6635f1438b9525898` on 2026-11-19. That commit is the merge of pull request 63. The push CI for that commit is green at <https://github.com/realmorrisliu/slowpage/actions/runs/44499726104>.

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

## Verification

Ran on this cloud VM. `just` is not installed here, so the `test` recipe in the justfile was run as its cargo commands. The home-layout fixture is not Apple Books on a Mac.

`cargo fmt --manifest-path spikes/Cargo.toml -p spike-report -p spike-bold-word-starts -p spike-library -- --check` passed.

`cargo clippy --manifest-path spikes/Cargo.toml -p spike-library --all-targets --features fixture -- -D warnings` finished with no warnings.

`cargo test --manifest-path spikes/Cargo.toml -p spike-library` passed 28 tests, including `home_layout_opens_a_second_root_file_and_a_file_url`.

`cargo test --manifest-path spikes/Cargo.toml -p spike-report --lib` passed 4 tests. `cargo test --manifest-path spikes/Cargo.toml -p spike-bold-word-starts` passed 12 tests. `cargo test --manifest-path spikes/Cargo.toml -p spike-pdf --lib` passed 3 tests.

`cargo test --manifest-path spikes/Cargo.toml -p spike-epub --lib` passed 6 tests and failed `fixtures_open_on_chapter_text` because `/tmp/superreader-fixtures/pride-se.epub` is not on this VM. That test is outside this diff.

Apple Books on a Mac, the user's library, `just verify`, the shell GUI, an Alan process, `cargo deny`, and a shell build were not run. The chrono advisory in `deny.toml` is still an ignore. CI for this pull request was not polled.
