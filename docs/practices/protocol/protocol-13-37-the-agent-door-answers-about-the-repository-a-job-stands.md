# Protocol 13.37: the agent door answers about the repository a job stands in

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`?manifest_id=`, optional and additive, on `list_jobs`, `list_job_board`, `list_reviews`, `get_activity_feed`, `list_alerts`, `list_drones`, `list_worktrees`, `list_servers` and `get_events_since`: absent is every repository, as before, so Bridge's All view is unchanged (#987).
A named `get_events_since` counts events about that Manifest, about a Job it owns, and those naming neither, which are the machine's.
`armada mcp` names the Manifest it walked to on every call to `/agent/mcp`, and the door names it on each of those routes and on the Manifest reads.
Through the door, a `:job_id` another Manifest owns is refused as `fleet.job_in_another_repository`, a call naming another Manifest is refused, and `propose_job` takes its owner from the scope.
Bridge's own routes are unscoped.
