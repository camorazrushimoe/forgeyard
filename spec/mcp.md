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

Shared code: `forgeyard-core`. Shared files: `state.json`, `events.jsonl`, `tokens/tokens.toml`, run artifacts.

## Process model

`fy start` always starts `mcp` next to `watch` (and `yard` only if the telegram token is set).

Listen: `127.0.0.1` only. Default port `18789` (`FORGEYARD_MCP_PORT` overrides).

Transport v0: MCP Streamable HTTP at `http://127.0.0.1:18789/mcp`.
No bind to `0.0.0.0`. The public URL, if any, comes from the tunnel (see [tunnel.md](tunnel.md)).

Missing watch / factory root → tools still answer. Reads that need live state return a structured `factory_down` / `watch_down`. Writes to inbox are allowed (same as `fy do` while watch is stopped: queued, not executed).

`fy stop` stops mcp too.

## Authorization

Every request, including localhost, carries:

```
Authorization: Bearer <mcp.bind_token>
```

If `mcp.bind_token` is unset: mcp refuses to start (exit 5). Onboard always creates one (generate 32 random bytes, hex) so a default install is not an open socket.

Mismatch → HTTP 401, no tool dispatch, append factory `hook=mcp status=denied` without the token or the tool arguments if they look like secrets.

Compare tokens in constant time. Do not log the bearer.

Remote clients (Grok, Hermes on another host) use the **tunnel URL** + the same bearer. They never receive `ngrok.auth_token`.

## Tools in v0

Exactly these. No `forge run`, no merge, no SSH, no token values.

| tool | maps to | notes |
|---|---|---|
| `factory_status` | `fy status` | byte-stable block |
| `project_status` | `fy status P` | unknown project → precondition |
| `list_projects` | bound project list | names + spec/step only |
| `queue_work` | `fy do URL TEXT` | same intake rules |
| `bind_repo` | `fy bind URL` | |
| `tail_events` | hook lines from `events.jsonl` | cap N, newest last |
| `last_outcome` | latest `runs/*/outcome.json` | no raw stdout dump by default |
| `spec_refresh` | `forge spec-refresh P` | |
| `factory_health` | pids + runner present + tunnel up | no secrets |

`start` / `stop` are **not** MCP tools in v0 (the client that kills mcp would kill itself). Use `fy start` / `fy stop` on the laptop.

Tool timeout cap: 15s. No `wait_until_merged`. Clients poll `project_status`.

## Protocol version

`factory_health` includes `protocol=forgeyard-mcp/1` and the fy version. Unknown tools → MCP error, not a guessed alias.

When watch/core grow new status fields, mcp forwards them. It does not re-derive workflow state.

## Never

- print PAT, LLM keys, SSH password, ngrok auth token, mcp bind token
- write `state.json` / `plan.json` except through existing core helpers
- expose raw `tokens.toml`
- treat Pi exit 0 as success (same as watch: only `outcome.json`)

## Test contract

No live ngrok, no live Telegram.

- bearer missing / wrong → 401 fixture
- `queue_work` writes the same inbox + event as `fy do`
- `factory_status` bytes equal `fy status`
- tools cannot call `gh pr merge`
