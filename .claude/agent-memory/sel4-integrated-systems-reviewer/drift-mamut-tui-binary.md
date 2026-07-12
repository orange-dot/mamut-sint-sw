---
name: drift-mamut-tui-binary
description: mamut-tui is a lib+bin crate that repo docs keep under-describing; recurring doc/code drift trap for binary counts and workspace-crate rosters
metadata:
  type: project
---

`crates/mamut-tui` ships BOTH a `src/lib.rs` (exports `run_tui_session`, consumed
by `mamut-standalone`) AND a `src/main.rs` with its own `fn main()` + `play` CLI.
So it is a runnable binary target (`cargo run -p mamut-tui -- play ...`), not just
a UI library.

The workspace therefore has **three** binaries with a `main`: `mamut-standalone`,
`mamut-seq`, `mamut-tui`. (`find crates -name main.rs` is the quick oracle.)

Drift trap seen in the SET3 review (2026-07-06):
- `CLAUDE.md` claimed "two binaries with a `main`" — omits mamut-tui.
- `README.md` "Workspace crates:" list enumerates 8 of 10 members — omits
  mamut-tui AND mamut-runtime. (CLAUDE.md DAG and AGENTS.md list all 10.)

**Why:** the architecture narrative treats mamut-tui as a library `mamut-standalone`
composes, so its bin surface gets forgotten.

**How to apply:** when a change edits binary counts or crate rosters in
CLAUDE.md/README/AGENTS, verify against `find crates -name main.rs` and the
`[workspace] members` list. Flag any roster that silently drops mamut-tui or
mamut-runtime.
