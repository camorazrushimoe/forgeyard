# Human CLI: `fy`

The person types **`fy`**, not `forge` / `yard` / `watch`.
Those stay internal binaries. `fy` is the only name in the install banner.

```
fy help
fy onboard
fy start
fy stop
fy status
fy bind <github-url>
```

`fy help` and `fy --help` print the same list plus one-line meanings. Unknown argv → that help text and exit 2.

## fy help

```
fy — forgeyard

  fy help       this text
  fy onboard    telegram, llm, github, ssh cluster
  fy start      run watch + yard, print the event log
  fy stop       stop a running fy start
  fy status     same block as forge status
  fy bind URL   attach a GitHub repo / issue / PR
```

No hidden subcommands in v0. Internals (`forge hook`, …) are not in this list.
