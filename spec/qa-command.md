# QA command on the host

v0 contract: after checkout of the PR sha, run **one** command on the cluster:

```
make qa
```

in `/srv/forgeyard/<project>/repo`.

Missing Makefile / target → QA writes `Verdict: no-merge` and says `make qa` is missing. Watch does not invent another command.

Skills (smoke, later regression) may add checks **inside** that target or extra steps the QA role documents. They do not replace `make qa` as the thing watch/QA must invoke first.

A toy repo used to test the factory must ship `make qa` (even if it is `true` or a tiny script).
