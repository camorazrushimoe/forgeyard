# Forgeyard foundation specification

Status: draft v0
Language: Rust, static binaries (`darwin-arm64` first)
Truth: this file plus `spec/`

## 1. Purpose

Even when a model crashes:

1. The factory knows **which GitHub project** it is on.
2. It knows **busy vs idle** because the wrapper wrote start/stop.
3. It has a **short event history** on disk.

## 2. Binaries and pack

```
fy        Human CLI: help, onboard, start, stop, status, bind, do
forge     Kernel CLI: bind, hooks, envelope, tokens, require-spec
watch     Deterministic loop. No LLM.
yard      Telegram panel. No LLM. Optional.
pi        External runner on the **laptop**. One process per run.
pack      roles + skills + pack.toml
```

Shared crate: `forgeyard-core`.

## 3. Invariants

I1. Project = GitHub repo name. `fy bind` / `fy do` / `forge bind`.
I2. Hooks are process-level. Start before Pi, stop after Pi exits.
I3. Every Pi prompt comes from `forge envelope` and contains the project name.
I4. No implementation without a non-empty, locally cached `spec.md` (`forge require-spec`). For a bound repository, the cache is refreshed deterministically from the default branch before this gate; manual copying is never required.
I5. Every role run has a bounded local execution record and, when its workflow requires a decision, a schema-validated structured outcome artifact. An exit code alone is not a workflow decision. QA `outcome.json` is the merge-gate truth; its PR comment is audit-only.
I6. A stale repository spec cache blocks every new workflow transition until a successful refresh.
I7. One append-only `events.jsonl` per project. No tokens, no full prompts.
I8. One busy lock per project.
I9. Pi runs on the laptop. Application git/run/test run on the SSH host.
I10. GitHub PAT never written to the SSH host. Push and `gh` run on the laptop.

## 4. Disk

Laptop `FORGEYARD_ROOT` (default `~/.forgeyard/factory`):

```
<ROOT>/
  factory.toml
  tokens/tokens.toml
  projects/<project>/
    PROJECT.toml spec.md spec-source.json state.json events.jsonl plan.json inbox.md
    runs/<run-id>/{request.json,stdout.jsonl,stderr.log,outcome.json}
```

Host:

```
/srv/forgeyard/<project>/repo/
```

Laptop `workspace/` is not the source of truth for application files.

## 5. Bind

Accepted: repo URL, `.git`, `/issues/{n}`, `/pull/{n}`, `git@github.com:owner/repo.git`.
Rebound to a different owner/repo → exit 3.

## 6. state.json

`schema = 1`.
`agent`: `tech-pm` | `developer` | `qa` | `forge` | `watch` | `yard` | `pi` | `human`.
Busy lock lives here. Ticket progress lives in `plan.json` (see spec/watch.md).
Atomic write: tmp + fsync + rename.

## 7. Event log

Required: `ts`, `project`, `hook` (`bind|start|stop|status|token_set|token_clear|panel|run|intake`).
`run_id`: `YYYYMMDDTHHMMSSZ-` + 4 hex.
Line ≤ 2 KiB.

## 8. forge CLI

Exit: 0 ok, 1 precondition, 2 usage, 3 conflict, 4 busy, 5 tokens, 10 I/O.

```
forge bind | project | require-spec | envelope
forge hook start | hook stop
forge run --project P --agent ROLE --step STEP
forge status | log | tokens
```

`fy` is what a person types. See spec/cli.md and spec/intake.md.

## 9. Wrapper

Start hook → envelope → `pi` on laptop → validate/store outcome → stop hook.
The wrapper records the effective endpoint, selected model, HTTP/API failure class, exit code, and bounded Pi stdout/stderr without recording tokens. A role exit code of 0 is only transport success; it is not approval, a plan, or a QA verdict.
After `implement`, wrapper runs `gh pr list` (spec/github-facts.md) and push-from-laptop (spec/github-auth.md).
Do not use `pi -c`.

## 10. Workflows

Watch drives A–E (spec/watch.md). Intake is `fy do` (spec/intake.md).
First happy path: `spec.md` on the repository default branch + `fy do <url> <text>` + deterministic spec cache refresh + A/B/C with `make qa`.

## 11. Status

Shared block: `forge status`, `fy status`, yard `/status`.

## 12. Concurrency

flock on `events.jsonl.lock` and `state.json.lock`. One Pi per project.

## 13. Non-goals

Hermes, Redis, Linear, DevOps role, Pi on the cluster, PAT on the cluster,
parallel tickets, production deploy, compiling Pi into forge.

## 14. Rust sketch

`crates/forgeyard-core`, `crates/fy`, `crates/forge`, `crates/watch`, `crates/yard`.
Deps v0: serde, toml, clap, fs2, time, small HTTP client.
Laptop tools assumed on PATH: `pi`, `gh`, `ssh`, `git`.

## 15. Map

- spec/watch.md — scenarios
- spec/cluster.md — SSH workbench
- spec/intake.md — `fy do`
- spec/cli.md / onboard.md / install.md
- spec/github-facts.md / github-auth.md
- spec/llm.md / qa-command.md / pi-runner.md
- spec/tokens.md / telegram-panel.md
- spec/adversarial-review-v0.md — this pass
