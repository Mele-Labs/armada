# Protocol 23.70: the merge queue on the hub

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Additive only. The repository moved to the forge's merge queue, so `hub.pull_requests` entries say where the queue holds each one, rather than a second list beside them.

| Field | Shape | Notes |
|---|---|---|
| `HubPullRequest.queue` | `HubQueue`: `state`, `position?` | Absent from a Fleet before 23.70, and where the pull request is not in the queue and no auto-merge is waiting on its checks |
| `HubQueue.state` | `waiting_for_ci`, `in_queue` (a state Fleet does not know), `queued`, `awaiting_checks`, `mergeable`, `unmergeable` | **Strict**: Bridge picks a mark from it. `waiting_for_ci` is a pull request with auto-merge on whose own `ci` is still running, so it has no entry yet |
| `HubQueue.position` | 1-based, 1 is next to merge | Absent for `waiting_for_ci` |

**One more forge call a visit**, beside the open pull requests' listing. A forge that will not answer keeps the last reading. Bridge's half is `packages/protocol/src/merge-lines.ts`.
