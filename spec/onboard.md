# Onboard

After `curl … | sh` the installer prints `next: fy onboard`.
`fy onboard` asks for secrets one field at a time. Tokens are not echoed.
Failed check → repeat that field, do not restart the whole wizard.

v0 target: Apple Silicon Mac. Writes `factory/tokens/tokens.toml` mode 0600.

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

Done banner: `ready.  start: fy start`

Re-run `fy onboard` to rotate any of the fields.

## Who reads cluster SSH

QA (`qa-on-cluster`) and, if a session needs a running service, developer.  
Watch does not SSH. It only requires that QA's `hook stop` happened.

How the remote tree is laid out per project is still later. This file only stores how to log in.
