# Human CLI: `fy`

The person types **`fy`**, not `forge` / `yard` / `watch`.

```
fy help
fy onboard
fy start
fy stop
fy status
fy bind <github-url>
fy do <github-url> <prompt...>
```

`fy help` and `fy --help` print the same list. Unknown argv → help + exit 2.

## fy help

```
fy — forgeyard

  fy help              this text
  fy onboard           telegram, llm, github, ssh cluster
  fy start             run watch + yard, print the event log
  fy stop              stop a running fy start
  fy status            same block as forge status
  fy bind URL          attach a GitHub repo / issue / PR
  fy do URL TEXT       queue work (see spec/intake.md)
```
