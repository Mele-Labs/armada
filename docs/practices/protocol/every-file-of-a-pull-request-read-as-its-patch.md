# Every file of a pull request, read as its patch

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 19 Oct 2026. `docs/concepts/session.md`, *Acts on a pull request*.
**Additive only**: one operation, one DTO and one route, and no field of an existing type changes.
The protocol ID moves with `crates/ipc/src`, the operation file, the route table and `packages/protocol/src`, and there is no number to take.

| Where | Side | Change |
| --- | --- | --- |
| `get_pull_request_patches` (`GET /pull_request_patches/:repository/:number`, `Bridge only`) | Bridge asks, Fleet answers | `PullRequestFilePatches`: `files`, each a `PullRequestFilePatch` as `get_pull_request_file` answers it, in the forge's order. A file the forge sent no patch for (binary, or too large) has `patch` absent and `said` the per-file route's sentence |
| `fleet.pull_request_unreadable` | Fleet refuses | A 500: the forge would not answer. A repository Fleet does not serve is a 422 |
| `PullRequestFilePatches` in `packages/protocol/src/pull-requests.ts` | Bridge reads | The hand-written mirror. Bridge reads the operation through one preload entry, `readReviewPatches` (`apps/desktop/src/main/reviews.ts`) |

**Read from the forge with the one `gh api repos/{owner}/{repo}/pulls/N/files` read `get_pull_request_file` makes**, and all of it is kept.
`get_pull_request_file` is a fold over the same parse, so the two answer from one parser.
Neither rides `PullRequestPage`: a pull request's patches can be larger than everything else on it.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
Were a pair to meet anyway, a Fleet without the route answers 404, and `get_pull_request_file` still answers one file at a time.

**The event stream.** Untouched: a request-response read, no event kind, and nothing published. The open risk below is neither better nor worse.
