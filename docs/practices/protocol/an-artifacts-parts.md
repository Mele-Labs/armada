# An artifact's parts: `add_part`, `answer_part`, `get_artifact_page_source`, and `parts` on `JobArtifact`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 14 Nov 2026. One artifact that grows: a Job adds the parts it needs and he answers each part that asks on its own. `docs/concepts/artifact.md`, *Parts*.
**Additive, with one field on an existing type read differently**: three operations, three routes, their body and answer types, `parts` on `JobArtifact`, and `state` on a scene node.
No event kind is new.
Bridge's half is written by hand in `packages/protocol/src/job-artifacts.ts` where a client needs it, and Classic does not call any of it. Native draws the parts and builds the page reader.

| Where | Side | Carries |
| --- | --- | --- |
| `add_part` (`POST /sessions/artifacts/part`, `agent_access = "Yes"`) | The Job writes | `AddPart` `{ title, part: { id?, kind, ... }, asks?, for?, links?, session_id? }`. `kind` is `words` (`text`), `takes` (`takes: [{ id, title?, scene? }]`, `question?`, `form?` of `pick`, `rank` or `score`, `parts?` for a score) or `buttons` (`actions`, each a page's action). Answered with the `SessionRecord`, and through the agent's door a receipt of `session_id` and `part` |
| `show_prototype` (`POST /sessions/artifacts/prototype`, `agent_access = "Yes"`) | The Job writes | `ShowPrototype` `{ title, scene, state?, for?, session_id? }`. `scene` is a plain file name under the Job's `$ARMADA_SCRATCH/scenes/`, and a path, `..` or a leading dot is refused. Fleet reads it and the `<fixture src="...">` files beside it (scene at most 64 KB, scene and fixtures 256 KB together, a cap of the prototype's own: the other parts weigh at most 48 KiB together) and keeps a `prototype` part under the scene's name. Answered with the `SessionRecord`, and through the agent's door a receipt of `session_id` and `part`. Fleet watches the scratch folders, and a save of the scene or a fixture puts the part again as its next revision with the same `scene_state`, and publishes `session.changed` |
| `answer_part` (`POST /sessions/artifacts/part/answer`, `agent_access = "No"`) | Bridge writes | `AnswerPart` `{ session_id, artifact, part, drawn, how, take?, words?, notes? }`. `how` is `approved`, `sent_back` or `picked`. Answered with the `JobArtifact` as it now reads |
| `get_artifact_page_source` (`GET /sessions/artifacts/page/source?session_id=&artifact=&revision=`, `agent_access = "Bridge only"`) | Bridge reads | 200 `Content-Type: text/html; charset=utf-8`: the kept revision's bytes as the Job wrote them, with no policy, kit or record added. `revision` is optional, the newest where left out |
| `JobArtifact.parts` (`get_job_artifacts`) | Fleet writes, Bridge reads | `ArtifactPart` `{ id, kind, asks, state, revision, drawn, added, answer? }` and the kind's own fields flattened in: `drawing` `scene`, `words` `text`, `takes` `question`, `form`, `takes: [{ id, title, scene? }]` and `parts`, `buttons` `actions`, `page` `path`, `prototype` `scene`, `markup`, `fixtures: [{ name, text }]`, `scene_state?` and `trouble?`. **`scene_state` is not `state`**, which is where the part stands. `state` is `open`, `approved`, `sent_back`, `picked` or `withdrawn`. `answer` is `{ words?, take?, notes, at? }`. Absent where the artifact holds one part and no other, as it was |
| `SceneNode.state` (`show_sketch`) | The Job writes, Bridge reads | `done`, `running`, `checking`, `waiting` or `failed`, optional, on any node. Another word is refused with the list |
| Refusals | Fleet | 422 `fleet.part_refused` (every broken rule at once), `fleet.part_unnamed`, `fleet.part_unknown`, `fleet.part_not_open`, `fleet.part_answer_empty`, `fleet.part_how`, `fleet.artifact_not_held`, `fleet.artifact_revision_unknown`, `fleet.session_unnamed`; 409 `fleet.part_redrawn` |

**A Needs you item may name an artifact.** `artifact` is an optional field of an item in `waiting_for` (`WaitingInput`), the artifact's id or its title, and `WaitingItem.artifact` carries the id.
It is checked against what the Job holds and refused as `fleet.artifact_not_held` where it holds none, and Bridge opens the artifact on a press.
Absent where the item names none, so a client that does not know it reads the item as before.
`docs/concepts/session/waiting-on-the-person.md`.

**`drawn` is the revision he read.** A part put again is a new revision with a new `drawn`, and an answer that carries the old one is a 409, so he never answers what the Job has replaced.
**A takes part is a vote.** `how` `approved` or `sent_back` on it is refused with `fleet.part_how`, saying to send a pick, and a `rank` or `score` part is cast with `cast_page_vote`.
**`form` stays readable**: it is the first part's kind as `sketch`, `draft` or `page`, so a client that reads no parts still draws the artifact.
**A part's id is its address.** A sketch, draft or page that joined is named by its own id, `sketch:<title>`, `draft:<title>` or the page's path, and a part `add_part` made by the id the Job gave it.
Votes and presses on a part name the artifact's id and the part's id.

**The event stream.** No new event: an add or an answer publishes `session.changed` on the Job once, as an answer to a sketch does.
Nothing here adds a stream or a per-part event, so the unmeasured sink risk is neither better nor worse.

**Skew.** The routes are new, so the protocol ID moves and a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
A `JobArtifact` with `parts` reads on a Bridge that does not know it as the artifact without them.
