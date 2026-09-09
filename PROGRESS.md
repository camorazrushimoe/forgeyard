# Implementation progress

Handoff file so a new session can continue without rereading the whole chat.

## How to continue

1. Open https://github.com/camorazrushimoe/forgeyard
2. Read this file and issue **#3** (v0 implementation tracker)
3. Take the open issue labeled `next` (or the lowest numbered open `impl` issue)
4. Branch `impl/<issue-number>-short-name` off `main`
5. Implement only that issue
6. Open a PR, write an **adversarial review** comment against SPEC.md + spec/*
7. Merge on `Verdict: approve`. Close the issue. Move the `next` label forward

## Status

| # | title | state |
|---|---|---|
| 3 | v0 implementation tracker | open |
| 4 | workspace | done (PR #19) |
| 5 | bind | done (PR #20) |
| 6 | state.json | done (PR #21) |
| 7 | events.jsonl | done (PR #22) |
| 8 | tokens.toml 0600 + meta | in this PR |
| 9 | byte-stable status block | open |
| 10 | forge CLI surface | open |
| 11 | hooks + envelope + forge run | open |
| 12 | fy help/bind/do/status | open |
| 13 | fy onboard | open |
| 14 | watch A–E | open |
| 15 | yard Telegram panel | open |
| 16 | fy start / stop | open |
| 17 | install.sh + pack + README | open |
| 18 | CI + darwin-arm64 artifacts | open |

## Current next after this PR merges

Issue **#9** — byte-stable status block.
