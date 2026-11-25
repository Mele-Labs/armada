# The waste in the burn

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 18 Nov 2026. The burn Fleet can name as avoidable over the last day, week or fortnight, by kind, for the Usage page's Waste view and for an agent.
**Additive only**: one operation, one route and the types it carries; no event, no error and no field on an existing type.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `classic-final:packages/protocol/src`, and there is no number to take.
Bridge's half is `classic-final:packages/protocol/src/usage-waste.ts`, written by hand.
The sums are made in SQL and in one pass over the requests that could be waste (`crates/store/src/usage_waste.rs`), so Bridge is sent figures and never rows.
**Names and counts only**; a Job is its id, and Bridge joins `session_id` to the titles it holds.
The one input named is the file a Read touched, in `causes`.
Kept beside it, and never sent, are the lines a Read asked for and the file an edit changed.
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
| `kinds` | Largest first, a kind with nothing left out: `{ kind, title, fix, units, occurrences, jobs }`. `kind` is `subagent_context`, `main_context`, `cold_cache`, `fresh_starts`, `repeat_reads` or `big_results`, and a later one is a new id, so a client draws a `kind` it does not know from its `title` and `fix` |
| `title`, `fix` | Fleet's sentences for the kind: what it is, and one act that cuts it |
| `occurrences` | The requests (for `fresh_starts`, the starts; for `repeat_reads`, the repeat reads; for `big_results`, the results over 20 KB) the kind counted. The later requests that read a result again add units and no occurrence |
| `causes` | The Job's three largest causes, largest first: `{ name, units }`. For `repeat_reads` a `name` is a file path as the Read named it, and for `big_results` a tool's name, such as `mcp__armada-fleet__waiting_for`. Absent for every other kind, and `units` adds up, before the cut, to the Job's `units` |
| `jobs` | The thirty Jobs the kind hit most, largest first: `{ session_id, units, occurrences, peak_context?, causes? }`. `peak_context` is the most context one counted request held, and is carried by the two context kinds only. `causes` is carried by `repeat_reads` and `big_results` only |

**Fresh for five minutes.** Fleet keeps each window's answer in memory and serves it for 5 minutes from when it was made, so a read inside that time is the same answer, with the same `until`, and a client that asks every minute sees it move every fifth.
The sums are made on a connection of their own that only reads, so a read made after the five minutes does not hold up any other request.
A refusal is not kept. Nothing drops the answer before its five minutes are out.

**Skew.** A Fleet that predates this answers 404, and the page offers no Waste view; a Bridge that predates it never asks.

**The kinds this adds.** `repeat_reads`, "Same file read again": a Read of lines of a file the same thread already read (the lines overlap, where no lines is the whole file), with no compaction and no edit of the file by that thread between, costed as the repeat result's tokens (bytes over 4) out of the input and writes of the request that carried it, and again as the same tokens read from the cache by each later request of the thread until its next compaction.
`big_results`, "Tool results over 20 KB": a result over 20 KB that is not a repeat read, costed the same for the bytes past 20 KB, with the later reading too.
In a request's burn they come after the cold cache and the fresh start and before the two context kinds.
The definitions, with what is kept to find them, are `docs/concepts/session/where-the-tokens-went.md`, *Waste*.
