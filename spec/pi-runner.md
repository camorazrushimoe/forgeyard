# Pi runner

Pi ([pi.dev](https://pi.dev)) is the factory runner. It is **not** compiled into `forge`.

Forgeyard uses **one** Pi per run. Role comes from files + envelope.

## Where Pi runs

| step | cwd |
|---|---|
| tech-pm (A, B, triage) | laptop `projects/<repo>/` |
| developer implement | SSH host `/srv/forgeyard/<project>/repo` |
| qa qa-on-cluster | same host path, after `git fetch` of the PR sha |

See [spec/cluster.md](cluster.md). Do not silently run implement/QA on the Mac if `cluster.host` is set.

## Who starts Pi

Only the wrapper (`forge run` / `fy` internals):

```sh
RUN_ID=$(forge hook start --project "$P" --agent "$ROLE" --step "$STEP")
# cluster steps: ssh then cd /srv/forgeyard/$P/repo
# laptop steps:  cd "$PROJECT_DIR"
export CUSTOM_API_KEY=$(forge tokens export --name llm.default)
set +e
pi -p --mode json "$PROMPT"
rc=$?
set -e
forge hook stop --project "$P" --agent "$ROLE" --run-id "$RUN_ID" --status "$ST"
```

`$PROMPT` is `forge envelope` then role file then skills then task.

## Session policy

Each `forge run` is a new Pi session. No `-c`. Context is spec, git, GitHub, events, envelope.

## Failure

| event | hook stop |
|---|---|
| pi exit 0 | ok |
| pi exit != 0 | crash |
| pi missing | fail `runner_missing` |
| token empty | fail `token_missing` |
| cluster missing on implement/QA | fail `cluster_missing` |

## factory.toml snippet

```toml
[runner]
kind = "pi"
bin = "pi"
print_flag = "-p"
mode = "json"
provider_env = "OPENROUTER_API_KEY"
```
