# `EPM1` Transport Freeze

This note freezes the current `EPM1` transport story at the documentation
level.

The intent is simple:

- `mamut-sint-sw` stops being the place where transport architecture keeps
  evolving directly
- the next transport/runtime-boundary work moves into the shared
  `Mamut Studio` Linux audio platform direction
- `EPM1` becomes the first adopter and proving ground for that future shared
  layer, not the long-term owner of transport infrastructure

## Freeze Assumption

This freeze assumes the current local transport work has reached an acceptable
bridge state.

For the purpose of planning and future resume, the accepted local transport
baseline is:

- patch load no longer lives on the callback fast path
- engine snapshot generation no longer lives on the callback fast path
- patch switch refreshes bend-range behavior correctly
- short output buffers are handled safely
- the callback boundary is non-blocking but still not the final
  Mamut-owned transport solution

This is enough to freeze around. It is not treated as the final transport
architecture for `EPM1`.

## What Is Frozen

The following is now frozen inside `mamut-sint-sw`:

- direct redesign of the standalone transport boundary
- direct queue architecture experimentation inside `mamut-standalone`
- direct callback/runtime-boundary redesign beyond ordinary bugfixes

Allowed during the freeze:

- ordinary bugfixes needed to keep `EPM1` usable
- narrow correctness fixes that do not reopen transport architecture
- docs alignment

Not allowed during the freeze:

- another local queue rewrite
- local lock-free transport experiments that bypass the shared platform track
- widening `mamut-sint-sw` into a general Linux audio infrastructure owner

## Why The Freeze Exists

`EPM1` is no longer just a synth repo that happens to need better callback
hygiene.

The transport problem has become broader:

- `EPM1`
- `EPM2`
- future Mamut Studio products
- one Linux platform family

That broader problem deserves its own architecture and vocabulary instead of
continued local evolution inside one product repo.

## Resume Point

`EPM1` transport work resumes only when the shared Linux audio platform effort
has produced a documentation-complete `v1` transport direction and a planned
implementation home.

The expected resume order is:

1. define the shared Linux audio platform docs in `mamut-platform`
2. later implement the shared transport layer in its own dedicated code home
3. adopt that shared layer into `mamut-sint-sw`
4. remove the temporary local transport implementation once replacement is
   proven

## Relationship To Platform Docs

The canonical next-step documents are expected to live in:

- `/home/dev/sel4/mamut-platform/docs/LINUX_AUDIO_PLATFORM_THESIS.md`
- `/home/dev/sel4/mamut-platform/docs/LINUX_AUDIO_STACK_DECISION.md`
- `/home/dev/sel4/mamut-platform/docs/C_RUST_BOUNDARY.md`
- `/home/dev/sel4/mamut-platform/docs/LINUX_AUDIO_OPEN_QUESTIONS.md`
- `/home/dev/sel4/mamut-platform/docs/ADR_WORKFLOW.md`
- `/home/dev/sel4/mamut-platform/docs/LINUX_AUDIO_TRANSPORT_ARCHITECTURE.md`
- `/home/dev/sel4/mamut-platform/docs/LINUX_AUDIO_TRANSPORT_BACKLOG.md`
- `/home/dev/sel4/mamut-platform/docs/EPM1_FREEZE_AND_RESUME.md`
- `/home/dev/sel4/mamut-platform/docs/adrs/`

This file is intentionally local and short. It exists so an engineer entering
through `mamut-sint-sw` can see immediately that transport work is paused here
on purpose and where it resumes.

## Resume Checklist

Before local transport architecture work resumes in `mamut-sint-sw`, the
following must already exist on the platform side:

1. platform thesis freeze
2. `C + Rust` stack decision freeze
3. `C` to `Rust` boundary freeze
4. open questions register
5. required ADR set with explicit status
6. planned implementation home for the shared layer
