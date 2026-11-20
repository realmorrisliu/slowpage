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
- A row that asks for dark needs a dark appearance. The `window-aqua` rows do not: the system may be dark, and the window stays light Aqua because the launch does not set `SPIKE_APPEARANCE`. A file that is byte-identical to another state's capture is not a distinct state.
- An Open panel or an empty shelf is not a loaded book. Later page, reopen, and PDF rows stay not executed.
- A missing fixture, an empty sha, or a mismatched sha is not a pass. A file that does not contain the named chapter is a failure of that check, not a silent success.
- A workflow must not run `cargo test -p spike-shell` for tests this tree does not contain.

## Blocked work

When a task stops, record the blocker in three lines:

- **Resume when.** A condition someone can check.
- **Owner.** Who decides or runs the next step.
- **Can continue.** Work that does not need the blocker.

If nothing else in the task can move, stop. Report what was delivered, the evidence, and the blocker. Do not open another run to wait, and do not describe the wait as the result.

## What stays a human judgment

Morris approves what the screen should look like and whether an interaction is acceptable. The harness can show that a capture is the wrong window or the same file. It cannot approve the picture.

## Merge

On 2026-11-19 at 11:06 Asia/Shanghai, Morris wrote, in cu:526828663: 去掉本机运行的阻塞。只要在云端完成，那就是完成了. No later message restored a local GUI gate.

For an established route, the necessary cloud build, tests, and artifact checks, plus an independent review of the production sequence, are enough to finish and to merge the current head. A GUI run, `nsh test`, a Mac visual check, or the real library that did not run is non-blocking and unverified. Record it that way. It is not a pass.

An older profile, or a `blocked` row in `scripts/verify`, is not a reason to require that local run before merge. Leave the row `blocked`. Do not mark it `pass`. A check that failed on a state the product did enter stays a failure. Do not rename it so the bar drops. A new visual design or an architecture tradeoff still waits for Morris. Do not wait for a separate approval on each pull request of an established route. Do not weaken an assertion to make a check green.

## History

Pull request #53 merged to main as `aa1f2e0`. Its shell tests are on main. The earlier resume that named `.worktrees/shell-s2-1`, head `c194bca`, and a baseline build of `fdd8183c5d43e0fbf623c188eebf29551254ab03` is retired. Those steps are not a current action.

A suite whose code head was `60d0dcf` scored fullscreen as pass from a windowed frame. That was a verifier fault. `python3 scripts/verify --harness` rejects the injected copy. A screenshot that was not taken is not an accepted picture.

On head `b3cce3f` against `3169a716`, `deny` failed on [RUSTSEC-2026-0543](https://rustsec.org/advisories/RUSTSEC-2026-0543) and the book-download step exited with curl status 56. Those results belong to that head. Judge a later head by the checks that run on it.
