# DSP Implementation Language Strategy

## Status

This is an implementation strategy note for next-generation DSP work.

It applies to:

- `material-core-idea.md`
- `gravitational-phase-lattice-idea.md`
- `next-generation-dsp-research-tracks.md`

It is not a commitment that every experiment becomes product code.

## Decision

Use `Rust + C`.

`Rust` remains the primary language for:

- DSP model ownership
- engine integration
- identity and parameter mapping
- patch-facing configuration
- deterministic tests
- offline render tools
- first scalar prototypes
- production `mamut-field` or `mamut-dsp` modules

`C` is reserved for:

- narrow audio-adjacent substrate code
- profile-proven hot kernels
- fixed-layout ABI experiments
- future platform/device-near integration

Do not introduce `Python`, `Julia`, `C++`, `Zig`, or Faust as a required base
for this DSP line.

## Why Not Python Or Julia As The Base

`Python` and `Julia` are useful for quick numerical sketching, plotting, and
parameter sweeps. They are not wrong tools in isolation.

They are rejected here as a base workflow because they add a separate execution
model that does not carry enough long-term value for this workspace:

- different numeric/runtime semantics from the product engine
- garbage-collected runtime assumptions that cannot enter audio callback design
- extra dependency and environment surface
- another artifact format for evidence and reproduction
- risk that "notebook truth" drifts from Rust engine truth

If a one-off scratch notebook is ever used outside the repo, it must be treated
as disposable evidence, not as source truth. The canonical experiment should be
recreated in Rust before it influences engine design.

## Rust Role

The first serious prototype should be Rust, even if it is still experimental.

Preferred shape:

```text
crates/mamut-field
  scalar reference implementation
  offline WAV/buffer renderer
  deterministic parameter sweeps
  finite-output tests
  no allocation in render kernels
```

For `GFM`, start with a scalar reference implementation:

- fixed lattice size through const generics
- fixed buffers
- deterministic local RNG
- explicit state reset
- explicit sample-rate handling
- no hidden global state

SIMD is a later optimization, not the first source of truth.

## C Role

C is a tactical escape hatch, not a parallel product engine.

Extract a C kernel only after Rust profiling shows a real budget problem on the
target machine.

The allowed C shape is narrow:

```c
gfm_step(params, state_in, state_out, frames)
```

The C side may own:

- raw arithmetic hot loops
- fixed-size state stepping
- optional target-specific SIMD intrinsics

The C side must not own:

- patch schema
- macro or identity meaning
- `Gravitacija`, `Pec`, `Baklja`, `Horizont`, or `Swarm` semantics
- engine lifecycle
- voice allocation
- plugin or standalone shell behavior
- strings, heap allocation, logging, or callbacks in the audio path

Rust remains responsible for mapping identity state into plain numeric C
parameters.

## C++ Position

C++ is not part of the base plan.

It is technically capable, especially for SIMD-heavy DSP, but it adds build,
ABI, exception/RTTI, review, and ownership complexity that this workspace does
not need yet.

Use C++ only for an isolated throwaway comparison if a specific library or
kernel technique must be evaluated. Do not make it a production dependency
without a new architecture decision.

## Prototype Sequence

1. Write the reference model in Rust.
2. Add offline render evidence in Rust.
3. Add deterministic and finite-output tests.
4. Integrate behind an explicit engine flag or patch field only after the
   reference behavior is musically useful.
5. Profile on the real target.
6. Extract a C kernel only if profiling proves the Rust hot loop cannot meet
   the budget.

## Rule Of Thumb

If the question is "what does this instrument mean?", keep it in Rust.

If the question is "can this one loop meet a measured budget?", C is allowed.
