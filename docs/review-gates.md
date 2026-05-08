# Review Gates

This repo vendors a strict `seL4` review discipline under `.claude/agents/`.
The `claude` directory name is historical; the same discipline applies to
Codex agents working in this repo.

These reviews are used here as an engineering bar for:

- honest boundaries
- type and ownership discipline
- panic-free runtime paths
- hot-path simplicity
- code/doc/schema alignment

They are not limited to literal `seL4` runtime code.

## Agent Set

Vendored agent files:

- `.claude/agents/sel4-integrated-systems-reviewer.md`
- `.claude/agents/sel4-rust-systems-reviewer.md`
- `.claude/agents/sel4-rust-single-file-reviewer.md`
- `.claude/agents/sel4-rust-execution-optimizer.md`
- `.claude/agents/sel4-systems-reviewer.md`
- `.claude/agents/sel4-single-file-reviewer.md`
- `.claude/agents/sel4-c-execution-optimizer.md`

Primary usage in this repo today is Rust-first:

- `sel4-integrated-systems-reviewer`
- `sel4-rust-systems-reviewer`
- `sel4-rust-single-file-reviewer`
- `sel4-rust-execution-optimizer`

The C reviewers are reserved for future:

- FFI bridges
- native backend code in C
- helper libraries with C hot paths

## Mandatory Gates

### Core Rust crates

Required for changes in:

- `crates/mamut-engine`
- `crates/mamut-dsp`
- `crates/mamut-runtime`
- `crates/mamut-standalone`
- `crates/mamut-patch`

Gate sequence:

1. `sel4-rust-single-file-reviewer` on the riskiest changed files
2. `sel4-rust-systems-reviewer` on the affected crate or subsystem
3. `sel4-integrated-systems-reviewer` when the change also alters docs,
   boundaries, patch semantics, or runtime shape

### Hot-path audio changes

Required for:

- render loop changes
- audio callback changes
- runtime audio queue and MIDI callback boundary changes
- voice allocator timing-sensitive changes
- filter/oscillator/final-stage fast paths

Mandatory extra gate:

- `sel4-rust-execution-optimizer`

### Architecture, docs, and patch-schema changes

Required when changing:

- `README.md`
- `docs/`
- patch model or validation policy
- crate boundaries
- runtime ownership expectations

Mandatory gate:

- `sel4-integrated-systems-reviewer`

## Review Bundles By Change Type

### Patch or schema changes

Run:

- `tools/review/run-rust-file-review.sh crates/mamut-patch/src/lib.rs`
- `tools/review/run-rust-subsystem-review.sh crates/mamut-patch`
- `tools/review/run-integrated-review.sh crates/mamut-patch docs/review-gates.md README.md`

### Identity or control changes

Run:

- `tools/review/run-rust-file-review.sh crates/mamut-identity/src/lib.rs`
- `tools/review/run-rust-subsystem-review.sh crates/mamut-identity crates/mamut-engine`
- `tools/review/run-integrated-review.sh crates/mamut-identity crates/mamut-engine docs`

### DSP or render-path changes

Run:

- `tools/review/run-rust-file-review.sh crates/mamut-dsp/src/lib.rs`
- `tools/review/run-rust-subsystem-review.sh crates/mamut-dsp crates/mamut-engine`
- `tools/review/run-rust-hotpath-review.sh crates/mamut-dsp/src/lib.rs crates/mamut-engine/src/lib.rs`
- `tools/review/run-integrated-review.sh crates/mamut-dsp crates/mamut-engine README.md`

### Standalone runtime changes

Run:

- `tools/review/run-rust-file-review.sh crates/mamut-standalone/src/main.rs`
- `tools/review/run-rust-subsystem-review.sh crates/mamut-standalone crates/mamut-engine`
- `tools/review/run-rust-hotpath-review.sh crates/mamut-standalone/src/main.rs crates/mamut-engine/src/lib.rs`
- `tools/review/run-integrated-review.sh crates/mamut-standalone README.md docs`

## Definition Of Done

A change touching core synth behavior is not done until:

- tests pass
- the relevant review scripts were run
- findings were addressed or explicitly recorded
- docs still describe the implemented system truth

For audio/runtime work, done additionally means:

- no obvious allocation or logging contamination in steady-state hot paths
- no misleading abstraction hiding ownership or timing behavior
- no doc/code drift around patch, identity, or runtime boundaries

## Helper Scripts

Repo-local helper scripts:

- `tools/review/run-rust-file-review.sh`
- `tools/review/run-rust-subsystem-review.sh`
- `tools/review/run-integrated-review.sh`
- `tools/review/run-rust-hotpath-review.sh`

These scripts do not mutate source files.

They standardize:

- which agent to use
- which files to review
- what evidence to include
- how to phrase the prompt
