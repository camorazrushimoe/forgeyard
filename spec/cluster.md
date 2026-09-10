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

The wrapper, not the model, feeds the password into SSH. Password is never printed and never written to `events.jsonl`.

Missing cluster config on implement/QA → `hook stop` `fail` `summary=cluster_missing`.
Do not fall back to editing the app on the laptop.

## Layout on the host

```text
/srv/forgeyard/<project>/repo/
```

First implement may `ssh … git clone` into that path. One-time bootstrap (run on the host, or pipe over ssh):

```sh
sh pack/bootstrap-host.sh toy https://github.com/YOU/toy.git
```

`forge run` sets `FORGEYARD_REMOTE=/srv/forgeyard/<project>/repo` and `FORGEYARD_SSH=user@host:port` for Pi. Watch does not SSH.

## Roles

Developer and QA: Pi on the laptop, all file/run/test via SSH into that clone.
Tech-pm: laptop only.
QA command on that tree: `make qa` (see qa-command.md).

Unpushed bytes do not exist for QA or watch.
