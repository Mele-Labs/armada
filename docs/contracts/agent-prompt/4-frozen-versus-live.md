# 4. Frozen versus live

**Part of:** [Agent Prompt Contract](../agent-prompt.md).

The Drone spawn rule already distinguishes these. Stated here in prompt
terms.

**Frozen means frozen for the Job, and the snapshot is taken at Job
creation.** Every step of a Job spawns its own Drone, so "frozen at spawn"
would mean re-resolved at every boundary — which would let a Drone weaken
its own yardstick between steps. Fleet snapshots what a Drone works under
when the Job is created and hands the same snapshot to every step's Drone.
See `../concepts/drone.md`.

| Layer | State |
| --- | --- |
| 1 Baseline | **Frozen at build.** Compiled in, unreachable from config |
| 2 Kit, 3 Manifest | **Frozen at Job creation.** Editing the Kit file mid-Job changes nothing for any Drone of that Job, including the ones it has not spawned yet |
| 4 WorkflowDef | **Frozen at the approval press** (#1581). A person may pick another workflow and set each step's gate until then; no Drone exists before it, so none reads a version that moved |
| 5 Task | **Frozen at the approval press** — `acceptance_criteria[]` especially, or the Judge grades against a list something invented later. A person corrects the list up to the press and not after; a criterion is appended only at an approved widening, so a citation to "criterion 4" still resolves |
| 6 Step | Its content changes at each step boundary, because the step does. Its *sources* are frozen with the rest |
| Allowlist, budget, freeze | **Live** — re-read at every gated checkpoint. Not prompt content; enforcement |

**A scope revision is a respawn, not an edit** — and it is the only
respawn that re-resolves anything. A step boundary is a respawn too, and
it re-assembles the same six layers from the same snapshot. On a scope
revision Fleet terminates the Drone, re-resolves configuration against the
new Manifest set, and spawns a fresh one, because a person approved the
new set. **There is no path that mutates a running Drone's system prompt**
— layers 1 to 6 are fixed for the process lifetime, and a process now
lasts one step.
