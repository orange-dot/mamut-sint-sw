---
name: drift-mozaik-enable-safety-overgeneralized
description: README over-generalized a Mozaik-scoped "no binding enables/re-seeds a layer" safety claim to an absolute "No binding kind…" — contradicted by the pre-existing bcs_layer_enabled kind
metadata:
  type: project
---

SET5-8's README `mozaik_control` bullet states "No binding kind enables, disables, or re-seeds a layer." That absolute phrasing is contradicted by the pre-existing `bcs_layer_enabled` binding kind (listed two lines up in the same README), which enables/disables the BCS layer from a CC. The substantive safety property holds only for Mozaik (there is genuinely no CC-bindable Mozaik enable/mode path; `mozaik on`/`--mozaik` is the only enable).

The SET5-8 backlog's own Design Sketch has the correct precise wording: "`bcs_layer_enabled` is the precedent that it *can* be done; for the seeded layers, enable stays a deliberate CLI/headless act." The README dropped that precision.

**Why:** this repo's integrated-review remit is doc-claims-match-code; an absolute claim the code contradicts is exactly the drift to catch, and here the authoritative backlog already had the honest scoping to copy.

**How to apply:** when reviewing controller-binding docs, scope enable/re-seed safety claims to the specific layer (Mozaik), never "no binding kind." Watch for the same over-generalization when `orbita_control`/`kosava_control` land. Relates to [[boundary-runtime-control-queue-continuous]].
