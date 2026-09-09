# Pi runner

Pi ([pi.dev](https://pi.dev)) runs **on the laptop** where the factory was installed.
It is not compiled into `forge`. It is not installed on the SSH host.

One Pi process per `forge run`. Role = envelope + `roles/<role>.md` + skills.

## Where the process lives vs where files live

| step | Pi process | files / bash |
|---|---|---|
| tech-pm | laptop | laptop `projects/<repo>/` |
| developer implement | laptop | SSH `/srv/forgeyard/<project>/repo` |
| qa qa-on-cluster | laptop | same remote path, PR sha |

Envelope tells Pi the SSH target and the remote path. Pi reaches the host with ordinary `ssh`/`scp`.

## Who starts Pi

Only the wrapper:

```sh
cd "$PROJECT_DIR"    # laptop project dir; tokens stay out of cwd
RUN_ID=$(forge hook start --project "$P" --agent "$ROLE" --step "$STEP")
export CUSTOM_API_KEY=$(forge tokens export --name llm.default)
export FORGEYARD_SSH="$USER@$HOST:$PORT"
export FORGEYARD_REMOTE="/srv/forgeyard/$P/repo"
set +e
pi -p --mode json "$PROMPT"
rc=$?
set -e
forge hook stop --project "$P" --agent "$ROLE" --run-id "$RUN_ID" --status "$ST"
```

`$PROMPT` = envelope + role + skills + task (including intake text).

## Session policy

New Pi session every run. No `-c`.

## Failure

| event | hook stop |
|---|---|
| pi exit 0 | ok |
| pi exit != 0 | crash |
| pi missing on laptop | fail `runner_missing` |
| token empty | fail `token_missing` |
| cluster missing on implement/QA | fail `cluster_missing` |
