# Sessions are called Jobs: four tools listed under new names

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

17 Oct 2026, the owner's rename (`.claude/decisions/2026-10-15-sessions-absorb-jobs.md`, *Sessions are called Jobs*). **The wire keeps `session`.** Field names, `session_id`, event kinds, routes, operation keys and error codes are unchanged, so no Bridge field moves and no migration is owed. What moves is the text an agent reads.

| Tool an agent lists | Operation it answers for | Still answered by |
|---|---|---|
| `start_another_job` | `start_another_session` | `start_another_session` |
| `fork_this_job` | `fork_this_session` | `fork_this_session` |
| `list_jobs` | `list_sessions` | `list_sessions` |
| `move_job` | `move_session` | `move_session` |

**Aliases live in the door, not in the inventory.** `ipc::door::RENAMED` pairs each Job name with the operation, which keeps its Session name as its key in `operations/`, its route and its daemon method. `tools/list` names the Job name once; `tools/call` takes either, so a running agent that listed its tools before the rename and a mod that names the old one keep working. Permission rules, Helm's reach and `QUERIES` read the operation and need no second row.

**Descriptions say Job.** Every `description` in `operations/` and the door's parameter descriptions read Job where they read Session. Notes that discuss both the old Job and the Session were left as written, since renaming one word there would merge two concepts. `docs/concepts/session.md` is still the concept's file.

**Skew.** The protocol ID moves with `ipc/src/door.rs` and the descriptions, as it does for any wire file. There is no number to take.
