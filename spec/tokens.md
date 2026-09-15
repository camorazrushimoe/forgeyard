# Tokens

Secrets live in one file: `<ROOT>/tokens/tokens.toml`, mode `0600`.
The example committed to git is `pack/tokens.example.toml` (copied to `tokens/tokens.toml` on first install, never overwritten).

Nothing prints raw secrets: not `fy`, not `forge`, not `yard`, not `mcp`, not the event log.

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

[mcp]
bind_token = ""        # Bearer for MCP; generated if empty at first onboard / fy start
port = 18789

[ngrok]
url = ""               # reserved https origin; not a secret
auth_token = ""        # ngrok account token; factory only
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
mcp.bind_token
mcp.port
ngrok.url
ngrok.auth_token
```

Empty string means unset.

## Metadata sidecar

`<ROOT>/tokens/tokens.meta.toml` holds `tail` (last 4 of token-like values only), `updated_at`, `updated_by`.
Do not put a tail of `cluster.password`. For cluster, meta is `set=yes|no` plus `host` and `user` (not secret).
`ngrok.url` may appear in full in meta (not secret). `ngrok.auth_token` and `mcp.bind_token` get tails only.

## Never

- secret in `events.jsonl`, Telegram, `fy start` lines, `state.json`, git
- `ngrok.auth_token` sent to an MCP client
