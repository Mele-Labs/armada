# Where a Job's time went

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 3 Nov 2026. Fleet already keeps each Job's split when it ends (`crates/store/src/job_times.rs`) and sums turns, Check runs and builds in SQL (`crates/store/src/time_summary.rs`); this serves those reads.
**Additive only**: three operations, three routes and the types they carry; no event kind, no error and no field on an existing type but `RetroRecord.time`, which Fleet already wrote and the mirror now has.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/time-summary.ts` and `RecordTime` in `retro.ts`, written by hand.
**Names and counts only**; a Job's title is carried only on the slowest Jobs.

| Where | Side | Carries |
| --- | --- | --- |
| `get_job_time` (`GET /sessions/time?session_id=`, `agent_access = "Yes"`) | Bridge reads; an agent reads its own | `JobTime` `{ session_id, time? }`. `time` is `{ workflow, began_at, ended_at, spent, usual? }`, absent where no split is kept (the Job is running, or ended before splits were kept). 422 `fleet.session_unnamed` for a Job Fleet does not know |
| `get_usage_time` (`GET /usage/time?since=&until=&slowest=`, `agent_access = "Yes"`) | Bridge reads; an agent reads | `UsageTime` `{ since, until, jobs, total, slowest[], usual[] }` for the Jobs that ended in `[since, until)`: `until` defaults to now and `since` to seven days before it, `slowest` to twenty. 422 `fleet.session_unnamed` for a bound that is not a time |
| `get_usage_hours` (`GET /usage/hours?since=&hours=`, `agent_access = "No"`) | Bridge reads | `UsageHours` `{ since, until, hours[], builds[] }`: `hours` hours (48 where left out) from `since`, which is the start of an hour, or ending with the hour now is in. 422 `fleet.session_unnamed` where `since` is not a time |

| Field | Is |
| --- | --- |
| A split, `RecordSplit` | `model`, `tools`, `checks`, `subagents`, `slot`, `you`, `pull_request`, `stopped`, `idle`: **milliseconds**, and a Job's nine sum to its `ended_at` less `began_at`. It is the retro's `time` row's type, so one split has one unit on the wire |
| `usual`, `UsualTime` | `{ workflow, jobs, split }`: the median of each bucket over the `jobs` kept Jobs of the workflow, whatever the window |
| `slowest[]`, `SlowJob` | `{ session_id, title?, workflow, began_at, ended_at, split }`, longest first |
| `hours[]`, `HourFigures` | `{ hour, at, burn?, model?, tools?, checks?, pull_request? }`. `burn` is cost units by model in millions, as `get_usage_summary` has them. The four medians are **seconds** |
| `builds[]`, `BuildFigures` | `{ commit, started, until?, source?, main_commit?, branches, turns, burn?, model?, tools?, checks?, pull_request?, jobs, split }`. `commit` and `main_commit` are seven characters; `source` is `main` or `preview` |

**A figure nobody produced is absent, never `0`.** An hour with no model wait has no `model`; a build that took no turn has no `burn` and no medians.
A client that draws a line breaks it there.
It must not default the key to `0` and must not compute with `undefined`, which is `NaN`.

**Hours are sparse.** An hour in which nothing ran is left out and a client places the rest by `hour`, counted from `since`. Fleet does no calendar arithmetic, so it does not fill the gaps with dates.

**A build's `burn` is cost units per turn, not millions of them**: the summed units, which are millions, times a million over `turns`.
It is the one burn on the wire that is not in millions, so a client does not add it to one.
A build's identity is `commit` with `started`: a commit run again after another has two builds. `until` is where its run ends, so its tokens are `get_usage_summary` over `[started, until)`.

**Why `builds` rides on `get_usage_hours`.** The page draws the builds on the hours and they must begin at the same hour, which one `since` guarantees; two operations would be two answers that could disagree.
The cost is that a client that wants builds alone reads the hours as well, a pass over the turns of the window either way.

**The sums are the store's** and no new behaviour: `usage_hours`, `usage_builds`, `usage_time`, `usual_splits` and `job_time`.
A read that finds a bound that is not a time is refused as `get_usage_summary` refuses it.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.

**The event stream.** Nothing here touches it, so the unbounded-sink risk is neither better nor worse.
