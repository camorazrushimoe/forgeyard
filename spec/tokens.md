# Tokens

Secrets live in one file: `<ROOT>/tokens/tokens.toml`, mode `0600`.
The example committed to git is `tokens/tokens.toml.example`.

Nothing prints raw secrets: not `fy`, not `forge`, not `yard`, not the event log.

## File schema

```toml
[telegram]
bot_token = ""
allow_user_ids = [123456789]

[llm]
endpoint = ""          # OpenAI-compatible base URL
default = ""           # API token
tech-pm = ""
developer = ""
qa = ""

[github]
pat = ""

[cluster]
host = ""              # hostname or IP
user = ""
port = 22
password = ""
```

Canonical names for `fy onboard` / `forge tokens set`:

```
telegram.bot_token
llm.endpoint
llm.default
llm.tech-pm
llm.developer
llm.qa
github.pat
cluster.host
cluster.user
cluster.port
cluster.password
```

Empty string means unset.

## Metadata sidecar

`<ROOT>/tokens/tokens.meta.toml` holds `tail` (last 4 of token-like values only), `updated_at`, `updated_by`.
Do not put a tail of `cluster.password`. For cluster, meta is `set=yes|no` plus `host` and `user` (not secret).

## Never

- secret in `events.jsonl`, Telegram, `fy start` lines, `state.json`, git
