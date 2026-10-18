# Protocol 23.11: a Drone resting at the gate, and turns nobody counted

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Job 3 on 3 Oct. Its last task's Drone submitted and ended its run, and `list_job_drones` read it
`running` for the seven minutes the step's Checks ran, so Bridge drew a Drone working while the
Checks were. Its three task Drones before it read `turns: 0` with no cost, each having made 6 to 12
calls: Fleet ended each within a quarter-second of its hand-in, before its terminating line.

**`JobDrone.at_rest_since`, optional and additive, so the minor moves.** When a `running` Drone's
last run ended, off its transcript's last terminating line with no run started or turn sent after
it; absent while it works and on a Drone that has left. `running` still means Fleet holds it — at
rest at the gate it is the Drone a red Check goes back to — so a fifth `DroneState` would have
been a major move for a Bridge matching on the states it knows, and the field says the same thing. A 23.10 Bridge
connects behind the banner and draws it working, as before.

**`turns` is absent on a stopped Drone whose row saw no terminating line**, where it read `0`. The
wire type is unchanged, and absent was already the field's word for none seen. Fleet now lets a
task's Drone that handed in come to rest before ending it, within `StepNorms::report_grace`, so
its row carries the harness's turns and cost; one that does not rest in that time reads absent.
