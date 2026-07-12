---
name: evidence-class-honesty
description: mamut-sint-sw separates synthetic (mamut-seq virtual port) from hardware (live-sessions/) evidence; reviewer must check audible claims aren't overclaimed
metadata:
  type: project
---

This repo maintains a strict evidence-class boundary that the integrated reviewer
should police on any docs/evidence change:

- **Synthetic development evidence** = `mamut-seq` virtual-port runs (deterministic
  control streams). Lives in `docs/EPM1_SEQ_V0.x_*` and scenario headers.
- **Hardware evidence** = real `PC4 -> mioXM -> EPM1` rig runs. Lives ONLY in
  `docs/live-sessions/` (hardware-only class).

Rules enforced by the SET3 docs (good pattern to expect):
- Each evidence doc states its class up front and carries a "Remaining
  hardware-confirmed step" section that DEFERS audible results to a rig run
  rather than claiming them.
- `docs/EPM1_GFM_LIVE_BUG_PLAYBOOK.md` synthetic-rig section says the classes are
  "not interchangeable" and "never file synthetic runs as hardware evidence."

**Why:** GFM failure classification can only be finalized on real hardware; a
synthetic pass proving the control stream is correct is not proof the layer was
audible.

**How to apply:** when reviewing new SEQ/GFM evidence, confirm (a) class is
declared, (b) audible/underrun claims are deferred to hardware unless a rig run is
actually recorded, (c) nothing synthetic is filed under `docs/live-sessions/`.
Event-count and jitter claims in these docs have been accurate to the code when
spot-checked (hand-computed gate-arm=78, bcs-layer-basic=31 matched).
