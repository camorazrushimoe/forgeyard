# yard — Telegram control panel

`yard` is a separate long-running binary. It is a deterministic control surface over the files `forge` owns.

It does not think. It does not call an LLM. It maps a small command set onto `forgeyard-core` reads and a few explicit writes (token rotation).

## Why a second binary

| | `forge` | `yard` |
|---|---|---|
| lifetime | milliseconds to seconds | days |
| interface | argv / stdout / exit code | Telegram |
| failure | must not depend on Telegram | may restart without touching agent wrappers |
| privilege | called by hooks as root-of-run | talks to the public Internet |

Putting the poller inside `forge` would make every hook binary a network service. That is the opposite of a reliable wrapper.

Shared code: `forgeyard-core`. Shared files: `state.json`, `events.jsonl`, `tokens/tokens.toml`.

## Process model

```
yard --root /var/lib/forgeyard --config /etc/forgeyard/yard.toml
```

`yard` long-polls `getUpdates`. One update at a time, in arrival order. No parallel command handlers in v0.
On handler error it replies with a fixed error line and continues. It does not crash the process on a single bad update.

Telegram bot token comes from `tokens/tokens.toml` `[telegram].bot_token` or `TELEGRAM_BOT_TOKEN`. Config does not store the token in git.

## Authorization

Allowlist only.

```toml
# tokens/tokens.toml
[telegram]
bot_token = "..."
allow_user_ids = [123456789]
```

If `from.id` is not in `allow_user_ids`:

- do not reply (v0 default), or reply `denied` if `yard.toml` sets `reply_denied = true`
- append a log line at factory root: `hook=panel status=denied` with Telegram user id, never the message text if it looks like a token

No group chats in v0. If `chat.type != private`, ignore.

## Commands in v0

Exactly these. Unknown text → fixed reply `unknown command. /status /who /projects /tokens /help`.

### /help

```
forgeyard panel
/status     factory or one project
/who        who is busy right now
/projects   list bound projects
/tokens     list token names (no values)
/settoken   rotate an LLM token (interactive)
```

### /status

No argument: factory-wide block from SPEC.md §11 (multi-project one-liner list, plus a header).

`/status <project>`: the exact single-project block from SPEC.md §11.

This is the one message that must exist on day one. If everything else slips, `/status` stays.

The bot must not rephrase the block. Byte-stable formatting, same function as `forge status`.

### /who

Projects where `agent_status=busy`, one line each:

```
my-app  tech-pm  review_in_progress  20260909T080012Z-ab12
```

If none: `idle: no agents running`.

### /projects

```
my-app     bound     spec=yes  step=review_in_progress
```

### /tokens

See `spec/tokens.md`. Prints a table of names. Never values.

```
name                 set  tail  updated
llm.default          yes  ..a1  2026-09-09T08:10:00Z
llm.tech-pm          no   -     -
telegram.bot_token   yes  ..9c  2026-09-01T00:00:00Z
```

### /settoken `<name>`

Starts a two-step exchange. Deterministic state machine in memory, not an LLM.

1. User sends `/settoken llm.tech-pm`.
2. Bot replies: `send the new value as the next message, or /cancel. the value will be deleted from chat after save.`
3. Next private message from the same user is treated as the secret.
4. Bot writes via the same function as `forge tokens set --name llm.tech-pm --from-stdin`.
5. Bot deletes the user's message (`deleteMessage`) if it has permission.
6. Bot replies: `set llm.tech-pm tail=..a1` (last 4 only).
7. Append `hook=token_set` with `token_name`, never the value.

Rules:

- only names from the allowlist in `spec/tokens.md`
- timeout 2 minutes → `expired, not set`
- `/cancel` aborts
- a second `/settoken` replaces the pending one
- if write fails, reply `token not set` and keep the previous value

`telegram.bot_token` may be rotated the same way. After save, `yard` reconnects on the next poll using the new token.

## What yard does not do in v0

- start or stop Hermes agents
- bind GitHub URLs (that is `forge bind`; may be added as `/bind <url>` later)
- edit `spec.md`
- merge PRs
- run prompts
- accept commands from anyone not on the allowlist

`/bind` and `/run spec-review` are reserved names. Do not implement them until `crew` exists.

## yard.toml

```toml
root = "/var/lib/forgeyard"
reply_denied = false
# optional sticky status message in one chat
# status_chat_id = 123456789
# status_message_id = 0   # written back after first send
```

### Sticky status (optional, v0.1)

If `status_chat_id` is set, `yard` keeps **one** Telegram message edited in place.
Whenever `state.json` of any project changes (mtime or hook), edit that message to the factory-wide `/status` text.

This is the preferred "one message about current state": not a flood of chats, one panel card.
If edit fails with message-not-found, send a new message and store `status_message_id` in `yard.toml` or `factory/panel-state.toml`.

v0 can ship without sticky edit. `/status` on demand is enough.

## Logging

Panel actions append to a factory-level log if no project is in scope:

`<ROOT>/panel-events.jsonl`

Same JSONL rules as project logs. `project` may be `"-"`.

Project-scoped commands (`/status my-app`) also append a short `hook=panel` line to that project's `events.jsonl`.

## Failure behaviour

| event | behaviour |
|---|---|
| Telegram API down | retry poll with linear backoff 1s, 2s, 5s, 10s cap |
| `tokens.toml` missing | exit 5 |
| allowlist empty | exit 2 (refuse to start as a public bot) |
| `state.json` missing for `/status P` | reply `unknown project: P` |
| lock contention | retry 3 times, then `status unavailable, retry` |

## Test contract

Determinism is testable without Telegram:

```
forge status --project my-app > a.txt
# yard_render_status("my-app") == a.txt
```

Golden tests in the Rust crate must pin the exact status block.
