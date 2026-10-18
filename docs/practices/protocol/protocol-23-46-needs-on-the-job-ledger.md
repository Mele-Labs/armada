# Protocol 23.46: needs on the job ledger

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

*Removed 17 Oct 2026, see "Needs are deleted".* **Additive only**: two operations and the DTOs of `ipc::needs`.
A need is a row in `ledger_attachments`, which 23.43 made, so there is no migration, and nothing an older Bridge reads changes.

| Operation | Carries | Notes |
| --- | --- | --- |
| `act_on_need` (`POST /needs`) | `NeedCall`: `act` (`declare`, `took` or `release`), `branch`, `path`, and `what` or `value` | Answers `NeedAnswer`: the holder's need, whether it was `already` declared, who is `ahead`, and whether a `release` `gave_back`. `agent_access` `No`: a Drone runs `armada need` |
| `list_needs` (`GET /needs?manifest_id=`) | `NeedList`: every standing need, by path and in order | Each line names its holder as a person would: the branch, else the job's title |

**Fleet finds the holder.** `armada need` names a branch and Fleet resolves it to the Job whose branch it is, else the session standing on it, else the branch alone (a `session` holder with id `branch:<name>`).
A `took` for a need nobody declared is `fleet.need_not_declared`, and a call that names nothing is `fleet.need_unnamed`, both 422.

**The ledger gains rows, not columns.** A Job's own slot and branch are written as `slot` and `branch` rows held by the Job, exclusive as a session's are, and `who_owns` answers them.
`kind=need` is not exclusive.
Bridge's half was a file of `packages/protocol`, written by hand like the rest, and is deleted.
