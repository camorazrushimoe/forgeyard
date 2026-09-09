# Dev cluster

The laptop runs the factory. The SSH host runs the product.

Watch states do not change. This file says **where** `implement` and `qa-on-cluster` execute.

## Two planes

| plane | machine | lives here |
|---|---|---|
| control | Mac (`fy`, `watch`, `yard`) | tokens, `events.jsonl`, `state.json`, `plan.json`, spec copies |
| work | SSH host from `cluster.*` | git clone, feature branches, process that serves the app, tests |

`main` on GitHub is the only blessed line. The clone on the host is a workbench. After watch merges, the host may `git fetch` + update `main`. That is housekeeping, not the merge itself.

Watch never SSHes. Developer and QA sessions do.

## Login

From onboard / `tokens.toml`:

```toml
[cluster]
host = "203.0.113.10"
user = "deploy"
port = 22
password = ""
```

Wrapper builds `user@host -p port`. Password stays in that file. Not in the event log.

Missing cluster config → `hook stop` `fail` `summary=cluster_missing`. Do not start Pi on the Mac as a fallback for implement/QA.

## Layout on the host

One clone per GitHub repo name (project name):

```text
/srv/forgeyard/<project>/repo/
```

Example: project `my-app` → `/srv/forgeyard/my-app/repo`.

First `implement` on an empty path:

1. `mkdir -p /srv/forgeyard/<project>`
2. `git clone` the bound GitHub repo into `repo/`
3. `git checkout -b feature/<issue>-<slug>` (or `fix/` for E)

No extra human approve to create that tree. Spec already passed A.

One working tree in v0. Busy-lock on the project means Dev and QA never edit it at the same time.

## What each role does on the host

### developer / implement

Cwd: `/srv/forgeyard/<project>/repo`.

- create or reuse the ticket branch
- edit files, run the app, run local tests **on this host**
- `git push -u` the branch
- open or update the PR into `main`
- do not merge, do not pretend the Mac checkout is source of truth

Unpushed bytes do not exist for QA or watch.

### qa / qa-on-cluster

Same host, same clone. After Dev `stop` + open PR:

1. `git fetch`
2. checkout the PR head sha (detached or the branch)
3. run/restart whatever this project uses to exercise that sha
4. comment on the PR: `Verdict: merge` or `Verdict: no-merge` plus the sha

QA does not clone a second product. QA does not test Dev's dirty working tree. QA tests the sha GitHub already has.

### tech-pm

Stays on the laptop project dir (`events`, `spec.md`, GitHub). No SSH required for A/B/E triage.

## How the wrapper starts Pi for cluster steps

Normative idea (exact flags later):

```text
ssh user@host → cd /srv/forgeyard/<project>/repo → pi …
```

or equivalent: wrapper SSHes each bash/file action. Cwd on the host is the clone, not `~/.forgeyard`.

Laptop `projects/<repo>/workspace/` is optional cache. It is not what QA gates.

## After merge

Watch merges on GitHub. Optional next command on the host: fetch `main` and check it out so the workbench matches blessed history. Not a watch state. Not a deploy-to-prod.

## Illegal

- implement or QA with cwd on the Mac when cluster is configured
- QA testing files that were never pushed
- second SSH host per role in v0
- merge on the host instead of GitHub
