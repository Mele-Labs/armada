# Protocol 23.77: Start Job on a retro item

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/retro.md`, *Agree and disagree*. **Additive only**: one operation, `Helm only` like `agree_lesson`, and one optional field on two types.

| Where | Carries | Notes |
| --- | --- | --- |
| `start_lesson_session` (`POST /lessons/:lesson_id/session`) | No body. Answers `LessonSessionStarted`: `lesson`, `session_id`, `brief`, `send_failed?` | `agree_lesson`'s sibling: a hosted Job on the item's repository, started as `start_session` starts one, and the brief sent to it as its first message at once, so the Job runs. `brief` is the text that was sent. `send_failed` is Fleet's words for why it was not, where the send was refused, and is absent otherwise |
| `Lesson.session_proposed`, `RetroItem.session_proposed` | The Job id, optional | State `agreed`. At most one of it and `job_proposed` is set. Absent from the wire when none |

**Refusals.** As `agree_lesson`: 404 `fleet.no_such_lesson`; 422 `fleet.lesson_names_no_place`, `fleet.lesson_armada_not_served`, `fleet.lesson_session_names_no_repository`. 422 `fleet.lesson_session_nothing_to_work_on` for a `kit` item. 409 `fleet.lesson_not_open` for an item answered with no Job to hand back.
**A Job that started and whose first message was refused is a 200 with `send_failed` set**: the item is `agreed` with the Job on it, so Bridge can open it and the person can send by hand.
A second press answers 200 with that Job, sets `send_failed` again, and sends nothing.

**One press makes one Job and one message.** The write that moves the item off `open` is the claim. An item already agreed with a Job answers 200 with it.

**Skew.** A Fleet before 23.77 has no route and answers 404, and never sends `session_proposed`, which Bridge reads as absent.
A Bridge before it never calls the route and ignores the field.
Bridge's half is `packages/protocol/src/retro-session.ts`, written by hand.
