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

## Start behaviour

If `ngrok.auth_token` and `ngrok.url` are both set:

1. mcp already listening on `127.0.0.1:18789`
2. `fy start` launches `ngrok http 18789 --url <host-from-ngrok.url>` (or equivalent current ngrok CLI)
3. banner prints `mcp: local http://127.0.0.1:18789/mcp` and `mcp: public <ngrok.url>/mcp`
4. missing `ngrok` binary → print `tunnel: missing; install ngrok` and continue; factory + local MCP still run

If only URL is set and auth token is empty: do not start a tunnel; print `tunnel: skipped (no ngrok.auth_token)`.
If only auth token is set: do not guess a URL; print `tunnel: skipped (no ngrok.url)`.

`fy stop` stops the ngrok child too.

Watch does not start ngrok. Yard does not start ngrok.

## Onboard (optional tail of the wizard)

After the existing six fields, `fy onboard` asks:

7. MCP bind token  
   Empty → generate and store. Non-empty → store as given. Never echo. Check: length ≥ 16.

8. ngrok reserved URL  
   Empty → skip tunnel. Example: `https://your-name.ngrok.app`  
   Check if non-empty: must be `https://` host, no path required (mcp appends `/mcp`).

9. ngrok auth token  
   Empty allowed if step 8 was empty. If step 8 was set, empty is allowed but start will skip the tunnel.  
   Check if non-empty: non-blank, do not call ngrok API in v0 (network optional).  
   Never echo. Never write to events.

Re-run onboard to rotate any of the three.

## Safety

- MCP never binds public interfaces itself.
- Tunnel child is one process, one port: MCP only. Not SSH, not GitHub.
- Denied MCP requests do not include tool args in logs if they look like tokens.
- `fy status` / Telegram `/status` may say `tunnel=up|down|off`, never the auth token, bind token tail only in `/tokens` meta (`..xxxx`) like other secrets.

## Client checklist (operator)

On the laptop: `fy onboard` (optional 7–9) → `fy start`.
On the remote agent: URL = `ngrok.url` + `/mcp`, header bearer = `mcp.bind_token`.
Rotate bind token → update every remote client. Rotate ngrok auth token → only the laptop.
