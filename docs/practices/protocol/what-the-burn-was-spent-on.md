# What the burn was spent on

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 4 Nov 2026. What the burn of the last 24 hours, 7 days and 14 days went on, and who spent it, for the Usage page's Spent on view.
**Additive only**: one operation and the types it carries, and one field on `TokenUnits`; no fact, event or field on `SessionRecord`.
Bridge's half is `packages/protocol/src/usage-spent.ts`, written by hand.
The sums are made in SQL (`crates/store/src/usage_spent.rs`) so Bridge is sent figures and never rows.
**Names and counts only**; titles are not carried, and Bridge joins `session_id` to the ones it holds.
The parts and how a turn is shared among them are `docs/concepts/session/where-the-tokens-went.md`, *What the burn was spent on*.

| Where | Side | Carries |
| --- | --- | --- |
| `get_usage_spent` (`GET /usage/spent?until=`) | Bridge reads | `UsageSpent`. `until` is optional RFC3339 and defaults to now. 422 `fleet.session_unnamed` for an `until` that is not a time |

| Field | Is |
| --- | --- |
| `until` | The end of every window, as resolved |
| `windows` | Always three, shortest first: `day`, `week` and `fortnight`, each `{ id, since, until, total, jobs }` |
| `total`, a `SpentSplit` | `on` `{ start_up, reads, own_words, output, cold_cache, announced, other_results, messages, background }` and `by` `{ main, subagents, background }`, in units |
| `jobs` | The thirty largest Jobs by units, largest first, each `{ session_id, split }` |

`on` and `by` each add up to the window's units, which are `get_usage_summary`'s `total.units` over the same span, and `by.subagents` is its `subagent_units`.
`get_usage_summary` weighs the model and a cache write's lifetime the same way, and `by_token` gained `cache_creation_1h`, the writes held an hour, with `cache_creation` now the 5 minute ones.

**Skew.** A Fleet that predates this answers 404, and the page offers no Spent on view; a Bridge that predates it never asks.
