# A Job's burn over time

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 19 Nov 2026. One Job's burn from its start to its end, for the native Bridge's chart of it and for an agent.
**Additive only**: one operation, one route and the types it carries; no event, no error and no field on an existing type.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `classic-final:packages/protocol/src`, and there is no number to take.
Bridge's half is `classic-final:packages/protocol/src/usage-job-timeline.ts`, written by hand.
The rows of one Job are read in one pass and cut into buckets in `crates/store/src/usage_job_timeline.rs`, so Bridge is sent figures and never rows.
**Names and counts only**; the Job is its id, and no tool is named but through what a pause waited on.
What each figure is, and how a pause is told from a gap, are `docs/concepts/session/where-the-tokens-went.md`, *A Job's burn over time*.

| Where | Side | Carries |
| --- | --- | --- |
| `get_job_timeline` (`GET /usage/job_timeline?session_id=`, `agent_access = "Yes"`) | Bridge reads; an agent reads | `JobTimeline`. 422 `fleet.session_unnamed` for a Job Fleet does not know |

| Field | Is |
| --- | --- |
| `session_id`, `started` | The Job, and where its span begins: the Job's start, or its first request where that came first |
| `ended` | Where the Job ended. Absent while it runs, and the span then runs to now |
| `bucket_seconds` | 60, 300, 600, 900, 1800 or 3600: the smallest that cuts the span into at most 120 buckets, and 3600 where none does |
| `total_units` | The Job's whole burn, which is `get_usage_summary`'s for the Job |
| `week_units` | Every Job's burn over the 7 days ending at the span's end, for a share of the week |
| `buckets` | Every bucket from `started`, oldest first, one with no burn included: `{ at, main_units, subagent_units }`. A compaction's own request is the main thread's |
| `context` | The main thread's input, cache writes and cache read at each request, `{ at, tokens }`, thinned to at most 400 by keeping the largest of each equal share |
| `compactions` | The stamp of each compaction |
| `pauses` | A gap of more than 5 minutes between two main-thread requests: `{ from, to, waited_on, label, cold }`. `waited_on` is `command`, `subagent`, `you` or `other`, and a later one is a new id, so a client draws one it does not know from its `label`. `label` is `Command ran`, `Subagent ran`, `Waited on you` or `Paused`. `cold` is true where the request after it found the cache cold |
| `peak_context`, `cold_wakes`, `subagent_share` | The most context the main thread held, the main-thread requests that found the cache cold, and the part of `total_units` subagents burned, 0 to 1 |

**Skew.** A Fleet that predates this answers 404, and the page draws no timeline; a Bridge that predates it never asks.
