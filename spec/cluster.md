# Dev cluster

The laptop runs the factory **and Pi**. The SSH host is only a workbench.
Pi does application work on that host, not on the laptop disk.

Watch states do not change.

## Two planes

| plane | machine | lives here |
|---|---|---|
| control | Mac (`fy`, `watch`, `yard`, **`pi`**) | tokens, events, plan, envelope, LLM calls |
| work | any SSH host (Linux or Mac) | git clone, branches, running app, tests |

Pi is never installed on the SSH host in v0. Pi on the laptop uses SSH as a tool.

## Login (v0 = password)

Onboard stores:

```toml
[cluster]
host = "203.0.113.10"   # or a DNS name
user = "deploy"
port = 22
password = ""
```

That is enough: address + username + password. The host OS can be Linux or macOS.

The wrapper, not the model, feeds the password into SSH (sshpass / expect / `SSH_ASKPASS`). Pi only sees `ssh user@host …` working. Password is never printed and never written to `events.jsonl`.

SSH keys are optional later. v0 does not require a key in `~/.ssh`.

Missing cluster config on implement/QA → `hook stop` `fail` `summary=cluster_missing`.
Do not fall back to editing the app on the laptop.

## Layout on the host

```text
/srv/forgeyard/<project>/repo/
```

First implement may `ssh … git clone` into that path.

## Roles

Developer and QA: Pi on the laptop, all file/run/test via SSH into that clone.
Tech-pm: laptop only.

Unpushed bytes do not exist for QA or watch.
