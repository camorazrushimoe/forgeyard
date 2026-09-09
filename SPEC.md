# Forgeyard foundation specification

Status: draft v0
Language of implementation: Rust, static `x86_64-unknown-linux-musl` binaries
Source of truth for this kernel: this file plus `spec/`

This spec defines the **foundation**, not the agents and not the full team workflow.
Agents are ordinary Hermes processes. Workflow (`crew`) is a later binary that must obey this spec.

## 1. Purpose

Forgeyard makes three things true even when a model crashes mid-task:

1. The factory always knows **which GitHub project** it is working on.
2. The factory always knows **whether an agent is busy or idle**, because a wrapper wrote start/stop.
3. The factory always has a **short event history** on disk.

Everything else (review quality, code quality, Linear, Redis, MCP) is out of scope.

## 2. Binaries

```
forge     CLI kernel. Short-lived. Called by wrappers, scripts, yard, and later crew.
yard      Telegram control panel. Long-lived. Reads the same files. Deterministic. No LLM.
```

Shared logic lives in a library crate `forgeyard-core` (project bind, state, log, tokens file).
`yard` does not reimplement status. It calls the same functions as `forge status`.

`crew` is **not** specified here beyond: it is the only writer of workflow `step` in `state.json`.
Until `crew` exists, `forge hook start|stop` may set `agent` / `agent_status` / `run_id` and must not invent new workflow names.

## 3. Invariants

I1. **Project = repository name.**
    Input is a GitHub URL to a repository, an issue, or a pull request.
    `forge bind` parses it, sets `project = repo`, and refuses to continue without a repo name.

I2. **Hooks are process-level.**
    `forge hook start` runs immediately before the agent process is spawned.
    `forge hook stop` runs after the process exits, including crash and signal.
    The language model is forbidden from being the sole caller of hooks.
    A hook invoked twice with the same `(project, run_id, hook)` is idempotent.

I3. **Every agent request contains the project name.**
    `forge envelope` is the only supported way to build the prompt prefix.
    If `--project` is missing or unknown, envelope exits `2` and prints nothing to stdout.

I4. **Spec-driven gate.**
    `forge require-spec --project P` exits `0` only if `projects/P/spec.md` exists and is non-empty.
    Implementation workflows must call this gate. The first workflow (`spec-review`) needs the file to review.
    Code generation without a spec is a contract violation of the factory, not a style issue.

I5. **One event log per project.**
    Path: `projects/<project>/events.jsonl`.
    Writes are append-only, one JSON object per line, under an exclusive flock on `events.jsonl.lock`.
    A line is at most 2 KiB. No spec text, no token values, no full prompts.

I6. **One busy agent lock per project.**
    A project has at most one `agent_status = busy` at a time in v0.
    `hook start` fails if the project is already busy with a different live `run_id`.

## 4. Factory layout on disk

Root is configured once (`FORGEYARD_ROOT` or `--root`, default `./factory`):

```
<ROOT>/
  factory.toml              # root config: github host, default workflow
  tokens/tokens.toml        # secrets, mode 0600, never in git
  projects/
    <project>/
      PROJECT.toml          # binding: owner, repo, source url
      spec.md               # the specification the agents are allowed to use
      state.json            # current step + busy agent
      events.jsonl          # append-only log
      events.jsonl.lock     # flock target (empty file)
      run/<run-id>/         # optional artifacts of one run
```

`project` directory name is the repo name. It must match `PROJECT.toml` field `project`.
Characters allowed: `A-Za-z0-9._-`. No slashes.

## 5. GitHub URL bind

Accepted inputs:

```
https://github.com/{owner}/{repo}
https://github.com/{owner}/{repo}.git
https://github.com/{owner}/{repo}/issues/{n}
https://github.com/{owner}/{repo}/pull/{n}
git@github.com:{owner}/{repo}.git
```

Rules:

- `project` := `{repo}`
- `owner` and `repo` are required
- issue/PR numbers go to `source_kind` + `source_ref`
- binding an already bound project with the same owner/repo is idempotent and updates `source_*` if the URL is more specific
- binding the same project name to a **different** owner/repo is an error (exit `3`)

`PROJECT.toml` fields:

| field | required | meaning |
|---|---|---|
| `project` | yes | repo name |
| `owner` | yes | GitHub owner |
| `repo` | yes | GitHub repo, equals `project` |
| `source_url` | yes | original URL |
| `source_kind` | yes | `repo` \| `issue` \| `pull` |
| `source_ref` | no | issue/PR number as string |
| `bound_at` | yes | RFC3339 UTC |

## 6. state.json

Schema version is integer `schema = 1`.

```json
{
  "schema": 1,
  "project": "my-app",
  "workflow": "spec-review",
  "step": "review_in_progress",
  "agent": "tech-pm",
  "agent_status": "busy",
  "run_id": "20260909T080012Z-ab12",
  "input": "https://github.com/camorazrushimoe/my-app/issues/14",
  "spec_path": "spec.md",
  "updated_at": "2026-09-09T08:00:12Z"
}
```

Allowed `agent` in v0: `tech-pm` | `software-engineer` | `qa` | `forge` | `yard`.
Allowed `agent_status`: `idle` | `busy`.
Allowed `workflow` in v0: `none` | `spec-review`.

Allowed `step` for `spec-review`:

```
unbound                 # should not appear on disk; bind creates bound
bound                   # project exists, spec may be missing
spec_missing
spec_ready
review_starting
review_in_progress
review_published
accepted
changes_requested
blocked
ready_for_implementation
crashed
```

`forge` writes: `agent`, `agent_status`, `run_id`, `updated_at` on hook start/stop.
`forge bind` may create the file with `workflow=none`, `step=bound`, `agent_status=idle`.
`crew` (future) writes `workflow` and `step`.
Until `crew` exists, `forge hook start --step S` may set `step` if provided.

Writes to `state.json` are atomic: write `state.json.tmp` + `fsync` + rename.

## 7. Event log

One JSON object, one line. Required keys on every line:

| key | type | |
|---|---|---|
| `ts` | RFC3339 UTC | |
| `project` | string | |
| `hook` | `bind` \| `start` \| `stop` \| `status` \| `token_set` \| `token_clear` \| `panel` | |
| `status` | `ok` \| `fail` \| `crash` \| `denied` | optional on `start` |

Optional keys: `run_id`, `agent`, `step`, `pid`, `duration_s`, `artifact`, `input`, `summary`, `token_name`.

Forbidden in a log line: raw tokens, full spec body, full envelope, Telegram bot token.

`summary` ≤ 200 characters.

`run_id` format: `YYYYMMDDTHHMMSSZ-` + 4 lowercase hex chars, generated by `forge hook start` if omitted.

## 8. forge CLI

Exit codes:

| code | meaning |
|---|---|
| 0 | ok |
| 1 | spec missing / precondition failed |
| 2 | usage / missing project |
| 3 | project conflict |
| 4 | lock busy (agent already running) |
| 5 | tokens file error |
| 10 | I/O or lock failure |

Commands:

```
forge bind <github-url> [--root DIR]
forge project --project NAME [--root DIR]
forge require-spec --project NAME [--root DIR]
forge envelope --project NAME --agent ROLE [--step STEP] [--run-id ID] [--root DIR]
forge hook start --project NAME --agent ROLE [--step STEP] [--run-id ID] [--root DIR]
forge hook stop  --project NAME --agent ROLE --run-id ID --status ok|fail|crash [--artifact URL] [--summary TEXT] [--root DIR]
forge status [--project NAME] [--root DIR]
forge log --project NAME [--tail N] [--root DIR]
forge tokens list [--root DIR]
forge tokens set --name NAME --from-stdin [--root DIR]
forge tokens clear --name NAME [--root DIR]
```

### bind

Creates `projects/<repo>/` if needed, writes `PROJECT.toml`, creates empty `events.jsonl` if missing, writes initial `state.json` if missing, appends a `bind` event.
Does not fetch GitHub. Does not clone. Network is optional in v0.

### require-spec

Exit 0 iff `spec.md` exists and size > 0.

### envelope

Writes to stdout a deterministic prefix. Stdin, if present, is appended after a blank line.
The prefix **always** contains:

```
PROJECT: <project>
REPO: <owner>/<repo>
AGENT: <role>
STEP: <step or none>
RUN_ID: <run_id or none>
SPEC: <absolute-or-root-relative path to spec.md>
RULES:
- every reply stays inside this project
- do not write code unless STEP allows it
- do not call forge hooks; the wrapper already does
```

### hook start

Fails with 4 if `agent_status=busy` and `run_id` differs.
On success: set busy, write `run_id`, append `start` line, print `run_id` to stdout.

### hook stop

Requires `--run-id`.
If state `run_id` matches: set `agent_status=idle`, keep last agent name, append `stop`.
If state `run_id` differs: still append `stop` with `status=fail` summary `stale_run_id`, exit 1. Do not clobber the live run.
`--status crash` should be used by the wrapper when the process dies on signal or non-zero without a published artifact.

### status

Without `--project`: list all projects, one line each.
With `--project`: print the human status block defined in §11. This is the same text `yard` sends for `/status`.

### tokens

See `spec/tokens.md`. Values never printed. `list` prints name, set/unset, last 4 chars, updated_at.

## 9. Wrapper contract (how an agent is run)

The only supported way to run a Hermes agent against a project:

```sh
RUN_ID=$(forge hook start --project "$P" --agent "$A" --step "$S") || exit $?
set +e
forge envelope --project "$P" --agent "$A" --step "$S" --run-id "$RUN_ID" \
  | hermes ...
rc=$?
set -e
if [ $rc -eq 0 ]; then st=ok; else st=crash; fi
forge hook stop --project "$P" --agent "$A" --run-id "$RUN_ID" --status "$st"
exit $rc
```

Session hooks inside Hermes may *also* call `forge hook`, but they are a duplicate signal, not the source of truth. The wrapper is.

## 10. First workflow note (not implemented in forge)

`spec-review`:

1. bind URL
2. ingest / require spec.md
3. assign exactly one reviewer: `tech-pm`
4. hook start
5. adversarial review of the spec (no code)
6. publish verdict to the originating issue or PR
7. hook stop with `--artifact` pointing at the GitHub comment or PR
8. step becomes `accepted` | `changes_requested` | `blocked`

`forge hook stop --status ok` for this workflow **should** include `--artifact`. `crew` will later reject `ok` without artifact. `forge` itself only warns on stderr in v0 so the kernel stays workflow-agnostic.

## 11. Status block

This exact shape is used by `forge status --project P` and by `yard` `/status`.
No extra prose. No model paraphrase.

```
forgeyard
project:  my-app
repo:     camorazrushimoe/my-app
source:   issues/14
workflow: spec-review
step:     review_in_progress
agent:    tech-pm  busy
run:      20260909T080012Z-ab12
spec:     present
updated:  2026-09-09T08:00:12Z
last:     start adversarial_review
```

If no projects exist:

```
forgeyard
projects: none
```

If `--project` omitted and several exist, one line per project:

```
my-app    spec-review  review_in_progress  tech-pm busy
other     none         bound               - idle
```

## 12. Concurrency

- flock exclusive on `events.jsonl.lock` for log append
- flock exclusive on `state.json.lock` for state read-modify-write
- `yard` holds no write lock while rendering `/status` (read `state.json` after a completed rename)

## 13. Non-goals for this repository

- Hermes agent prompts / SOUL.md
- workflow engine (`crew`)
- cloning the target repo
- talking to Linear
- Redis bus
- idle/wake of containers
- generating or reviewing code

## 14. Implementation sketch (for the first Rust commit)

Workspace:

```
crates/forgeyard-core   # bind parse, state, log, tokens, status text
crates/forge            # CLI
crates/yard             # Telegram poller
```

Dependencies allowed in v0: `serde`, `serde_json`, `toml`, `clap`, `fs2` (lock), `chrono` or `time`, `ureq` or a small Telegram HTTPS client. No async runtime required for `forge`. `yard` may use a blocking poll loop.
