# Protocol 22.1: the merge line, read off disk

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

One route, one event kind and four DTOs: `GET /merge_lines` (`get_merge_lines`) answers
`MergeLines`, and `merge_lines.changed` carries the same body whole. Each `MergeLine` is one served
repository's `root`, its `line` in place order and the newest three `off` it, as `MergeLineEntry`
rows with `state` a `LandState`. Additive. `git show ecfffb2c1ebd:docs/capabilities/merge-line.md`, *In Bridge*, has
the field-by-field table.

**A fleet-wide fact that persists, so a route and an event**, `manifest.reread`'s rule. What is
new was the writer: `armada land` was another process, so nothing told Fleet a line moved. Fleet
read every served repository's `armada-land/` every two seconds and published only when the answer
changed. The files are no longer read; the loop folds the hub and publishes only when that moved. Bridge reads the route once per connection and keeps no timer.

**`LandState` is a strict enum, not an open set.** Bridge branches on it: the two live states
pulse and each end state draws its own facts. A new state is a major move.

**On the unmeasured risk above: neutral.** It adds no queue. It publishes at most once a read and
only on a change, so a quiet line costs the shared backlog nothing. A busy turn costs a handful of
events a minute.
