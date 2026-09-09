# Pi runner

Pi ([pi.dev](https://pi.dev)) is the factory runner. It is **not** compiled into `forge`.
It is an install-time dependency: one CLI that can read, write, edit files and run bash.

Forgeyard uses **one** Pi per run. There are no long-lived per-role agents.
Identity for the run is `forge envelope` + `roles/<role>.md` + the skills listed in `pack.toml` for that step.

## Why Pi, not Hermes, not a raw LLM

| | raw cloud LLM | Hermes crew | Pi |
|---|---|---|---|
| bash / files | no | yes | yes |
| one-shot CLI | maybe | no (daemons + doors) | `pi -p` |
| process to wrap | n/a | 3 containers | 1 process |
| roles | prompt only | 3 homes + 3 keys | files |

Hermes stays out of this factory. Doors, idle-stop, Redis bus stay out.

## Who starts Pi

Only the wrapper (`forge run` now, `crew` later):

```sh
cd "$PROJECT_DIR"          # projects/<repo>/workspace or a checkout
RUN_ID=$(forge hook start --project "$P" --agent "$ROLE" --step "$STEP")
export CUSTOM_API_KEY=$(forge tokens export --name llm.default)
# map to whatever provider Pi expects, e.g. OPENROUTER_API_KEY / ANTHROPIC_API_KEY
set +e
pi -p --mode json "$PROMPT"
rc=$?
set -e
forge hook stop --project "$P" --agent "$ROLE" --run-id "$RUN_ID" --status "$ST"
```

`$PROMPT` is stdout of:

```
forge envelope --project P --agent ROLE --step STEP --run-id ID
```

Envelope concatenates, in this order:

1. kernel prefix from SPEC.md §8 (PROJECT, REPO, AGENT, STEP, RUN_ID, SPEC path, RULES)
2. `roles/<role>.md`
3. each `skills/<name>/SKILL.md` listed for this step in `pack.toml`
4. the task text (stdin to envelope)

Pi also sees `AGENTS.md` in the project checkout if present. Envelope rules win over AGENTS.md when they conflict.

## Working directory

Pi cwd is the **project checkout**, not `FORGEYARD_ROOT`.

Recommended layout after bind:

```
<ROOT>/projects/<repo>/
  PROJECT.toml state.json events.jsonl spec.md
  workspace/          # git clone of owner/repo  (optional in v0)
```

`forge run` should `chdir` to `workspace/` if it exists, else to the project dir.
Pi must not be started with cwd = `<ROOT>` (tokens live there).

## Isolation (v0, honest)

Pi's bash can reach the whole user account if the OS allows it. v0 rules:

- cwd is the project tree
- envelope forbids touching `<ROOT>/tokens/`
- envelope forbids `git push` to `main`/`master`
- one project busy lock already exists

A container/sandbox around Pi is a later hardening step, not part of the first install.

## Session policy

Each `forge run` is a **new** Pi session. Do not pass `-c` (continue).
Previous chat is not context. Context is spec.md, git, GitHub, events.jsonl, envelope.

`--name "forgeyard:$PROJECT:$RUN_ID"` so sessions are grep-able.

## Provider / token

Pi is multi-provider. Forgeyard stores one logical key: `llm.default` (see spec/tokens.md).
The wrapper exports the env var Pi needs. v0: operator sets `PI_PROVIDER_ENV` in factory.toml, default `OPENROUTER_API_KEY`.

Role-specific keys (`llm.tech-pm`, …) are optional overrides. If unset, use `llm.default`.

## Failure

| event | hook stop status |
|---|---|
| pi exit 0 | ok |
| pi exit != 0 | crash |
| pi binary missing | fail, summary=`runner_missing` |
| token empty | fail, summary=`token_missing` — do not leave start without stop |

## factory.toml snippet

```toml
[runner]
kind = "pi"
bin = "pi"                 # on PATH after install
print_flag = "-p"
mode = "json"
provider_env = "OPENROUTER_API_KEY"
```
