---
name: drift-set4-range-enumeration
description: The SET4 backlog's slice range is enumerated in ≥3 index surfaces that drift when slices are appended
metadata:
  type: project
---

The `docs/EPM1_BACKLOG_SET4_MIDI2_UMP_EXPRESSIVENESS.md` slice range (`SET4-0..N`)
is hard-coded as a literal in at least three index surfaces:

- `CLAUDE.md` reading-order item 14 (`SET4-0..8` as of 2026-07-07)
- top-level `README.md` doc-index line (`SET4-0..8`)
- `docs/README.md` backlog entry (updated to mention the `SET4-9..12` track)

**Why:** the 2026-07-06 addendum appended SET4-9..12 (touch-surface track) and
updated only `docs/README.md`, leaving CLAUDE.md and the top-level README
advertising `SET4-0..8`. The backlog's own Shared Boundary Constraints name all
of CLAUDE.md/AGENTS.md/README.md/docs/README.md as must-match-in-same-change.

**How to apply:** whenever a SET4 slice is added/removed, grep for
`SET4-0\.\.` across CLAUDE.md and both READMEs and bump every literal range in
the same change. Also relevant: SET4-10 route A adds a `mamut-net-ump` binary,
which would make CLAUDE.md's "There are three binaries with a `main`" line
stale — see [[drift-mamut-tui-binary]].

Cut decision recorded by the addendum: the touch-surface app
(`pc4-microkit-studio/apps/pc4ms-touch-surface-android`) is treated as just
another UMP source riding the SET4-6 ingress (same posture as `mamut-seq`), with
the cross-repo boundary held by a golden-vector fixture exported from
`mamut-midi2` — mamut owns the contract, Kotlin is reviewed under the sibling
repo's conventions. No second authority path into the engine.
