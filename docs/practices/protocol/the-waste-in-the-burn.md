# The waste in the burn

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 18 Nov 2026. The burn Fleet can name as avoidable over the last day, week or fortnight, by kind, for the Usage page's Waste view and for an agent.
**Additive only**: one operation, one route and the types it carries; no event, no error and no field on an existing type.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/usage-waste.ts`, written by hand.
The sums are made in SQL and in one pass over the requests that could be waste (`crates/store/src/usage_waste.rs`), so Bridge is sent figures and never rows.
**Names and counts only**; a Job is its id, and Bridge joins `session_id` to the titles it holds.
What each kind is, how its cost is counted and the order a request's burn is handed out in are `docs/concepts/session/where-the-tokens-went.md`, *Waste*.

| Where | Side | Carries |
| --- | --- | --- |
| `get_usage_waste` (`GET /usage/waste?window=`, `agent_access = "Yes"`) | Bridge reads; an agent reads | `UsageWaste`. `window` is `day`, `week` or `fortnight` and defaults to `week`. 422 `fleet.session_unnamed` for any other window |

| Field | Is |
| --- | --- |
| `until` | The end of the window, as resolved |
| `window` | `{ id, since, until }`: the window asked for, as resolved |
| `total_units` | The window's whole burn, which is `get_usage_summary`'s `total.units` over the same span |
| `waste_units` | The kinds' units added up. No unit is in two kinds, so it is no more than `total_units` |
| `kinds` | Largest first, a kind with nothing left out: `{ kind, title, fix, units, occurrences, jobs }`. `kind` is `subagent_context`, `main_context`, `cold_cache` or `fresh_starts`, and a later one is a new id, so a client draws a `kind` it does not know from its `title` and `fix` |
| `title`, `fix` | Fleet's sentences for the kind: what it is, and one act that cuts it |
| `occurrences` | The requests (or, for `fresh_starts`, the starts) the kind counted |
| `jobs` | The thirty Jobs the kind hit most, largest first: `{ session_id, units, occurrences, peak_context? }`. `peak_context` is the most context one counted request held, and is carried by the two context kinds only |

**Skew.** A Fleet that predates this answers 404, and the page offers no Waste view; a Bridge that predates it never asks.
