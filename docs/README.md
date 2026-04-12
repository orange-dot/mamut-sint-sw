# Docs

This directory is the local `EPM1` documentation entrypoint for the
`Mamut EPM` software line.

Ownership split:

- this repo `mamut-sint-sw` is the canonical implementation repo for `EPM1`
- sibling repo `mamut-sint-hw` is the canonical implementation repo for `EPM2`
- shared umbrella framing currently remains curated in the hardware-side docs
  corpus while implementation ownership is split across both repos

Start here:

- `mamut-epm-program-map.md` - shared program map for `Mamut EPM`, `EPM1`, and `EPM2`

Current source-doc posture:

- local `EPM1` implementation truth lives in this repo's code, runtime, patches,
  and top-level README
- the current umbrella/source architecture docs that shaped `EPM1` still live in
  `/home/dev/sel4/mamut-sint-hw/docs/`

Current `EPM1` source anchors:

- `../mamut-sint-hw/docs/software-clone-architecture.md`
- `../mamut-sint-hw/docs/rust-workspace-architecture.md`
- `../mamut-sint-hw/docs/dsp-subsystem-spec.md`
- `../mamut-sint-hw/docs/patch-schema-v1.md`
