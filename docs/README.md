# Docs

This directory is the local `EPM1` documentation entrypoint for the
`Mamut EPM` software line.

Ownership split:

- this repo `mamut-sint-sw` is the canonical implementation repo for `EPM1`
- sibling repo `mamut-sint-hw` is the canonical implementation repo for `EPM2`
- shared `Mamut EPM` identity framing still remains anchored across the broader
  umbrella docs corpus
- shared Linux audio platform planning is now anchored in
  `/home/dev/sel4/mamut-platform/docs/`

Start here:

- `mamut-epm-program-map.md` - shared program map for `Mamut EPM`, `EPM1`, and `EPM2`
- `EPM1_TRANSPORT_FREEZE.md` - local freeze/resume note for the last direct `EPM1` transport slice
- `EPM1_SPRINT_6_PC4_PERFORMANCE_RIG.md` - Sprint 6 backlog, acceptance bar, and live-rig contract
- `EPM1_SPRINT_6A_HOST_UNDERRUN_STABILITY.md` - next work item for real-host underrun diagnosis and performance stabilization
- `EPM1_PC4_LIVE_PROFILE.md` - locked Sprint 6 `PC4` control map, live slot policy, and performance-window truth model
- `EPM1_FIRST_PERFORMANCE_PLAYBOOK.md` - shortest trustworthy path for the first real `PC4` performance session
- `live-sessions/2026-04-29-pc4-ag03-headless.md` - first documented real `PC4 -> mioXM DIN 1 -> EPM1 -> AG03` headless hardware session
- `live-sessions/2026-04-29-pc4-ag03-headless-performance-mode.md` - follow-up PC4 Performance mode run with startup entry events and stuck-held-note finding
- `live-sessions/2026-04-29-pc4-ag03-clean-idle-noteoff-fix.md` - clean-idle Global MIDI follow-up and repeated-pitch note-off engine fix
- `live-sessions/2026-04-29-pc4-ag03-noteoff-fix-validated.md` - hardware validation that the repeated-pitch note-off fix clears the stuck-note failure
- `live-sessions/2026-04-29-pc4-ag03-full-controller-map.md` - full one-way `PC4` sustain/K/S/SW/wheels/aftertouch hardware map validation into Mamut
- `EXTERNAL_DSP_REFERENCES.md` - local notes on external DSP references such as
  `AudioNoise` and `dspc`

Current source-doc posture:

- local `EPM1` implementation truth lives in this repo's code, runtime, patches,
  and top-level README
- local transport architecture work is now frozen at the current bridge state;
  future transport evolution is expected to move into the shared Linux audio
  platform documentation track
- the current shared software/source architecture docs that shaped `EPM1`
  still live in `/home/dev/sel4/mamut-sint-hw/docs/`
- the current shared Linux audio platform direction now lives in
  `/home/dev/sel4/mamut-platform/docs/`

Current `EPM1` source anchors:

- `../mamut-sint-hw/docs/software-clone-architecture.md`
- `../mamut-sint-hw/docs/rust-workspace-architecture.md`
- `../mamut-sint-hw/docs/dsp-subsystem-spec.md`
- `../mamut-sint-hw/docs/patch-schema-v1.md`
- `../mamut-platform/docs/LINUX_AUDIO_PLATFORM_THESIS.md`
- `../mamut-platform/docs/LINUX_AUDIO_STACK_DECISION.md`
- `../mamut-platform/docs/C_RUST_BOUNDARY.md`
- `../mamut-platform/docs/LINUX_AUDIO_OPEN_QUESTIONS.md`
- `../mamut-platform/docs/LINUX_AUDIO_TRANSPORT_ARCHITECTURE.md`
- `../mamut-platform/docs/EPM1_FREEZE_AND_RESUME.md`
