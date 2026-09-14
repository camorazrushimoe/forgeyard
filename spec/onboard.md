# Onboard

After `curl … | sh` the installer prints `next: fy onboard`.
`fy onboard` asks for secrets one field at a time. Tokens are not echoed.
Failed check → repeat that field, do not restart the whole wizard.

v0 target: Apple Silicon Mac. Writes `factory/tokens/tokens.toml` mode 0600.

## Terminal UX

The wizard is still a TTY (no TUI crate required in v0). It must not look like a raw dump of prompts.

Rules:

- Number every field as `n / N` (example: `3 / 9`). N is the total step count in this wizard version.
- Print a blank line plus a separator line before each step (`─` × 40, or ASCII `-` if the locale is not UTF-8).
- Color is optional and must degrade: if stdout is not a TTY or `NO_COLOR` / `TERM=dumb` is set, print plain text.
- Palette when color is on (ANSI, no extra deps):
  - step header / number — bold cyan
  - why / hint / example — dim
  - ok after a check — green
  - retry after a failed check — yellow
  - fatal / refused — red
  - generated values notice (`generated mcp.bind_token`) — green, never the value
- Secrets stay un-echoed even with color. Do not color the hidden input itself.
- After a successful field: one short `ok` line, then the next separator. Do not reprint the previous secret.
- Skip (empty optional field): `skipped` in dim, not an error color.
- Done banner stays one block, green + bold if color is on:

```
ready.
  start: fy start
```

Example shape (color omitted here):

```
────────────────────────────────────────
3 / 9  LLM token
why: OpenAI-compatible key for Pi
check: GET {endpoint}/models

llm.default:
ok
```

Do not animate, do not clear the whole screen, do not depend on a terminal graphics library.
Tests may set `NO_COLOR=1` and assert the numbered headers + separators exist.

## Steps

1. Telegram bot token  
   Why: yard panel. Empty allowed → skip yard. Check: Telegram `getMe` if non-empty.

2. LLM endpoint  
   OpenAI-compatible base URL. Example printed in the prompt:  
   `https://openrouter.ai/api/v1`  
   `https://api.openai.com/v1`

3. LLM token  
   Stored as `llm.default`. Check: `GET {endpoint}/models` with the token.

4. GitHub PAT  
   Needs `repo` (issues, PRs, merge). Check: `GET /user`.

5. Dev cluster SSH host  
   One string: `user@host` or `user@host:port`.  
   Example: `deploy@203.0.113.10` or `deploy@dev.example.com:22`.  
   Stored as `cluster.user`, `cluster.host`, `cluster.port` (default 22).

6. Dev cluster SSH password  
   Hidden input. Stored as `cluster.password`.  
   Check: TCP + SSH auth with that user/host/password, timeout ~10s.  
   Do not log the password. Do not print it back.

7. MCP bind token  
   Why: every MCP request must present this bearer (local and remote).  
   Empty → generate 32 random bytes hex and store as `mcp.bind_token`.  
   Check if typed: length ≥ 16. Never echo.  
   See [mcp.md](mcp.md).

8. ngrok reserved URL (optional)  
   Empty → no public tunnel. Example: `https://your-name.ngrok.app`  
   Stored as `ngrok.url`. Check if non-empty: `https://` + host.  
   This is an address, not a password. See [tunnel.md](tunnel.md).

9. ngrok auth token (optional)  
   Empty allowed. Stored as `ngrok.auth_token`.  
   Needed only if step 8 is set and `fy start` should launch ngrok.  
   Never echo. Never give this value to MCP clients.

Done banner: `ready.  start: fy start`

Re-run `fy onboard` to rotate any of the fields.

## Who reads cluster SSH

QA (`qa-on-cluster`) and, if a session needs a running service, developer.  
Watch does not SSH. It only requires that QA's `hook stop` happened.

How the remote tree is laid out per project is still later. This file only stores how to log in.
