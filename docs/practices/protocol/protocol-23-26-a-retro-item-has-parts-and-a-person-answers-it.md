# Protocol 23.26: a retro item has parts, and a person answers it

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

The owner, 4 Oct 2026, after Job 3's retro was rejected as prose nobody could act on and wrong
about whose fault it was. `../concepts/retro.md`, *Items* and *Agree and disagree*, is the concept.

**All additive.** Two routes, one query parameter and a handful of optional fields.

| Change | Where | Absent or older |
| --- | --- | --- |
| `id` | `RetroItem`, `Lesson` | Always present: the Job's id, a hyphen and the item's place in its retro |
| `title`, `what`, `fix` | `RetroItem`, `Lesson` | An item kept before 23.26, which has `statement` alone |
| `state` | `Lesson` | Always present: `open`, `agreed`, `accepted` or `discarded`. Every item kept before starts `open` |
| `job_proposed` | `Lesson` | Absent until agreeing proposed a Job for the item |
| `paths` | `RecordCheck` | A failure that names no file, or one the Drone ran itself |
| `GET /lessons?state=` | `list_lessons` | Absent is `open`; any other word than the four is a 400 |
| `POST /lessons/:lesson_id/agree` | `agree_lesson` | New. Answers the `Lesson`. `agent_access` is `Helm only` |
| `POST /lessons/:lesson_id/disagree` | `disagree_lesson` | New. Answers the `Lesson`. `agent_access` is `Helm only` |

**`statement` stays**, and on an item written since 23.26 it repeats `what`, so a Bridge that
predates the parts still has a sentence to draw. **The listing's default changed in meaning, not in
shape**: before 23.26 it listed every item, and it now lists the `open` ones. A 23.22 Bridge
never answers an item, so every item it reads is open and it sees no difference.

**Agreeing**, by where the fix lands. `manifest` proposes a Job at the approval gate on the
repository the item's own Job worked on, and `armada` on Armada's own repository, the Manifest
named `armada`. Both go through `propose_from_request`, with the item as the request. `kit`
proposes nothing and the state becomes `accepted`. An item that is not `open` answers with the
state it stands in and proposes nothing, so agreeing twice makes one Job.

| Refused | Code |
|---|---|
| No item has that id | 404 `fleet.no_such_lesson` |
| An item kept before 23.15, which names no place its fix lands | 422 `fleet.lesson_names_no_place` |
| A fix in Armada, where this Fleet does not serve the Manifest `armada` | 422 `fleet.lesson_armada_not_served` |

A refusal from the proposer itself is returned as it is, and the item is open again.
**Disagreeing** keeps the row as `discarded` and never takes back a Job already proposed.

**Store V105** adds `title`, `what`, `fix`, `state` and `job_proposed` to `job_retro_items`. Existing
rows read `open`.

**The record gains a fact.** A gate failure that names a file carries `paths`, each file with
`named_in_drone_calls`: whether a tool call of the step's Drones names it. `false` says the failure
did not come from the Drone's own calls. It is handed to the retro call and drawn nowhere.

**The retro is written on its own model.** `ARMADA_RETRO_MODEL` overrides it and the default is
`sonnet`, from `crates/config/settings.toml`'s `retro-model`. The Judge's dial is not moved.
