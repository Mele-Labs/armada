# When a Job's Drone was last told

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`JobSummary.last_told_at`, optional and additive: when a person or Helm last sent the Job's Drone
words (`redirect_drone`, or the answer to a command it was refused) or restarted a step (`restart_step`).
Bridge reads it to decide that a failing pull request or red main the Job owns has somebody on it again, so
the Cockpit raises no call for it.

**Held in memory and lost on a Fleet restart**, as `releasing::Spared` is: a redirect into a healthy Drone
writes nothing to the log, so there was no record to read it from, and a surface treats the field as "told
since X", never as a history. A restart Fleet makes of its own after a boot does not set it. It rides the
rows Fleet already serves and adds no event, so it reaches a Board with the next summary of the Job.
