# Dev cluster

The laptop runs the factory **and Pi**. The SSH host is only a workbench.

Watch states do not change. This file says where files live and how Pi reaches them.

## Two planes

| plane | machine | lives here |
|---|---|---|
| control | Mac (`fy`, `watch`, `yard`, **`pi`**) | tokens, events, plan, envelope, LLM calls |
| work | SSH host from `cluster.*` | git clone, branches, running app, tests |

Pi is **never** installed on the SSH host in v0. Pi on the laptop uses SSH as a tool: `ssh user@host '…'` to clone, edit via scp/`ssh`, run tests, start processes. The model process stays on the machine where the factory was installed.

`main` on GitHub is the only blessed line. Watch never SSHes. Developer/QA sessions do, through Pi on the laptop.

## Login

From onboard / `tokens.toml`:

```toml
[cluster]
host = "203.0.113.10"
user = "deploy"
port = 22
password = ""
```

Wrapper injects host/user/port into the session (env `FORGEYARD_SSH=user@host:port`). Password is available to the wrapper for `sshpass`/`ssh`, not printed.

Missing cluster config on implement/QA → `hook stop` `fail` `summary=cluster_missing`.
Do not skip SSH and work only on the Mac for those steps.

## Layout on the host

```text
/srv/forgeyard/<project>/repo/
```

First implement may `ssh … git clone` into that path. No extra approve after spec A.

One working tree in v0. Busy-lock → Dev and QA never edit it at the same time.

## What each role does

### developer / implement

Pi on the laptop. Remote cwd via SSH: `/srv/forgeyard/<project>/repo`.

- create or reuse the ticket branch on the host
- edit/run/test **on the host** (ssh)
- `git push` the branch (auth still an open spec item)
- open or update the PR into `main`
- do not merge

Unpushed bytes do not exist for QA or watch.

### qa / qa-on-cluster

Same host, same clone. After Dev `stop` + open PR:

1. ssh: `git fetch` + checkout PR head sha
2. run checks on that sha
3. comment `Verdict: merge` or `Verdict: no-merge`

### tech-pm

Laptop only. No SSH.

## Illegal

- installing or expecting `pi` on the SSH host
- implement/QA with no SSH when cluster is configured
- QA testing unpushed files
- merge on the host instead of GitHub
