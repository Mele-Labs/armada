# A page's actions: `press_page_action`, and `presses` on `JobArtifact`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 14 Nov 2026. His press of a button a Job's page declares with `<armada-action>`. `docs/concepts/artifact.md`, *Page actions*.
**Additive only**: one operation, one route, its body and answer types, and one field on an existing type. No event kind is new.
Bridge's half is `packages/protocol/src/job-artifacts.ts`, written by hand. Classic does not call it, and native draws the buttons later.

| Where | Side | Carries |
| --- | --- | --- |
| `press_page_action` (`POST /sessions/artifacts/press`, `agent_access = "No"`) | Bridge writes | `PressPageAction` `{ session_id, artifact, revision, action, words? }`: the page's path, the kept revision pressed and the action's `id` |
| Its answer | Fleet writes, Bridge reads | 200 `PagePress` `{ revision, action, item?, act, at, did }`. `act` is `message`, `start` or `answer`. `did` is tagged by `kind`: `sent`, `started` with `job`, or `answered` with `item`, the ask's id |
| `JobArtifact.presses` (`get_job_artifacts`) | Fleet writes, Bridge reads | Every `PagePress` on a page, oldest first. Absent where none, and on every form but a page |
| Refusals | Fleet | 422 `fleet.page_action_unknown`, `fleet.page_act_refused`, `fleet.page_ask_not_open`, `fleet.page_archived`, `fleet.artifact_not_held`, `fleet.session_unnamed`, or the refusal of the start or the answer it made |

**The action is read off the kept revision**, never off the call, so the body names the revision and the action's id and nothing of what it does.
**Fleet keeps the list of acts**, and an act off it is refused at the press and named to the Job when the page is kept.

**The event stream.** No new event: a press publishes `session.changed` on the page's Job, and a start publishes what any start does.
One press is one or two sends, so the unmeasured sink risk is neither better nor worse.

**Skew.** The route is new, so the protocol ID moves and a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
A `JobArtifact` with `presses` reads on a Bridge that does not know it as the artifact without them.
