# Job story

**What it is:** The owner's messages in a Job sorted into asks and questions, the reply that answers each question, and the story of each Job read from its asks, what he was shown and decided, what was built and what was handed on.

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
| Asks from outside a Job: words to the Operator, notes, voice | Built: the Operator files each with `file_ask`, under the features it belongs to. A note on a screen is filed by Fleet itself while the Operator is off |
| A question linked to the reply that answers it, `list_job_questions`, `mark_answer_read`, `answer_unread` on the record | Built in Fleet; native draws the Q&A tab, the folded answer and the row's dot later |

## Sorting a message

**Rule.** Every message he sends in a Job is one of five kinds.
A new ask opens an ask, and its kind is a bug, a change request, an idea or a fix.
A revision changes an ask the Job holds, and the ask keeps what he first said.
An answer is a pick, an approval or a reply, joined to the ask it answers.
A question asks the Job something he wants an answer to, such as why, what or how, rather than asking for work.
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

## A question and its answer

**Rule.** A question is linked to the reply of the Job's that answers it.
When the Job's turn ends, each of its open questions is due a read, and so is a message as soon as it is sorted as a question.
The notice loop takes the due questions three at a time and reads each on a task of its own, on the model of `models.dispute`.
The model reads his words and the Job's last message after the question, and answers yes or no.
A reply over 4000 characters is cut to its first 1000 and its last 3000, since an agent concludes at the close.

**Rule.** A yes keeps the link: his question, the agent's message row that answers it, the reply's first prose up to about 300 characters, and when.
The row id is the one `get_job_thread` takes as `before` and `upto`, so a client scrolls the thread to the answer.
A no keeps the reply as read, so the same reply is not asked about again, and the question stays open for the next turn.
The model may answer a bare yes or no, without the label, and markdown around it is read through.
A failed read, or an answer that says neither, is logged with what the model said and tried again on the next passes of the loop, three reads in all, and then waits for the next turn's end.
The count of tries is kept in memory, and a turn's end starts it over.

**Rule.** An answer starts unread, and `mark_answer_read` marks it read when he jumps to it or a client draws it.
The store keeps it, so it holds across a restart, and a read on another Mac is passed on to the Mac that holds the Job.
`answer_unread` on the Job's record says whether any answer of the Job's waits unread, for the dot on its row.
A question he sorts as another kind loses its link.
`list_job_questions` lists a Job's questions, the unanswered first and each group newest first.

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

**Rule.** An ask also carries the Job's own written replies: its text turns in the thread the ask was said in, after the ask's first words and before the next ask's first words, oldest first, each cut to about 600 characters at a word.
Tool calls, empty turns and notices are not replies, and an ask said outside the thread has none.

**Rule.** An artifact goes under the ask he last spoke on before the artifact was first kept.
Fleet records no message an artifact answers, so the time is the one fact there is.
What came before any ask of the Job is listed apart.

**Rule.** `features` lists the features the Job is on, most touched first, for the head of its Story tab.
Each counts the files the Job wrote under it, the Job's commits that named it in a `Feature:` line, and the Job's asks filed under it.
A Job on no feature has none, and the tab draws no section.

**Rule.** `lead` is one line an ask, open ones first, for the done card to draw beside the Job's own summary.
The summary stays on the record, and the record gains no field.

## An ask from outside a Job

**Rule.** Words he says to the Operator, a note he leaves on a screen and a line he dictates reach the Operator as words, and it files each that asks for something with `file_ask`.
Bridge turns speech into text in the composer, so voice has no channel of its own.
Talk, and an answer to a card, are not asks.

**Rule.** It is the same ask row with no Job: `said_in` is empty, the ask opens in the backlog since nothing builds it yet, and it shows on each feature it is filed under.
The Operator names the features, each held by the project's library, and a slug the library lacks is refused.
A note is filed under its id, so the same note filed twice is one ask.

**Rule.** A note arrives with the features Fleet reads from where he left it.
The files its element was drawn from, `source` and `definition` in the note, are mapped by the library's path prefixes, and where a note names no file its component's name is read against the files of each feature's entry points and prefixes.
A note that names neither arrives with no feature, and the Operator chooses from the library.

**Rule.** The Operator being off does not leave a note unfiled.
A note sent to a Job is that Job's, and its message is sorted there.
A note open and sent to no Job for ten minutes is filed by Fleet: one ask with no Job, in the backlog, under the features Fleet reads from where he left it.
Its kind and title come from the sorter a Job's messages go through, and its key is the note's id, so the same note filed twice is one ask.
A note the sorter reads as talk or a question is no ask, and a sorter that fails leaves it a change request.
With the Operator on this does not run, and the Operator files the note as above.

## Open questions

- **[story-sorting-model-dial]** Does sorting a message take a dial of its own beside `models.dispute`? It runs on the dispute model today, so changing that setting changes both reads.
