# Debug layer

The factory already writes `state.json`, `events.jsonl`, `plan.json`, `spec-source.json`, and `runs/<id>/outcome.json`. Those files are not enough to answer “why is watch not advancing?” because the tick decision lives one frame on the stack and dies.

This spec adds a **read-only diagnosis surface**. It does not drive the workflow. Watch remains the only state machine. Pi remains the only LLM process.

## Why this exists

Observed failure mode while testing: `fy start` looks alive, Pi may have exited 0, and the same step repeats. The operator cannot tell apart:

- busy lock held
- spec missing / stale
- tech-pm outcome missing, invalid, or for another SHA
- implementing with no GitHub PR (crash counter)
- `outcome_invalid` with no parse reason
- watch spawned with stdout/stderr discarded

## Surfaces

| surface | path / command | role |
|---|---|---|
| last tick facts | `projects/<p>/why.json` | written by watch after every `tick_one` |
| watch process log | `<ROOT>/watch.log` | redirected child stdout/stderr, capped |
| parse failure | `runs/<run-id>/outcome.error` | why `outcome.json` was not stored |
| operator command | `fy why [project]` | one block assembled from files already on disk |

No fifth UI. No TUI. No MCP tool in this spec (`project_why` would be a later mcp.md addition).

## why.json

Atomic write: tmp + fsync + rename. Lock: `why.json.lock` next to the file.

Schema `1`. Extra fields may be added later; readers ignore unknown keys. Writers always emit every key below.

```json
{
  "schema": 1,
  "ts": "2026-09-15T05:00:00Z",
  "project": "toy",
  "action": "BlockedOnSpecRefresh",
  "reason": "spec_stale",
  "busy": false,
  "spec_present": true,
  "spec_stale": true,
  "spec_sha": "abc",
  "spec_verdict": "",
  "plan_item": "",
  "plan_status": "",
  "cycles": 0,
  "crashes": 0,
  "pr_open": false,
  "last_outcome": "",
  "run_id": "",
  "watch_pid": 1234
}
```

`action` is the `Action` name from the tick (`SleepBusy`, `BlockedOnSpec`, `BlockedOnSpecRefresh`, `RunTechPm`, `SeedPlan`, `RunImplement`, `RunQa`, `MergePr`, `PlanDone`, `RetryBlocked`).

`reason` is a short stable token, not free prose:

| reason | when |
|---|---|
| `busy` | `SleepBusy` |
| `spec_missing` | no usable cached spec |
| `spec_stale` | `spec-source.json.stale` |
| `spec_unreviewed` | no current-SHA tech-pm approve |
| `spec_rejected` | latest current-SHA verdict is reject / needs_changes |
| `plan_seeded` | empty plan → one item |
| `plan_done` | all items done/merged |
| `run_implement` | starting or retrying implement |
| `run_qa` | starting QA |
| `merge_pr` | watch merging |
| `no_pr` | implementing and GitHub shows no open PR for the item branch |
| `outcome_invalid` | last run for this step had no validated outcome |
| `retry_blocked` | cycle or crash cap hit |
| `idle_tick` | nothing else applied |

`last_outcome` shape when present: `kind/verdict@sha-or-pr` (example `spec_review/approve@abc`, `qa/no_merge@9`). Empty if none.

`spec_verdict` is `approve`, `reject`, or empty.

why.json is **not** an input to `tick`. Deleting it must not change workflow behaviour. A missing or undecodable file is treated by `fy why` as absent (`why: -`).

Write it even on `SleepBusy`. Overwrite in place; no history file.

## Events

Do not append an event on every 2s loop sleep.

When `action` changes since the previous why.json for that project, append to the **project** `events.jsonl`:

```
hook=run step=tick status=<action> summary=<reason>
```

`agent=watch`. No secrets. Cap still 2 KiB. First tick on a project (no previous why.json) counts as a change.

Factory `<ROOT>/events.jsonl` is not used for tick lines.

## watch.log

`fy start` must not spawn watch with discarded stdout/stderr.

- Path: `<ROOT>/watch.log`
- Append. If the file exceeds 256 KiB, rename to `watch.log.1` (replace previous `.1`) and start a new `watch.log`.
- Same redaction as event lines (`sanitize`). Do not write tokens.toml values.
- `FORGEYARD_WATCH_TRACE=1`: also print `project action reason` on the `fy start` TTY each time why.json is written with a changed action.

Missing log file is not an error for `fy why`.

## outcome.error

Written only when the wrapper would set `summary=outcome_invalid` (parse miss or `validate_outcome` fail).

Path: `runs/<run-id>/outcome.error`

```
reason: missing kind
excerpt: <≤200 bytes of the scanned text, already sanitized>
```

Do not dump full Pi stdout. Do not write this file on `runner_missing`, `pi_exit_*`, or API classification failures — those already have `hook=stop summary=`.

If a later run stores a valid `outcome.json` for the same `run_id`, delete `outcome.error`.

## fy why

```
fy why
fy why <project>
```

Project resolution matches `fy status`: argv, else current project, else usage exit 2.
Unknown project → exit 1 (precondition), same message shape as `fy status`.

Byte-stable block (keys left-aligned width 12):

```
forgeyard why
project:    toy
watch:      up|down
busy:       yes|no
workflow:   spec-review
step:       review
action:     BlockedOnSpecRefresh
reason:     spec_stale
spec:       present stale sha=abc verdict=-
plan:       item=1 status=ready cycles=1 crashes=0
pr:         no
outcome:    spec_review/approve@old
run:        20260915T050000Z-ab12
stop:       fail outcome_invalid
error:      runs/20260915T050000Z-ab12/outcome.error
updated:    2026-09-15T05:00:00Z
```

Sources, in this order of display:

| line | source |
|---|---|
| watch | `watch.pid` + alive check |
| busy / workflow / step / run | `state.json` |
| action / reason / updated | `why.json` (`-` if missing) |
| spec | `spec-source.json` + cache file |
| plan | `plan.json` current in-flight item, else `-` |
| pr | `why.json.pr_open` (do not call GitHub from `fy why`) |
| outcome | `why.json.last_outcome` or latest matching `outcome.json` |
| stop | last project `hook=stop` status + summary |
| error | path if `outcome.error` exists for `state.run_id` or last run |

`fy why` never calls an LLM, never SSHes, never talks to GitHub. If a fact is missing, print `-`.

Add the command to `fy help`.

## Watch / start behaviour

After `tick` computes an `Action` and before/after dispatch (order does not matter as long as it is the same tick):

1. Write `why.json` from the `WatchIo` facts used for that decision plus the action.
2. If action changed, append the tick event.
3. Dispatch as today.

`watch loop` must not swallow the action: it is recorded on disk even when `fy start` is not tailed.

`watch tick P` still prints `Action` on stdout (existing behaviour) **and** writes why.json.

## Status line

Do not change the existing `fy status` golden block. Diagnosis is `fy why`. Optionally later, `factory_status` may grow a `why:` line; not in this spec.

## Never

- feed why.json back into `tick`
- treat Pi exit 0 as success
- print PAT, LLM keys, SSH password, mcp bind token, ngrok auth token
- store full prompts or unbounded Pi stdout in why.json / outcome.error / watch.log
- require GitHub or the cluster to answer `fy why`

## Test contract

No live GitHub, no live Pi, no live ngrok.

- Fake `WatchIo` + one tick writes why.json with the matching `action` / `reason`.
- Second tick with the same action does not append a new `hook=run step=tick` line.
- Third tick with a different action appends exactly one new tick line.
- `fy why` on a fixture directory matches the golden block above (watch pid may be `down`).
- Wrapper path that used to yield `outcome_invalid` also writes `outcome.error` and still does not write `outcome.json`.
- why.json deleted → workflow fixtures still pass; `fy why` prints `action: -`.
