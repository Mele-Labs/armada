# The forge's merge verdict, and one commit's files

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 16 Nov 2026. `docs/concepts/session.md`, *Acts on a pull request*.
**Additive only**: one enum, two fields, one operation and one route, and no existing field changes.
The protocol ID moves with `crates/ipc/src`, the operation file, the route table and `classic-final:packages/protocol/src`, and there is no number to take.

| Where | Side | Change |
| --- | --- | --- |
| `PullRequestMergeState` | Fleet writes | GitHub's `mergeStateStatus` word for word: `clean`, `blocked`, `behind`, `dirty`, `unstable`, `has_hooks`, `draft`, `unknown`. Only `clean`, `unstable` and `has_hooks` merge |
| `PullRequestState.merge_state` | Fleet writes, Bridge reads | Read with the pull request's facts. Defaults to `unknown` when absent |
| A `pr` row's `merge_state` detail | Fleet writes, Bridge reads | The same word, on every row the ledger follows, so a Pull Requests row can stop offering Merge on a blocked pull request |
| `PullRequestPage.merge_state` | Fleet writes, Bridge reads | Read with the page. Defaults to `unknown` when absent |
| `get_pull_request_commit_patches` (`GET /pull_request_commit_patches/:repository/:number/:commit`, `Bridge only`) | Bridge asks, Fleet answers | `PullRequestFilePatches`, the shape `get_pull_request_patches` answers, for one commit against its first parent |
| `fleet.pull_request_commit_invalid` | Fleet refuses | A 422: `commit` is not seven to 64 hex characters. A forge that would not answer is a 500 `fleet.pull_request_unreadable` |

**Why the verdict and not the checks.** Checks pass on a pull request still waiting on a review its base branch asks for, so an offer worked out from them is one the forge refuses.

**One commit is read from `repos/{owner}/{repo}/commits/{sha}`**, which lists its files under `files` as a compare does, so the compare's parse reads it.
Fleet does not check that the commit is one of the pull request's: `number` only labels the answer.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.

**The event stream.** `merge_state` rides `session.changed` as one more short detail on a `pr` row, and the route publishes nothing. The open risk below is neither better nor worse.
