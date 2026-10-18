# An agent lists Jobs in a line each

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`list_sessions` through the agent's door answered 696 KB: every Job ever, each with its whole ledger, and the pull request bodies in it.
The door cuts at 64 KiB, and a Helm job that read the cut file saw two of nine live Jobs and said only two were live.
**Additive only**: one query parameter, three types and one field on the door's answer.
No operation is added, so nothing is tagged in `commander_reach.rs`.
There is no number to take: the protocol ID moves with the wire files.
Bridge's half is unchanged, because Bridge and Pocket send no `view` and are answered the row they were.

| Where | Carries | Notes |
| --- | --- | --- |
| `list_sessions` (`GET /sessions?manifest_id=&q=&state=&view=`) | `view` is `full` (absent) or `summary` | `full` is `SessionList`, as before. `summary` is `SessionSummaries` |
| `SessionSummary` | `id`, `title?` (160 characters at most), `manifest_id?`, `origin`, `state`, `turn?` (`idle` or `working`), `last_turn_at?`, `last_seen_at`, `slot?`, `branch?`, `pull_requests` (`number`, `title`, `state?`), `jobs` | What is **standing** only: a merged pull request or a slot given back is history, and `get_session` has it. `turn` is absent on a terminal Job, which reports where a turn ended and not where one began. `jobs` are the Jobs it holds that have not finished, by id |
| The agent's door, `list_sessions` | Asks for `view=summary`, and for `state=live` where the call names no `state` | `view` is not among the tool's parameters, so an agent cannot ask for the wide row. `state=ended` is its way to a finished Job |
| `door::Answer.narrowed_by` | The tool's query parameters | Said in a cut answer: *To be answered less, call this tool again with `q` or `state` set*, after the route that serves it whole. A tool with no query parameter says *Ask a narrower question if you can* |

**A narrow row beside the wide one, and not a cut of it.** The summary has no field that grows with a Job's history, so how many Jobs fit is a count of lines and not a hope.
`crates/api/src/tests/door_sessions.rs` holds the bound against a Fleet whose Jobs each have a long ledger of pull requests with their bodies.
A truncation would still have been a guess about which half to keep.
The default belongs to the door and not the route, because a route that answered a summary unasked would change Bridge's list.

**Who reads the route.** Bridge's main process (`apps/desktop/src/main/sessions.ts`) and Pocket send no `view` and are unchanged.
Fleet's own readers (`read_thought`, the routing of a thought) call the daemon's `list_sessions` directly and never this route.
