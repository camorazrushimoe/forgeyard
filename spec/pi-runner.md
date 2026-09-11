# Pi runner

Pi ([pi.dev](https://pi.dev)) runs **on the laptop** where the factory was installed.
It is not compiled into `forge`. It is not installed on the SSH host.

One Pi process per `forge run`. Role = envelope + `roles/<role>.md` + skills. The wrapper captures its bounded output and turns workflow decisions into validated local artifacts; it never treats `pi` exit 0 as a decision.

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
pi -p --mode json "$PROMPT" >"$RUN_DIR/stdout.jsonl" 2>"$RUN_DIR/stderr.log"
rc=$?
set -e

# outcome validate writes outcome.json only when the role/step/schema/current-spec
# contract is valid. provider_classify inspects the bounded diagnostics without
# exposing a token and may return e.g. llm_connect, llm_http_401, llm_protocol.
if [ "$rc" -ne 0 ]; then
  ST=crash
  SUMMARY="pi_exit_$rc"
elif PROVIDER_FAILURE=$(forge provider-classify --stderr "$RUN_DIR/stderr.log"); then
  ST=fail
  SUMMARY="$PROVIDER_FAILURE"
elif forge outcome validate --project "$P" --run-id "$RUN_ID" --agent "$ROLE" \
       --step "$STEP" --stdout "$RUN_DIR/stdout.jsonl" --exit-code "$rc"; then
  ST=ok
  SUMMARY=ok
else
  ST=fail
  SUMMARY=outcome_invalid
fi
forge hook stop --project "$P" --agent "$ROLE" --run-id "$RUN_ID" \
  --status "$ST" --summary "$SUMMARY"
```

`$PROMPT` = envelope + role + skills + task (including intake text). `request.json` records the role, step, effective endpoint, selected model (when the provider exposes one), and start time, but never an API key or raw prompt.

## Run artifacts and outcome contract

Each run has `projects/<repo>/runs/<run-id>/`:

| file | purpose |
|---|---|
| `request.json` | redacted execution metadata: role, step, endpoint origin/host, model, timestamps |
| `stdout.jsonl` | bounded raw Pi JSON-mode output for diagnosis |
| `stderr.log` | bounded Pi stderr for diagnosis |
| `outcome.json` | schema-validated workflow result consumed by watch |

The wrapper retains enough output to diagnose a provider failure, including API status/error class when Pi exposes it, while redacting tokens and truncating oversized streams with an explicit marker.

Required `outcome.json` values:

- tech-pm `review`: `{ "kind": "spec_review", "spec_sha256": "…", "verdict": "approve" | "needs_changes", "summary": "…" }`;
- developer `implement`: `{ "kind": "implementation", "branch": "…", "summary": "…" }`;
- QA: `{ "kind": "qa", "pr": <number>, "verdict": "merge" | "no_merge", "summary": "…" }`.

The role prompt must require one machine-readable outcome in its final response. `forge outcome validate` rejects missing, malformed, wrong-role/step, or stale-spec outcomes. It records `outcome_invalid` and the run ends as `fail`, rather than `ok`.

For QA, `outcome.json` is the **only workflow source of truth**. After validation, the wrapper publishes the same verdict and summary as a PR comment in the canonical form `Verdict: merge` or `Verdict: no-merge`, with the run ID. That comment is a human-visible audit trail and must not be parsed by watch to decide a transition. If publishing the comment fails, the QA run is `fail qa_comment_publish`; no merge occurs until a later valid QA run both stores its outcome and publishes its comment.

Watch reads only validated `outcome.json` and GitHub facts. It never infers approval, a branch, a PR, or a QA result from process exit status or arbitrary model prose.

## Session policy

New Pi session every run. No `-c`.

## Failure

| event | hook stop |
|---|---|
| pi exit 0 + valid outcome | ok |
| pi exit 0 + missing/invalid outcome | fail `outcome_invalid` |
| pi exit != 0 | crash (preserve bounded diagnostics) |
| Pi/provider API error | fail with classified summary (for example `llm_http_401`, `llm_connect`, `llm_protocol`) |
| pi missing on laptop | fail `runner_missing` |
| cluster missing on implement/QA | fail `cluster_missing` |
