# The plan's windows, and what the limit holds

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 12 Oct 2026. **Additive only**: one operation, one event kind, one fact, five optional fields.
Bridge's half is `packages/protocol/src/plan-usage.ts`, `sessions.ts`, `protocol.ts` and `events.ts`, written by hand.
Nothing here is the lifeboat's.

| Where | Carries | Notes |
| --- | --- | --- |
| `get_plan_usage` (`GET /plan_usage`) | `PlanUsage`: `windows` (`PlanWindow`: `kind`, `percent_used`, `resets_at?`) and `read_at?` | The newest reading, from whichever job reported last. Empty `windows` is no reading. For display: nothing is held back for a percentage |
| `plan_usage.changed` (event) | `PlanUsage`, whole | Published when a report moves a window a whole percent or changes the windows. A repeat publishes nothing. Served and published, so a Bridge opened later asks |
| `SessionFact::Measured.plan?` (mod to Fleet) | `PlanWindow[]` | Left out off a subscription. An empty report never clears the reading held |
| `SessionFact::Limited` (mod to Fleet) | `resets_at` | The turn ended on the plan's limit |
| `SessionFact::Measured.context_breakdown?` (mod to Fleet) | `ContextBreakdown` | What fills the context window. Names, paths and counts; the ten heaviest MCP tools. Fleet keeps the latest and an absent one leaves it |
| `SessionRecord.context_breakdown?` | `ContextBreakdown`: `categories` (`name`, `tokens`, `kind` of `used`, `free`, `buffer` or `deferred`), `memory_files` (`path`, `tokens`), `mcp_tools` (`name`, `server`, `tokens`), `total_tokens`, `max_tokens`, `autocompact_threshold?` | Mirrored in `packages/protocol/src/context-breakdown.ts`. `kind` is a plain enum Bridge matches on, so a fifth value is a change to tell every reader |
| `SessionRecord.held_until?` | an instant | The job is held for the limit and is woken a minute after it |
| `JobSummary.paused.until?` | an instant | The Job's Drone was stopped by the limit; admission starts it again a minute after |

**The event stream.** `plan_usage.changed` is one small message per moved whole percent, through the shared drop-oldest backlog, and a mod reports at most every ten seconds a session.
It adds no queue and assumes no delivery is complete: a Bridge that missed one reads `get_plan_usage` on the resync that follows a `Missed`.
It leaves the unmeasured sink risk where it was.
Held sessions and Jobs add no event of their own; they ride `session.changed` and `job.paused`.
