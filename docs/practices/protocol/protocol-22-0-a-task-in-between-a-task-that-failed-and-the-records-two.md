# Protocol 22.0: a task in between, a task that failed, and the Record's two new signers

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Spike 022, the wire lock for the new Job, signed off by the owner on 2 Oct 2026, slice 1a (#1760).

**Major, three times over, and bundled so the milestone takes one.** A major is the lifeboat for
a Fleet caught mid-Job, so the spike's *protocol changes, bundled* section puts every breaking
change of the milestone here rather than one per slice.

| Change | Why it breaks |
|---|---|
| `TaskState` gains `handed_in` | A strict set Bridge matches on: an older Bridge draws a handed-in task as `open` |
| `TaskState` gains `failed` | The same set, the same reason |
| `Actor` gains `judge` and `check` | A strict set stored on every recorded row; an older peer reads neither |
| `TaskCounts` gains optional `handed_in` and `failed` | Additive on its own, absent at zero; it rides the major |

**`handed_in` is the owner's answer 1**: a task's agent has handed its work in and the step's Checks
have not answered, so done arrives at green. **Nothing writes it or `failed` at 22.0**: slice 1b
writes the first at a task Drone's hand-in, slice 2 the second when a group's Checks go red, and
`update_task` refuses both, because both are Fleet's to mark.

**`judge` and `check` sign the rows their own answer wrote.** A refusal's step stop and escalation
are signed `judge`, as are a Judge asking a person and a gaming flag; a failed Check's stop and
hold, and a hand-back's `retrying`, are signed `check`. Fleet signs every move it decided on more
than one answer: an advance, a policy hold, a gate that could not decide. `fleet::Ruling::signed_by`
is the one place that says which. `human` stays `human`; spike 020's rename to `person` is not
needed, because Bridge says *you*.

**The figure.** Done over every count but `dropped`: a handed-in task and a failed one join the
total and neither joins `done`.

**Store V90 moves `KNOWN_SCHEMA_VERSION`**, so a Fleet built before this refuses the store at open
rather than failing to fold a Job signed `judge`. It also rebuilds the plan's `state` check to admit
the two new states. Minor resets to 0, and a Bridge and a Fleet must both be rebuilt from the same
commit.
