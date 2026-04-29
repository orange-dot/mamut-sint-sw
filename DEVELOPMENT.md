# Development

Use this file as the local implementation entrypoint for `mamut-sint-sw`.

## Canonical Smoke Path

```bash
cargo test --locked
```

This is the main public proof path for the repo today.

## Runtime-facing Smoke

For a quick product-facing sanity pass:

```bash
cargo run --locked -p mamut-standalone -- list-factory
```

Optional safe runtime paths:

```bash
cargo run --locked -p mamut-standalone -- dry-run molten-horizon
cargo run --locked -p mamut-standalone -- play --demo molten-horizon
```

## Reading Order

1. Root `README.md`
2. `docs/README.md`
3. `docs/EPM1_PC4_LIVE_PROFILE.md`
4. `docs/EPM1_FIRST_PERFORMANCE_PLAYBOOK.md` for the first real performance run
