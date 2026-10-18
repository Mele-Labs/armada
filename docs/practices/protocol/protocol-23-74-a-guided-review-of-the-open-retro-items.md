# Protocol 23.74: a guided review of the open retro items

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/retro.md`, *Reviewing*. **Additive only**: two operations, both `Helm only` like `agree_lesson`, and no change to a type that exists. Neither stores anything.

| Where | Carries | Notes |
| --- | --- | --- |
| `review_lessons` (`POST /lessons/review`) | `ReviewLessons`: `manifest_id?`; the body may be left out. Answers `LessonReview`: `model`, `entries[]`, `set_aside[]` | One model call over the open items, read as `list_lessons` reads them (up to 200). `entries` are best first and carry no rank. `ReviewEntry`: `lesson_id`, `merged_ids[]`, `reason`. `SetAside`: `lesson_id`, `why`. No open item answers empty with `model` empty and makes no call. 500 `fleet.lesson_review_failed` |
| `ask_lesson` (`POST /lessons/:lesson_id/ask`) | `LessonAsk`: `question`, `history[]` of `AskTurn` `role` (`person` or `fleet`), `text`. Answers `AskLessonAnswer`: `answer` | One model call, answered from the item and the record rows it cites. 404 `fleet.no_such_lesson`; 422 `fleet.lesson_question_refused` for an empty question, one over 2000 characters, more than 20 turns or a turn over 4000 characters; 500 `fleet.lesson_ask_failed` |

**The answer of a review is held to the open items.** An id the model invented is dropped. An id that leads an entry is kept once, so it is taken out of every `merged_ids` and out of `set_aside`. An item set aside needs a `why`. **An open item the model named nowhere is appended to `entries` with the reason `not ranked`**, so nothing a person has not answered goes missing from the queue. Bridge keeps the conversation, because Fleet holds no thread.

**Skew.** A Fleet before 23.74 has neither route and answers 404, so a Bridge that offers the review has nothing to call and shows the plain list. A Bridge before it never calls them. Bridge's half is `packages/protocol/src/retro-review.ts`, written by hand like the rest.
