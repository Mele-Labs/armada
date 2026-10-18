# Protocol 23.14: each wave's line, and the order a wave waits in

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Spike 022, slice 6's last two pieces (#1692): `WaveRoundView`'s per-pass line, and the waits-on
edges a wave's graph is drawn from.

**Two optional fields, both additive.** `JobDetail` gains `wave_rounds`, and `JobSummary` gains
`waits_on`. Each is absent where empty, so a 23.13 Bridge reads neither key and draws each wave by
its number alone, in one column, as it did.

| Field | On | Carries | Absent |
|---|---|---|---|
| `wave_rounds` | `JobDetail` | Retired: Fleet no longer fills it | Always |
| `waits_on` | `JobSummary` | The ids of the Jobs this one waits on: its `depends_on` edges, which `JobDetail.dependencies` serves whole | A Job that waits on nothing |

**A pass is counted as `dispatched_pass` counts it**: one, plus every return the roll-up sent onto
the step that proposes. A recording belongs to the pass the returns before it opened, so a Judge's
rerun inside a pass replaces that pass's line, and a return starts the next. **The approach is
served whole**, a paragraph; Bridge's strip reads its first sentence, because a tab does not
truncate.

**The edges ride the Board row rather than a read per child.** The row is built from the Job record,
which already holds its edges, so serving them costs no store read; the alternative was one
`get_job` per member to draw one graph. Only `depends_on` crosses: a `blocks` edge is the same
fact read from the other Job, whose own row carries it.

**No store change.** The lines are read off the plan's history and the step moves already kept, and
only for a Job whose frozen workflow has a step that proposes Jobs.
