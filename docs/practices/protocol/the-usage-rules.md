# The usage rules

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 18 Oct 2026. Which token rules Fleet tells a Job about, their limits, and the Jobs it told.
**Additive only**: one operation and the types it carries.
Bridge's half is `packages/protocol/src/usage-rules.ts`, written by hand.
**Names and counts only**; titles are not carried, and Bridge joins `session_id` to the ones it holds.
The rules and their words are `docs/concepts/session.md`, *Where the tokens went*.

| Where | Side | Carries |
| --- | --- | --- |
| `get_usage_rules` (`GET /usage/rules?since=&until=`) | Bridge reads | `UsageRules`. `since` and `until` are optional RFC3339, `until` exclusive and defaulting to now, `since` to seven days before it. 422 `fleet.session_unnamed` for a bound that is not a time |

| Field | Is |
| --- | --- |
| `since`, `until` | The window as resolved |
| `rules` | Always four, in the order `context`, `subagents`, `cache_hit`, `opus_reading`, each `{ rule, on, limit, trips }` |
| `on`, `limit` | The rule's on-off and its limit as settings.json holds them: `limit` is `{ value, unit }`, the unit `thousand tokens`, `percent`, `percent` and `turns` in that order |
| `trips` | The Jobs told in the window, newest first, and empty for a rule that never tripped. A trip is `session_id`, `at`, `figure` (tokens, a percent or a count, by rule), `burn_per_hour_before` and `burn_per_hour_after` |

**`burn_per_hour_before`** is the Job's burn in cost units per hour of active time over the 60 minutes up to `at`, counted as the after is below, so the two compare.
It is there whatever the number of turns in that hour, and 0 where there is one.
**`burn_per_hour_after`** is left out until the Job's turns cover the 60 minutes after `at`, and stays out where the Job made fewer than 5 turns in them.
Both are the cost units of the turns over the Job's **active time**, which is the sum of the gaps between consecutive turns in the hour with each gap counted as at most 5 minutes, so a Job that sat idle does not read as a win.
There is no total over trips: Bridge draws the change.

**Skew.** A Fleet that predates this answers 404 and a Bridge that predates it never asks.
