# The usage summary

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 18 Oct 2026. What a window of turns cost, summed for the Usage page. **Additive only**: one operation and the types it carries; no fact, event or field on `SessionRecord`. Bridge's half is `packages/protocol/src/usage-summary.ts`, written by hand. The sums are made in SQL (`crates/store/src/usage_summary.rs`) so Bridge is sent figures and never rows. **Names and counts only**; titles are not carried, and Bridge joins `session_id` to the ones it holds.

| Where | Side | Carries |
| --- | --- | --- |
| `get_usage_summary` (`GET /usage/summary?since=&until=&tz_offset_minutes=`) | Bridge reads | `UsageSummary`. `since` and `until` are optional RFC3339, `until` exclusive and defaulting to now, `since` to seven days before it; `tz_offset_minutes` (default 0) is the caller's local offset from UTC. 422 `fleet.session_unnamed` for a bound that is not a time |

**Burn is in cost units**: input 1, cache-creation 1.25, cache-read 0.1, output 5, divided by a million. The weights are constants in `store::usage_summary`.

| Field | Is |
| --- | --- |
| `since`, `until`, `tz_offset_minutes` | The window as resolved, and the offset the days were cut with |
| `total`, a `UsageBreakdown` | `units`; `subagent_units`; `above_200k_units` and `above_300k_units` (units of turns whose input, cache-creation and cache-read tokens add to more than that); `by_token` `{ input, cache_creation, cache_read, output }`; `by_model`, `by_first_tool` (`none` where a turn called nothing) and `by_bash_use` (only turns whose first call was a shell command), each `{ name, units }[]` largest first; `by_day` `{ day, units, by_model }[]` oldest first, a day with no turn left out; `hour_grid`, seven rows Monday first of eight three-hour blocks from local midnight |
| `sessions` | The thirty largest Jobs by units: `session_id`, `peak_context`, `subagent_share`, `models`, and `usage`, a `UsageBreakdown` of its own |
| `rest` | The Jobs past thirty as `{ sessions, usage }`; left out where there are none |
| `prior` | The equal period before `since`: `since`, `until`, `units`, `subagent_share`, `above_200k_share`, and `last_day` and `day_before`, `{ day, units }` for the last two whole local days before `until` |

`by_model`, `by_first_tool` and `by_day` each add up to `units`, as do `by_token` and `hour_grid`. The plan's windows are `get_plan_usage`'s and are not repeated.

**Skew.** A Fleet that predates this answers 404 and a Bridge that predates it never asks.
