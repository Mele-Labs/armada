# Protocol 13.43: a gate reuses a passing dry run

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`CheckRun.reused_from_dry_run`, additive: when a Check's result came from the Drone's own `run_checks` instead of the gate running it again, rather than absent for a Check the gate ran itself (#1014).
