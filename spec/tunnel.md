# Remote access (optional ngrok)

Default factory is laptop-local. MCP listens on `127.0.0.1` only.

A paid ngrok reserved domain is the v0 way to let **other machines' agents** reach that port. The tunnel is optional. Empty onboard fields → no tunnel, local MCP still works.

## Three values (do not mix them)

| name | who has it | what it does |
|---|---|---|
| `mcp.bind_token` | factory **and** every MCP client | Bearer on every MCP request |
| `ngrok.auth_token` | factory only | lets `ngrok` attach this laptop to the ngrok account |
| `ngrok.url` | public | reserved HTTPS origin, e.g. `https://factory.ngrok.app` |

The remote agent config is:

```
MCP URL:  https://<reserved-domain>/mcp
Header:   Authorization: Bearer <mcp.bind_token>
```

That is the whole client setup. The agent must not get `ngrok.auth_token`.

`ngrok.url` is not a secret. It is also not authentication. Anyone who finds the URL still needs the bearer.

## What you type at onboard

Canonical wizard text lives in [onboard.md](onboard.md) steps 7–9. This file only states the meaning of the three values.

- reserved URL (`ngrok.url`) and ngrok account token (`ngrok.auth_token`) are both optional.
- Both empty → local MCP only.
- URL without token (or token without URL) → tunnel skipped at `fy start`.
- Step 7 is the MCP bearer. Generate / keep / rotate rules are in [mcp.md](mcp.md) (empty on re-run keeps an existing token).

## Start behaviour

If `ngrok.auth_token` and `ngrok.url` are both set:

1. mcp already listening on `127.0.0.1:<port>` (port rules in [mcp.md](mcp.md))
2. `fy start` launches `ngrok http <port> --url <host-from-ngrok.url>` (or equivalent current ngrok CLI)
3. banner prints `mcp: local http://127.0.0.1:<port>/mcp` and `mcp: public <ngrok.url>/mcp`
4. missing `ngrok` binary → print `tunnel: missing; install ngrok` and continue; factory + local MCP still run

If only URL is set and auth token is empty: do not start a tunnel; print `tunnel: skipped (no ngrok.auth_token)`.
If only auth token is set: do not guess a URL; print `tunnel: skipped (no ngrok.url)`.

`fy stop` stops the ngrok child too. Pid file: `<ROOT>/ngrok.pid`.

Watch does not start ngrok. Yard does not start ngrok.

## Safety

- MCP never binds public interfaces itself.
- Tunnel child is one process, one port: MCP only. Not SSH, not GitHub.
- Denied MCP requests do not include tool args in logs if they look like tokens.
- `fy status` / Telegram `/status` / `factory_status` may say `tunnel=up|down|off`, never the auth token, bind token tail only in tokens meta (`..xxxx`) like other secrets.

## Client checklist (operator)

On the laptop: `fy onboard` (optional 7–9) → `fy start`.
On the remote agent: URL = `ngrok.url` + `/mcp`, header bearer = `mcp.bind_token`.
Rotate bind token → update every remote client. Rotate ngrok auth token → only the laptop.
