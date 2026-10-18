# Protocol 14.19: a person answers a call Helm was refused

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#1389`. Two new event kinds, three new routes and a DTO family, all additive. `helm.asking_to_run`
carries `HelmAskingToRun { waiting: HelmCallInFlight }` when a Helm job reaches for something
the person's own agent settings do not cover; `helm.call_answered` carries `HelmCallAnswered {
call, manifest_id, tool, detail, rule, settled, at }` when the ask ends, whoever ended it. The
routes are `POST /helm/permission` (the agent door's own permission tool, reached by the CLI and
never by a model), `GET /helm/calls` and `POST /helm/calls/answer`.

**The ask carries no `job_id`, and that is what makes it a new type rather than a
`CommandInFlight`.** Every field of that one is about a Job — `step_id`, `allow_for_job`, a rule
written into `armada.yml` on the Job's branch — and a Helm call has none: nothing is queued, no
step is running, and what waits is one process inside one tool call. `HelmCallAnswer` is its own
closed set for the same reason, and a surface matches on it to pick controls, so a fourth value in
it is a major bump the way `WhenBlocked`'s third was.
