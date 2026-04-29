# Mamut EPM Program Map

`Mamut EPM` is the umbrella program. It currently has two explicit product
lines:

- `EPM1` = software/runtime line in `mamut-sint-sw`
- `EPM2` = hardware instrument line in `mamut-sint-hw`

Current posture:

- `EPM1` is the current primary playable expression
- `EPM2` is the hardware continuation of the same identity
- both lines share the same identity vocabulary: `Horizont`, `Pec`, `Baklja`,
  and `Gravitacija`

## Canonical Repo Ownership

`mamut-sint-sw` is canonical for:

- `EPM1` engine/runtime implementation
- DSP implementation
- standalone runtime behavior
- patch runtime code and validation
- factory patch bank behavior

`mamut-sint-hw` is canonical for:

- `EPM2` hardware execution
- simulation, KiCad capture, and bench evidence
- hardware architecture and block-level implementation direction

Current umbrella/source-doc posture:

- shared `Mamut EPM` identity and program framing is currently curated in the
  sibling `mamut-sint-hw` docs track
- this does not change software implementation ownership: `EPM1` runtime code
  and execution truth live in `mamut-sint-sw`

## Shared vs Implementation-Specific

Shared across `EPM1` and `EPM2`:

- umbrella identity and program framing
- `Horizont`, `Pec`, `Baklja`, `Gravitacija`
- macro language and behavioral targets
- top-level product thesis

`EPM1`-specific:

- Rust workspace structure
- engine/DSP/runtime implementation
- patch schema execution and validation in code
- standalone session ergonomics

`EPM2`-specific:

- analog voice path and control architecture
- simulation, KiCad, bench workflow, and hardware stage gates
- physical build constraints and calibration path

## Working Rule

When a question is about playable software behavior, runtime code, DSP, patch
execution, or standalone usage, the canonical repo is `mamut-sint-sw`.

When a question is about physical instrument architecture, simulation, KiCad,
bench evidence, or hardware stage gates, the canonical repo is
`mamut-sint-hw`.
