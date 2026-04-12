# mamut-sint-sw

`mamut-sint-sw` is the sibling Rust workspace for the software-clone line of
the `Mamut` synth concept.

Current status:

- Sprint 1 core workspace is implemented
- patch/schema/identity/engine contracts are live in code
- standalone is currently a headless validation and dry-run shell
- plugin/editor work is intentionally deferred

Workspace crates:

- `mamut-params` - stable parameter and macro registry
- `mamut-patch` - canonical TOML patch model and validation
- `mamut-identity` - macro-to-identity resolution
- `mamut-dsp` - shared DSP utilities and buffer helpers
- `mamut-engine` - voice allocation and process pipeline skeleton
- `mamut-standalone` - headless runtime for patch validation and engine dry-runs

Quick start:

```bash
cargo test
cargo run -p mamut-standalone
cargo run -p mamut-standalone -- validate patches/factory/molten-horizon.toml
```
