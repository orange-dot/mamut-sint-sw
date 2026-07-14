---
name: boundary-session-command-vs-binding-path
description: Two distinct entry paths converge on the shared session method; the SET6 backlog initially cited the CC-binding arm as the headless command path — fixed at gate, and SET6-4 must mirror the right chain
metadata:
  type: project
---

There are two distinct entry paths onto the shared session method `set_mozaik_param` (`session/switching.rs:84`): the **headless command chain** — parse (`cli/runtime_commands.rs:67`, the `"mozaik"` arm) → dispatch (`session/commands.rs:74`) → session method — and the **controller-profile binding arm** (`session/runtime.rs:316-317`, the `RuntimeControlMessage::MozaikControl` route from SET5-8). The SET6 tonewheel backlog originally cited `runtime.rs:317` as "the headless `mozaik set` path"; the gate corrected it to the parse→dispatch→session-method chain.

**Why:** an implementer building `tonewheel drawbars` / `tonewheel status` / `tonewheel set` who follows the wrong anchor lands in the CC-binding branch and misses the request/reply session-method shape both paths must converge on ("one truth" for clamping/defaults).

**How to apply:** at the SET6-4 gate, check the tonewheel commands mirror the full chain and that any future drawbar CC binding routes onto the same shared session method. Registration is a nine-digit *vector* value — if it ever rides the runtime-control queue as a continuous sender, the drop-newest/no-coalesce caveat in [[boundary-runtime-control-queue-continuous]] applies to all nine lanes at once.
