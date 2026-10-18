# A Job moves to another repository

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/session.md`, *A moved job*. **Additive only**: one operation, one DTO, three error codes and a detail on a ledger row that already crosses the wire.

| Where | Side | Change |
| --- | --- | --- |
| `move_session` (`POST /sessions/move`) | Bridge calls, an agent may | `MoveSession`: `session_id`, `manifest_id`. Answers 200 with the new Job's `SessionRecord`. `agent_access` Yes |
| `fleet.no_such_session`, `fleet.no_such_manifest` | Fleet refuses | 404 for a Job or repository Fleet does not hold. `fleet.no_such_session` is 422 as `start_session`'s `fork` answers it and 404 here |
| `fleet.session_move_same`, `fleet.session_move_listening` | Fleet refuses | 409 for a move into the Job's own repository, and for a terminal Job whose mod is still asking |
| `Attachment.detail.moved` on `forked_to`, `forked_from` | Fleet writes, Bridge reads | The id of the repository on the other side. Present only on a move |
| `Attachment.detail.branch` on `forked_from` | Fleet writes, Bridge reads | The old Job's branch or branches, comma-separated. Present only on a move that left one |
| `moveSession` (preload) | Bridge | `moveSession(sessionId, manifestId)`, answering `SessionActed` as `startSession` does |

**Skew.** The ID moves, so a Bridge and a Fleet that differ are refused and the lifeboat serves. Bridge's half is `packages/protocol/src/hosted-sessions.ts`, written by hand. There is no number to take: `armada need` has nothing to order. Attachment kinds and details are open text, so no type changes and a Bridge that has not met `moved` draws the pair as a fork. **The event stream is untouched** beyond the `session.changed` each Job already publishes, so the back-pressure risk below is neither better nor worse. The lifeboat is untouched.
