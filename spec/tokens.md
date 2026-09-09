# Tokens

Secrets live in one file: `<ROOT>/tokens/tokens.toml`, mode `0600`.
The example committed to git is `tokens/tokens.toml.example` and contains empty strings.

Neither `forge` nor `yard` ever prints a raw token. Telegram messages that contain a token are deleted after save when the bot has `can_delete_messages`.

## File schema

```toml
[telegram]
bot_token = ""
allow_user_ids = [123456789]

[llm]
default = ""
tech-pm = ""
software-engineer = ""
qa = ""
```

Canonical names (the only names `/settoken` and `forge tokens set` accept):

```
telegram.bot_token
llm.default
llm.tech-pm
llm.software-engineer
llm.qa
```

Unknown names → exit 2 / Telegram reply `unknown token name`.

Empty string means unset.

## Metadata sidecar

To show `tail` and `updated_at` without storing a second copy of the secret in logs, write:

`<ROOT>/tokens/tokens.meta.toml`

```toml
[llm.tech-pm]
tail = "a1b2"          # last 4 chars, or empty if unset
updated_at = "2026-09-09T08:10:00Z"
updated_by = "yard:123456789"   # or "forge-cli"
```

`tail` is not a secret. It is an operator check that the rotation landed.

## CLI

```
forge tokens list
forge tokens set   --name llm.tech-pm --from-stdin
forge tokens clear --name llm.tech-pm
```

`set` reads one line from stdin, trims surrounding whitespace, refuses empty, writes atomically (`*.tmp` + fsync + rename + fchmod 0600), updates meta, appends `hook=token_set`.

`clear` sets the value to `""`.

## How agents get a key

Out of scope for the kernel to launch Hermes. The intended contract:

- wrapper exports `CUSTOM_API_KEY` from `llm.<agent>` if set, else `llm.default`
- if both empty, wrapper does not start the agent and `hook start` is not left hanging (`hook stop --status fail --summary token_missing`)

`yard` does not inject env into running processes in v0. After `/settoken`, the **next** wrapper start sees the new value. A busy agent keeps the env it was born with until `hook stop`.

## What must never happen

- token value in `events.jsonl`
- token value in Telegram reply
- token value in `forge status`
- token value in `state.json`
- committing `tokens/tokens.toml`
