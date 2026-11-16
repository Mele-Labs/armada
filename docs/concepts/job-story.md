# Job story

**What it is:** The owner's messages in a Job sorted into asks, and the story of each Job read from its asks, what he was shown and decided, what was built and what was handed on.

---

**Kind:** Concept.

**Part of:** [Job](session.md).

Fleet sorts and keeps in `crates/fleet/src/story/` and `crates/store/src/story_asks.rs`.
The wire is `crates/ipc/src/story.rs`, with one file a call in `crates/ipc/operations/`.
The owner approved the design on 16 Nov 2026.

## Status

| Part | State |
|---|---|
| Sorting each message after it is delivered, `list_message_kinds` | Built in Fleet; native draws the chip later |
| His correction of a message, `set_message_kind`, kept as an example | Built in Fleet; native draws the press later |
| Asks, `list_job_asks`, `change_ask` | Built in Fleet; native draws them later |
| An ask built when the Job building it merges | Built |
| An ask handed when its Job starts or messages another Job with it | Built |
| A Job's story, `get_job_story`, and the done card's lead lines | Built in Fleet; native draws the Story tab later |
| A project's asks by feature slug, `list_feature_asks` | Built in Fleet; the feature library files asks under slugs later |
| Asks from outside a Job: words to the Operator, notes, voice | Not here. The Operator files them |

## Sorting a message

**Rule.** Every message he sends in a Job is one of four kinds.
A new ask opens an ask, and its kind is a bug, a change request, an idea or a fix.
A revision changes an ask the Job holds, and the ask keeps what he first said.
An answer is a pick, an approval or a reply, joined to the ask it answers.
Talk stays in the thread and opens nothing.

**Rule.** Delivery writes one row and goes on.
The notice loop takes the rows waiting, three at a time, and reads each on a task of its own.
A failed read drops the row, and the message stays unsorted with no chip.
A row taken by a read that Fleet died under stays unread for good.

**Rule.** The model reads the message, the Job's open asks (id, kind, title), the last six turns, and the eight newest messages he sorted himself.
It answers the kind, the ask kind, the ask joined and a title.
A revision or an answer that names no open ask of the Job is talk.
A new ask with no kind is a change request, and with no title takes his first line.
It runs on the model of `models.dispute`, the cheapest on the roster.

**Rule.** He changes a message's kind with one press, `set_message_kind`.
The change is kept with `sorted_by` set to `owner`, and the newest few ride in the next prompt as examples.
A message he has set is never sorted again, and a read of the model that was running when he set it is dropped.
A message that opened an ask and becomes something else takes the ask with it, unless the ask holds revisions or answers of its own, which Fleet refuses.

## An ask

**Rule.** An ask keeps its kind, a short title, his first words, each revision and answer with his words, the Job it was said in, the Job building it, its feature slugs and its state.
Slugs are plain strings, and an ask on two features shows on both.

| State | Meaning |
|---|---|
| `backlog` | Decided and not started. He sets it |
| `building` | A Job is building it. An ask opens here, in the Job it was said in |
| `built` | A pull request of the Job building it merged after the Job took the ask |
| `handed` | Passed to another Job, which builds it |
| `dropped` | He dropped it |

**Rule.** Fleet moves an ask from what it sees.
Built is read from the building Job's `pr` rows, the way a wait on a pull request reads them.
Handed is read when the building Job starts another Job or messages one: a small model names which of its open asks the words carry.
Nothing the agent says about its own work moves an ask.
He moves an ask to the backlog or drops it with `change_ask`, and changes its kind, title and features there.

## A Job's story

**Rule.** The story is the Job's asks, open ones first, each group oldest first.
Each ask shows how it started, its revisions, what he was shown and decided, what was built, what was handed on and whether it is open.

**Rule.** An artifact goes under the ask he last spoke on before the artifact was first kept.
Fleet records no message an artifact answers, so the time is the one fact there is.
What came before any ask of the Job is listed apart.

**Rule.** `lead` is one line an ask, open ones first, for the done card to draw beside the Job's own summary.
The summary stays on the record, and the record gains no field.

## Open questions

- **[story-sorting-model-dial]** Does sorting a message take a dial of its own beside `models.dispute`? It runs on the dispute model today, so changing that setting changes both reads.
