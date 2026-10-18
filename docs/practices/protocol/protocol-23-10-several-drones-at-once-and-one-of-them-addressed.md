# Protocol 23.10: several Drones at once, and one of them addressed

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Spike 022, slice 5 (#1766, carrying #1666, #1651 and #1648's remaining half).

**Two routes and optional fields, all additive.** `PlanTask` gains `concurrent_with`, the other
tasks of its group it may run at the same time as, read both ways and less any pair Fleet ran
apart, and `touched_after_done`. `JobProcess` gains `drone_id`, and `recorded` now marks one row
per live Drone rather than one per Job, which is how *Drones running* is counted. Each is absent
where empty or false, so a 23.9 Bridge connects behind the banner and draws one Drone.

| Route | Body | Answers | Refused |
|---|---|---|---|
| `POST /jobs/:job_id/drones/:drone_id/kill`, `kill_one_drone` | — | `JobSummary` | 409 `fleet.drone_not_live`, with `drone_id` on its own field |
| `POST /jobs/:job_id/drones/:drone_id/redirect`, `redirect_one_drone` | `Redirection`, `redirect_drone`'s | `JobSummary` | 409 `fleet.drone_not_live`; 422 on a blank instruction |

**The kept Drone is the Job-wide act.** A Job keeps one Drone from admission to its end (answer
2); naming it is `kill_drone` or `redirect_drone`, and `kill_drone` now ends every Drone beside it
too, since the step they work stops. A Drone beside it is stopped alone, its task back to `open`
for the kept Drone, or told in its own session with the Job unmoved.

**The machine's cap counts Drones.** `get_capacity`'s `occupied` and the bound behind it count
each held Job's kept Drone and each Drone beside one; the Job's `drone_cap` from 23.8 sits inside
it, and a Drone beside a kept one gives way to a Job waiting to start.

**A Drone beside the kept one is announced, and is on no record of moves.** `drone.spawned` and
`drone.exited` carry its id with the Job as it stands, whose `assigned_drone` stays the kept one;
`list_job_drones` lists it off its task binding. **`observe_job` is one channel per Job**, made at
admission and dropped at the Job's end, so `closed` with `drone_ended` means the Job stopped
writing rather than one Drone exiting.

**`record_plan` takes an optional `concurrent_with` per task**, the places in the list of tasks
that may run beside it, refused where one is the task itself or in another group. It is an MCP
tool, not this protocol. **Store V99** keeps it, each Drone beside a kept one and how it left, each
task Drone's edit calls, and the pairs run apart. Overlap is read from edit calls only (answer
10): a shell write is not seen.
