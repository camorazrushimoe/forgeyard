# MCP control plane

`mcp` is a separate long-running binary. It is a deterministic control surface over the same files `forge` owns.

It does not think. It does not call an LLM. It does not SSH. It does not merge. It maps a small tool set onto `forgeyard-core` (the same functions as `fy`).

Watch stays the only workflow driver. Pi stays the only LLM process.

## Why a fifth binary

| | `forge` / `fy` | `yard` | `mcp` |
|---|---|---|---|
| lifetime | short | days | days |
| interface | argv | Telegram | MCP tools |
| clients | human on the laptop | phone | remote / local agents |
| failure | must not depend on the network | Telegram may die | MCP may die; watch keeps running |

Do not put the MCP listener inside `watch`. A crashed tool handler must not stop the tick machine.

Shared code: `forgeyard-core`. Shared files: `state.json`, project `events.jsonl`, factory `events.jsonl`, `tokens/tokens.toml`, run artifacts.

## Process model

`fy start` always attempts to start `mcp` next to `watch` (and `yard` only if the telegram token is set).

Listen: `127.0.0.1` only. Port resolution, first match wins:

1. `FORGEYARD_MCP_PORT` if set and parseable as `1..65535`
2. `mcp.port` from `tokens.toml` if set and parseable
3. `18789`

Transport v0: MCP Streamable HTTP at `http://127.0.0.1:<port>/mcp`.
No bind to `0.0.0.0`. The public URL, if any, comes from the tunnel (see [tunnel.md](tunnel.md)).

Missing watch / factory root → tools still answer. Reads that need live state return a structured `factory_down` / `watch_down`. Writes to inbox are allowed (same as `fy do` while watch is stopped: queued, not executed).

`fy stop` stops mcp too. Pid file: `<ROOT>/mcp.pid`.

### Start contract (token + process)

`fy start` does **not** fail the factory when mcp cannot run.

1. If `mcp.bind_token` is unset at start: generate 32 random bytes hex, persist as `mcp.bind_token`, print `mcp: generated bind_token` (never the value). Same generator as onboard.
2. Spawn `mcp`. Write `mcp.pid`.
3. Missing `mcp` binary → print `mcp: missing` and continue. Watch still runs.
4. mcp process exit 5 (token still empty after the generate step, or refused) → print `mcp: refused` and continue.
5. Any other mcp spawn failure → print `mcp: down` and continue.

The `mcp` binary itself still exits 5 if it starts with an empty token. `fy start` is responsible for generating one first so a default laptop is not an open socket and is not a dead factory.

### Rotate contract

- First onboard, or onboard when `mcp.bind_token` is unset: empty input generates a token.
- Re-run onboard when a token is already set: empty input **keeps** the existing token. Print `kept` (dim), do not rotate remote clients by accident.
- Rotate only when the operator types a new value (length ≥ 16) or the exact word `rotate` (generate a fresh token and store it).
- `forge tokens set mcp.bind_token VALUE` also rotates.

After a rotate, every remote client must be updated. Print `mcp: rotated bind_token` (never the value).

## Authorization

Every request, including localhost, carries:

```
Authorization: Bearer <mcp.bind_token>
```

Mismatch → HTTP 401, no tool dispatch.

Compare tokens in constant time. Do not log the bearer.

Remote clients (Grok, Hermes on another host) use the **tunnel URL** + the same bearer. They never receive `ngrok.auth_token`.

A stolen `mcp.bind_token` is factory intake: the holder can `queue_work` and `bind_repo`. That is accepted for v0. Rotate the bearer to cut them off.

## Factory event log

Project `events.jsonl` stays per-project (SPEC.md §7).

Factory-level lines go to `<ROOT>/events.jsonl` (same line cap, same redaction). Hook name: `mcp`.

| event | where | fields |
|---|---|---|
| bearer missing / wrong | factory `events.jsonl` | `hook=mcp` `status=denied` `step=auth`. No token, no Authorization header, no tool arguments that look like secrets |
| tool dispatched | factory `events.jsonl` | `hook=mcp` `status=ok` `step=<tool>` `project=<name-or-empty>` |
| tool that mutates a project (`queue_work`, `bind_repo`, `spec_refresh`) | **also** that project's `events.jsonl` | same hook; intake/bind already append their own lines through core |

`project` on a factory line may be empty. Do not invent a project to have somewhere to write.

## Tools in v0

Exactly these. No `forge run`, no merge, no SSH, no token values.

| tool | maps to | notes |
|---|---|---|
| `factory_status` | factory block below | not `fy status P` |
| `project_status` | `fy status P` | unknown project → precondition; bytes equal `fy status P` |
| `list_projects` | bound project list | names + spec/step only |
| `queue_work` | `fy do URL TEXT` | same intake rules |
| `bind_repo` | `fy bind URL` | |
| `tail_events` | hook lines | `project` required for project log; omit project → factory log. Default N=50, cap 200, newest last |
| `last_outcome` | latest `runs/*/outcome.json` | **project required**. No raw stdout dump by default |
| `spec_refresh` | `forge spec-refresh P` | |
| `factory_health` | pids + runner present + tunnel up | no secrets |

`factory_status` byte-stable block:

```
forgeyard
watch:     up|down
mcp:       up|down
yard:      up|down|off
tunnel:    up|down|off
runner:    present|missing
projects:  <n>
```

`start` / `stop` are **not** MCP tools in v0 (the client that kills mcp would kill itself). Use `fy start` / `fy stop` on the laptop.

Tool timeout cap: 15s. `spec_refresh` and `bind_repo` may use 30s (GitHub network). No `wait_until_merged`. Clients poll `project_status`.

## Protocol version

`factory_health` includes `protocol=forgeyard-mcp/1` and the fy version. Unknown tools → MCP error, not a guessed alias.

Transport is MCP Streamable HTTP. The implementation PR names the crate and freezes initialize / tools-list / call fixtures. This file does not re-specify the MCP handshake.

When watch/core grow new status fields, mcp forwards them. It does not re-derive workflow state.

## Never

- print PAT, LLM keys, SSH password, ngrok auth token, mcp bind token
- write `state.json` / `plan.json` except through existing core helpers
- expose raw `tokens.toml`
- treat Pi exit 0 as success (same as watch: only `outcome.json`)

## Test contract

No live ngrok, no live Telegram.

- bearer missing / wrong → 401 fixture + factory `hook=mcp status=denied`
- empty token at `fy start` → token generated, mcp attempted, watch still started
- re-onboard empty step 7 with token already set → token bytes unchanged
- `queue_work` writes the same inbox + event as `fy do`
- `project_status` bytes equal `fy status P`
- tools cannot call `gh pr merge`
