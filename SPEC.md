# Forgeyard foundation specification

Status: draft v0
Language of implementation: Rust, static musl binaries
Source of truth for this kernel: this file plus `spec/`

This spec defines the **foundation**. The runner is Pi. Roles and skills are files in the pack.
Workflow (`crew`) is a later binary that must obey this spec.

## 1. Purpose

Forgeyard makes three things true even when a model crashes mid-task:

1. The factory always knows **which GitHub project** it is working on.
2. The factory always knows **whether a run is busy or idle**, because a wrapper wrote start/stop.
3. The factory always has a **short event history** on disk.

## 2. Binaries and pack

```
forge     CLI kernel. Short-lived.
yard      Telegram control panel. Long-lived. No LLM.
pi        External runner (not compiled in). One process per run.
pack      roles/*.md + skills/*/SKILL.md + pack.toml
```

Shared logic: crate `forgeyard-core`.
`crew` later writes workflow `step`. Until then `forge hook start --step` may set step.

## 3. Invariants

I1. Project = repository name. `forge bind` parses a GitHub repo/issue/PR URL.
I2. Hooks are process-level. Wrapper fires start before Pi, stop after Pi exits (including crash).
I3. Every Pi prompt is built by `forge envelope` and always contains the project name.
I4. Spec-driven gate. `forge require-spec` exits 0 only if `spec.md` is non-empty.
I5. One append-only `events.jsonl` per project, flocked, line ≤ 2 KiB. No tokens, no full prompts.
I6. One busy lock per project.

## 4. Factory layout on disk

`FORGEYARD_ROOT` or `--root`, default `./factory`:

```
<ROOT>/
  factory.toml
  tokens/tokens.toml
  projects/<project>/
    PROJECT.toml spec.md state.json events.jsonl events.jsonl.lock
    workspace/              # optional git checkout
    run/<run-id>/
```

Pack lives in `$FORGEYARD_HOME/pack/` after install, not inside each project.

## 5. GitHub URL bind

Accepted: repo URL, `.git`, `/issues/{n}`, `/pull/{n}`, `git@github.com:owner/repo.git`.
`project` := repo name. Rebound to a different owner/repo is exit 3.

## 6. state.json

`schema = 1`. Allowed `agent`: `tech-pm` | `developer` | `qa` | `forge` | `yard` | `pi`.
Allowed `workflow`: `none` | `spec-review`.
Steps for spec-review: `bound`, `spec_missing`, `spec_ready`, `review_starting`, `review_in_progress`, `review_published`, `accepted`, `changes_requested`, `blocked`, `ready_for_implementation`, `crashed`.

Atomic write: tmp + fsync + rename.

## 7. Event log

Required keys: `ts`, `project`, `hook` (`bind|start|stop|status|token_set|token_clear|panel|run`), `status` optional on start.
`run_id`: `YYYYMMDDTHHMMSSZ-` + 4 hex.

## 8. forge CLI

Exit: 0 ok, 1 precondition, 2 usage, 3 conflict, 4 busy, 5 tokens, 10 I/O.

```
forge bind <github-url>
forge project --project NAME
forge require-spec --project NAME
forge envelope --project NAME --agent ROLE [--step STEP] [--run-id ID]
forge hook start --project NAME --agent ROLE [--step STEP] [--run-id ID]
forge hook stop  --project NAME --agent ROLE --run-id ID --status ok|fail|crash [--artifact URL] [--summary TEXT]
forge run --project NAME --agent ROLE --step STEP   # wrapper around pi; see spec/pi-runner.md
forge status [--project NAME]
forge log --project NAME [--tail N]
forge tokens list | set --name N --from-stdin | clear --name N
```

`envelope` stdout prefix always includes PROJECT, REPO, AGENT, STEP, RUN_ID, SPEC, RULES, then role file, then skills, then stdin task.

## 9. Wrapper contract

```sh
RUN_ID=$(forge hook start --project "$P" --agent "$A" --step "$S") || exit $?
set +e
PROMPT=$(forge envelope --project "$P" --agent "$A" --step "$S" --run-id "$RUN_ID")
pi -p --mode json --name "forgeyard:$P:$RUN_ID" "$PROMPT"
rc=$?
set -e
forge hook stop --project "$P" --agent "$A" --run-id "$RUN_ID" --status "$st"
```

Do not use `pi -c`. New session every run.

## 10. First workflow: spec-review

bind → require spec.md → role tech-pm → skill adversarial-review → GitHub verdict → hook stop with artifact URL.
One reviewer only.

## 11. Status block

Exact text shared by `forge status` and `yard` `/status`. See `examples/status.txt`.

## 12. Concurrency

flock on `events.jsonl.lock` and `state.json.lock`.

## 13. Non-goals

- Hermes, doors, Redis, Linear, DevOps role
- compiling Pi into forge
- workflow engine `crew` (later)
- production deploy

## 14. Rust sketch

`crates/forgeyard-core`, `crates/forge`, `crates/yard`.
Deps v0: serde, toml, clap, fs2, time, small HTTP client for yard.

## 15. Pack and install

See `spec/pi-runner.md`, `spec/install.md`, `spec/roles-and-skills.md`, `pack.toml`.
