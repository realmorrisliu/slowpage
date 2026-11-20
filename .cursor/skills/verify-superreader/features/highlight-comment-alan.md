# Highlight, comment, and alan

A highlight can carry Morris's note. `@alan` in that note embeds alan in-process, and the reply streams into the thread. None of that UI or handoff exists in the spike. The highlight itself is scored by the PDF and EPUB features. The decision is in `docs/design/north-star.md`.

## Sub-features

- `comment-note` is pending. A highlight cannot carry a note.
- `comment-ui` is pending. The comment chrome is a later prototype.
- `alan-spawn` is pending. The spike does not embed `alan`. The milestone waits on `docs/roadmap.md`.
- `superreader-json` is pending. The `superreader` command is not in this repo.
- `annotations-sqlite` is pending. Annotations are not in SQLite.

## How to get to it (user POV)

- Drag a passage and highlight it. That part is the PDF and EPUB highlight checks.
- Write a note on the highlight. There is no control for this.
- Type `@alan` and a question. There is no comment field.
- Read the library back with `superreader` JSON. There is no command.

## Driving it with scripts/verify

Preconditions:

- `scripts/verify --doctor` passes.
- Do not start `alan`. The embed milestone in `docs/roadmap.md` has not started.

- **Note.** There is no command. The summary row `comment-note` stays `pending`.
- **Ask alan.** There is no command. The summary row `alan-spawn` stays `pending` until a later change adds the embed.
- **Other agents.** There is no `superreader` binary. `superreader-json` stays `pending`.
- **Proof.** Pending rows are present in `summary.json`. They are not `pass`.

## Gotchas

- A yellow highlight is not a comment thread. Do not pass `comment-note` from `pdf-highlight-ink`.
- The anchor types in `spikes/report` are the value a later thread would store. Their presence in Rust is not the comment UI.
- alan's Seatbelt sandbox is irrelevant until the command exists. Do not fake a JSON reply.
