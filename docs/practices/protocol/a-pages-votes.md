# A page's votes: `cast_page_vote`, and `votes` on `JobArtifact`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 14 Nov 2026. His vote on takes a Job's page declares with `<armada-vote>`. `docs/concepts/artifact.md`, *Page votes*.
**Additive only**: one operation, one route, its body and answer types, and one field on an existing type. No event kind is new.
Bridge's half is `packages/protocol/src/job-artifacts.ts`, written by hand. Classic does not call it, and native draws the votes later.

| Where | Side | Carries |
| --- | --- | --- |
| `cast_page_vote` (`POST /sessions/artifacts/vote`, `agent_access = "No"`) | Bridge writes | `CastPageVote` `{ session_id, artifact, revision, vote, result }`: the page's path, the kept revision voted on, the vote's `id` and the `VoteResult` |
| `VoteResult` | Bridge writes, Fleet reads it back | Tagged by `form`: `pick` with `take`, `rank` with `order`, or `score` with `marks`, each `{ take, part, keep }`. Each takes `notes?`, his words by take id |
| Its answer | Fleet writes, Bridge reads | 200 `VoteCast` `{ revision, vote, form, result, at }`. `form` is `pick`, `rank` or `score`, and `result` is the cast with empty notes left out |
| `JobArtifact.votes` (`get_job_artifacts`) | Fleet writes, Bridge reads | The newest `VoteCast` per vote on a page, in the order first cast. Absent where none, and on every form but a page |
| Refusals | Fleet | 422 `fleet.page_vote_unknown`, `fleet.page_vote_form`, `fleet.page_vote_takes`, `fleet.page_archived`, `fleet.artifact_not_held` or `fleet.session_unnamed` |

**The vote is read off the kept revision**, never off the call, so the body names the revision and the vote's id, and Fleet checks the result against the takes and parts that revision declares.
**`form` is said twice** on the answer, on `VoteCast` and as `result`'s tag, so a reader of `votes` can branch without opening the result.

**The event stream.** No new event: a cast publishes `session.changed` on the page's Job.
One cast is one send, so the unmeasured sink risk is neither better nor worse.

**Skew.** The route is new, so the protocol ID moves and a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
A `JobArtifact` with `votes` reads on a Bridge that does not know it as the artifact without them.
