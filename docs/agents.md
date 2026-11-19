# Agent operating rules

One place for how an agent runs a task. [AGENTS.md](../AGENTS.md) points here. [CONTRIBUTING.md](../CONTRIBUTING.md) and [tooling.md](tooling.md) are not repeated below.

This file binds agents only after it is merged to `main`.

## Start of a task

Write four lines before editing:

- **Goal.** What the user asked to be true.
- **Scope.** The change that serves that goal.
- **Non-goals.** What this task will not do.
- **Done when.** An observable check that can fail.

Routine reversible work proceeds without a confirmation at each step. A visual or interaction judgment stays with Morris. Do not weaken an assertion to make a check green.

## Evidence

Keep one chain: product problem, root cause, change, and the evidence that the change held. Put each item in one of four classes:

| Class | Meaning |
| --- | --- |
| Product defect | The product did the wrong thing while the state was actually entered. |
| Verifier fault | The check scored a state that was not entered, or scored a pass from a missing input. |
| Not executed | The check did not reach the state. This is not a pass and not a product failure. |
| Future scope | Named work that is outside the current task. It does not block the current task, and it is not a reason to drop a regression that was already delivered. |

`failed`, `skipped`, and `not-run` are not `pass`. A green CI run is not product acceptance. On a pull request, the run's `headSha` is the branch head. `actions/checkout` checks out the merge commit, and that checkout is the tree the job ran.

## One run

Collect every check that can run in the same session. One failed row does not stop the rows that can still run. After a batch of fixes, run the regression once. Do not repeat an unchanged run unless there is a new change or a stated hypothesis that the new run can falsify.

## Gates that already failed in production

`scripts/verify` and `scripts/verify-lock.sh` enforce these. `scripts/harness-check` injects the bad cases and must fail them. `just verify` exits 75, once, when the 1-minute load is not under 8, when the verify lock is held, or when `spike-shell` is already running. `SR_VERIFY_MAX_LOAD` cannot raise 8. Nothing is killed.

- A fullscreen request is not a fullscreen pass. The row needs the fullscreen flag and title from `chrome.json`, the window id and scale from the window tool, and the pixel size from the PNG. Those are compared with the point size and with the earlier windowed frame. A byte-identical copy of the windowed frame is not entered. `chrome.json` does not carry a window id or a pixel size, and the check does not look for them there.
- A dark row needs a dark appearance. A file that is byte-identical to another state's capture is not that appearance.
- An Open panel or an empty shelf is not a loaded book. Later page, reopen, and PDF rows stay not executed.
- A missing fixture, an empty sha, or a mismatched sha is not a pass. A file that does not contain the named chapter is a failure of that check, not a silent success.
- A workflow must not run `cargo test -p spike-shell` for tests this tree does not contain.

Main does not contain the shell unit tests or the chapter-open CI step that pull request **#53** added. Those stay on `#53`. This branch does not cherry-pick that product branch and does not claim main already has its guards.

## Blocked work

When a task stops, record the blocker in three lines:

- **Resume when.** A condition someone can check.
- **Owner.** Who decides or runs the next step.
- **Can continue.** Work that does not need the blocker.

If nothing else in the task can move, stop. Report what was delivered, the evidence, and the blocker. Do not open another run to wait, and do not describe the wait as the result.

## What stays a human judgment

Morris approves what the screen should look like and whether an interaction is acceptable. The harness can show that a capture is the wrong window or the same file. It cannot approve the picture.

## Example: #53 contents, title, and a false fullscreen

This is a record of what happened. It is not a successful product acceptance.

**Goal.** The ordinary reader shows its contents control and its book title, and a fullscreen row is scored only when the window actually entered fullscreen.

**Scope.** The shell changes on `cursor/shell-form-tokens-d356`, plus the verifier change that stopped treating a request as the state.

**Non-goals.** S3 reading form, S4 immersive mode, and merging #53.

**Done when.** A run of the built commit shows the contents control and the title, and the fullscreen row is tied to a fullscreen window of the loaded book. That run has not happened.

What is known:

- `54d3529` shows the reader title with the traffic lights. `60d0dcf` restores the contents control. Both are on #53, not on main.
- The suite whose code head was `60d0dcf` scored fullscreen as pass from a windowed frame. That was a verifier fault. `c75397a` is the change on #53 that rejects an unentered fullscreen. The same injected case is rejected by `python3 scripts/verify --harness` on this branch. The product fullscreen entry on #53 is still not an accepted screenshot.
- CI run [44286351544](https://github.com/realmorrisliu/slowpage/actions/runs/44286351544) records head `9048bb9`. The job checked out `2898d25c2c67b731e375266ef64e668c2ad8cce5`, the merge of `9048bb9` into `3169a716`. That checkout ran five shell decision tests and `fixtures_open_on_chapter_text`, and they passed. That is not product acceptance, and those tests are not on main.
- CI run [44433135069](https://github.com/realmorrisliu/slowpage/actions/runs/44433135069) records head `b3cce3f`. The job checked out `20d3b1464d5e74b2d3bd1508ce257e1368d670df`, the merge of `b3cce3f` into `3169a716`. `no-fast-models` ran `scripts/harness-check` (the four load lines, then `harness self-test passed`). `deny` failed on [RUSTSEC-2026-0543](https://rustsec.org/advisories/RUSTSEC-2026-0543): chrono 0.4.45 is unmaintained, reached through `nsh-base` 0.5.0 and `pdfium-render` 0.9.5, both already on main. This PR does not change a lockfile, and the advisory says no safe upgrade. Ignoring it would relax deny, so it stays a dependency blocker outside this harness. `test` failed in the existing "Fetch books the typesetter tests open" step, curl exit 56, before a test binary ran. This PR does not change that step. Neither failure was rerun.
- The #53 worktree head at the last check was `c194bca`. The plain-zh derived fixture was written. The Bold baseline binary from `fdd8183c5d43e0fbf623c188eebf29551254ab03` was not built. `just verify` was not run.

### #53 resume

- **Resume when.** One check shows the 1-minute load under 8, `/tmp/superreader-verify.lock` absent, and no `spike-shell` or Slowpage process. The admitted 朝花 file still hashes to `91e59f1acb65d7bc77fa2d52fca8b223b893ad837f55c62de3e85fa848e0f293`, and the derived plain-zh fixture is the one in `spikes/epub/fixtures/plain-zh-manifest.json` on that branch. Support uses a fresh temp directory, not `~/Library/Application Support/app.slowpage.reader`.
- **Owner.** Morris coordinates the run. The worktree is `.worktrees/shell-s2-1` on `cursor/shell-form-tokens-d356`. Do not stash, reset, or switch that worktree to do other work.
- **Then.** In an isolated worktree at exactly `fdd8183c5d43e0fbf623c188eebf29551254ab03`, build with `~/.local/bin/cargo` and do not set `CARGO_TARGET_DIR`. Copy the release `spike-shell` to `/tmp/spike-shell-main-fdd8183` only if that path is not a running binary. Then one `just verify` of `c194bca` or a later #53 head, if a new load check is still under 8. If it is not, stop. Do not set `SR_VERIFY_MAX_LOAD` above 8, do not wait in a loop, and do not kill another project.
- **Can continue.** Harness work on a branch from main. Not the #53 suite.

Checked once at `2026-11-19T00:48:25Z`: 1-minute load **27.56**, lock absent, no `spike-shell`. That does not mean the load is still 27.56, and it does not mean the suite can start.
