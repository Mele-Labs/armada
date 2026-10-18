# Protocol 21.0: no Studio node is `frozen`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Decided with the owner on 1 Oct 2026, asked about a Finding's and an Outline's `frozen` after a
Sketch lost it in 20.0: *"I hate this frozen shit. Its overcomplicating it,"* and then **"Remove it
everywhere"**. `.claude/decisions/2026-10-01-no-studio-node-is-frozen.md`.

**Major, because a state value a client reads is gone.** `StudioNodeState` loses `frozen`, which is
the table's *variant the other side matches on* row: a body naming it no longer decodes, and a
Bridge built before this would draw a Finding with no state where it drew `frozen`.

**A Finding whose scout has ended carries no `state`.** How it ended is already `ended` on its
content, and `gathering` would say a scout still reads — Bridge pulses one, and a restart settles it
as failed. A Finding without `state` is always one with `ended`; Fleet's store refuses any other on
read. **An Outline is `draft`**, its only state. Store V86 moved every frozen Finding to no state and
every frozen Outline to `draft`, and narrowed the column's `CHECK` so neither comes back. Minor
resets to 0.
