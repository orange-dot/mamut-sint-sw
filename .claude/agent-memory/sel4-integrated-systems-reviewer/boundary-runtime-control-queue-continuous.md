---
name: boundary-runtime-control-queue-continuous
description: The runtime-control queue is a discrete-event path; mozaik_control (SET5-8) is the first continuous value-controller on it — a shape orbita_control/kosava_control will copy
metadata:
  type: project
---

The `RuntimeControlMessage` queue (`RUNTIME_CONTROL_QUEUE_CAPACITY`, drop-newest on full via `ArrayQueue::push`, NO coalescing) was designed for discrete events: program-change, favorites, toggle-param, and the panic/reset priority bypass. Drain cadence is per-GUI-frame (`gui/app_methods.rs`), per-TUI-tick (`mamut-tui/src/app.rs`), or per-headless-prompt (`session/commands.rs`).

SET5-8's `mozaik_control` is the FIRST continuous value-carrying sender on this queue. It routes here (not the realtime coalesced `ControllerEvent` path) deliberately, to reuse the single `set_mozaik_param` → `EngineCommand::SetMozaikParam` request/reply path ("one truth" for clamping/smoothing/detent-snap). That tradeoff is correct, but it means Mozaik CCs bypass the controller coalescing the realtime path gets, and under sustained overflow the drop-newest semantics can settle the control at a stale intermediate value rather than the performer's final position.

**Why:** matters because the immediate consumer is a continuous slide lane (PC4MS Android touch surface). For the intended GUI session the per-frame drain keeps up; a headless session with an idle prompt is where overflow could bite.

**How to apply:** when reviewing the mirror slices `orbita_control`/`kosava_control` (SET5-2/SET5-6, which the backlog says copy this exact kind+action+routing shape), expect the same queue-shape caveat — and check whether by then a param-keyed coalescing (latest-wins) has been added. Relates to [[evidence-class-honesty]].
