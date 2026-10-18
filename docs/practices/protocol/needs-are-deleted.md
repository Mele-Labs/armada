# Needs are deleted

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 17 Oct 2026. `.claude/decisions/2026-10-15-sessions-absorb-jobs.md`, *Needs are deleted, 17 Oct 2026*. **Removal only**: no operation, event kind, route or field is added. The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and a pair that disagrees is refused. There is no number to take.

**Operations removed (2).** Each has no route, no MCP tool and no Bridge caller; `armada need` was the only caller of the first and an agent's `list_needs` the only reader of the second.

| Operation | Route |
| --- | --- |
| `act_on_need` | `POST /needs` |
| `list_needs` | `GET /needs?manifest_id=` |

`GET /clashes`, `GET /repository_areas` and `GET /slot_touches` stay and are served as before. Their handlers moved from the api's `needs` module to `clashing.rs`, and the trait Fleet implements for them is `Clashing` where it was `Needs`. Neither is wire.

| Where | Side | Change |
| --- | --- | --- |
| `NeedAct`, `NeedCall`, `NeedAnswer`, `NeedLine`, `NeedList` | Fleet removes, Bridge stops reading | `ipc::needs` and its mirror in `packages/protocol` are gone. Bridge never read them |
| `fleet.need_unnamed`, `fleet.need_not_declared` | Fleet stops raising | Both were 422s of `act_on_need` |
| `ChosenHold.kind` and `HoldMoved.kind` `need` | Fleet stops reading and writing, Bridge stops sending | `release_session_holds` acts on `slot`, `branch` and `pr`. A standing `need` row in a store is no longer acted on, so naming one is `fleet.hold_not_standing` |
| `SessionAttachment` kind `need` | Fleet stops writing, Bridge stops reading | A row an earlier build wrote still reads and is passed over. The mod no longer reports one |
| `NotMerged` kind `waiting_behind` | Fleet stops producing | No build since the Job code went produced it. `from_kind` reads a stored one as `refused` |

**Left, because a surface other than a need reads it.** The `ledger_attachments` rows of kind `need` an earlier build wrote stay in the store, unread; no migration drops them, which the Job tables' one later migration will. `Store::standing_in_order` stays: pull request owners read it.

**Left, and not this:** Fleet's single re-run of a hung CI job (`rerun_failed` in `pull_notices.rs`), the Helm `rerun_failed_checks` act, and write-clash detection (`list_write_clashes`), which is what tells two agents they are changing the same file.

**The `needs` commit status** Fleet put on each open pull request, and the forge write behind it (`Delivery::publish_status`, `CommitStatus`, `StatusState`), are gone. It was never required in the ruleset, and `checks.yml` never reported a context of that name.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves. No reader needs to tolerate the old shape. The lifeboat is not touched.

**The event stream.** Neither better nor worse in kind: no event is added or removed. A need's declaration used to move a Job's row, so Fleet publishes fewer `session.changed` events by those, a handful an agent. The bounded broadcast and the "you missed N events" message are still not built and the number is still unmeasured.
