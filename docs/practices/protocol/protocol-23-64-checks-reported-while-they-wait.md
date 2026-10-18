# Protocol 23.64: Checks reported while they wait

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/manifest.md`, *How many Checks run at once*. **Additive only**: no field and no operation, two new values and one id that was absent.

| Where | Carries | Notes |
| --- | --- | --- |
| `list_manifest_checks` | A `gate` row with `state` `waiting` or `running` | One per Check of a running gate that has not finished, with no `ended_at` and, while waiting, no `started_at`. **Waiting covers every wait**: an earlier Check at the one-at-a-time gate, a prerequisite, a place, a machine slot. A finished Check is the stored ruling's row, as before |
| `list_manifest_checks` | An `asked_run` row with `state` `waiting` | A Drone's run that has started none of its Checks. Reads `running` once one starts |
| `get_merge_lines` | `MergeLineCheck.requester.job_id` and `handle` | On a branch some Job owns. Absent on one none does. A Bridge from before 23.64 ignores them |
| `get_merge_lines` | `MergeLineEntry.checks` on a queued branch | A branch in `line` with state `waiting` that a Job owns lists the Manifest's declared Checks, each `waiting` and naming the Job, until the line gates it and writes its own. Absent on a branch no Job owns |

**A known cost of calling this minor.** A Bridge from before 23.64 has no `waiting` in its gate vocabulary and draws such a row failed, so a Fleet ahead shows a waiting Check on that Bridge's Checks page as red.
The banner for a Fleet ahead does not say so.
Bridge's half is `packages/screens/src/manifest-checks.ts`.
