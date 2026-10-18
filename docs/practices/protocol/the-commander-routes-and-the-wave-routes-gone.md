# The Commander routes and the wave routes: gone

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

The Commander's socket, its words, its card and the owner's pick, its repair
Drone and its model choice are gone from Fleet, with the `commander.changed` and
`commander.card` events and the types that carried them. So are
`POST /jobs/:job_id/approve_wave` and the `dispatch_job` MCP tool, with the wave
a plan proposed. The Commander was deleted with the Jobs it steered
(`.claude/decisions/2026-10-15-sessions-absorb-jobs.md`). A Job's linked
Jobs and *after another* are not waves and stay.
