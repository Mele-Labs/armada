# Protocol 21.4: a row names its Drone, and says how much it thought

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Two additions to a transcript row, on `observe_job` and on `get_drone`'s turns (#1662, #1664).

**`drone_id`, beside `step` and `by`**: the Drone whose transcript the row is in, the id
`list_job_drones` names. A Job with several Drones streams one interleaved history, and nothing
told their rows apart. A field rather than a socket per Drone, because the socket's point is one
order on one clock, and a viewer of one Drone filters. Fleet stamps it from the file's name,
`<drone-id>.jsonl`, so nothing is written per line and an older file stamps as fully as a new one.
Absent is a Fleet before 21.4, and a Helm thread's rows, which no Drone's transcript holds.

**`thinking`, a new `Saw` kind**, carrying `estimated_tokens`: the harness's estimate, cumulative
within one model call. It arrived before as `unrecognised` with the kind `system/thinking_tokens`
and the figure dropped. How much, never what: the reasoning text stays off the wire.

**Minor, by the 7.8 precedent, where `background_work` was named out of `unrecognised` the same
way.** Every Bridge reader of `Saw.event` has a fallback arm and nothing checks the union for
exhaustiveness, so an older Bridge reads the kind it has no case for rather than failing. That
Bridge does draw it wrong in one place: `story.ts`'s fallback is the unreadable row, so a
`thinking` row reads there as *a line the reader could not parse*, under the Fleet-ahead banner.
This Bridge reads `thinking` in the log and the working view exactly as it read the unrecognised
row before. Drawing the estimate on the Drones tab is still to do.
