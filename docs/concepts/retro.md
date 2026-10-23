# Retro

**What it is:** What got in the way while one Job ran, whose way it got in —
the agent's, the owner's or Fleet's — and where each fix lands, written on a press
and once when the Job ends, and read on the Retros page.

---

**Kind:** Entity. Owned by [Job](session.md); a Job has several.

## What it is

A retro is a list of items. Each item names whose way it got in and where its
fix lands, says in one sentence what got in the way, and cites the rows of the
Job's record that show it. A Job is a conversation with a person, so its
retro is read from what Fleet kept of the conversation. The owner's decision, 8
Oct 2026, after walking the mock.

| Who | Means |
| --- | --- |
| `agent` | The Job's agent was slowed or stopped |
| `owner` | The person who owns the Job had to step in, wait or redo something |
| `fleet` | Armada itself cost the Job something it should not have |

## Where the fix lands

**Every item names where its fix lands, apart from whose way it got in.** The
owner's decision, 3 Oct 2026. Who it cost and where it is fixed are two
questions: a command an agent was refused cost the agent, and is fixed in Kit.

| `lands_in` | Means | For example |
| --- | --- | --- |
| `armada` | Armada itself: Fleet or Bridge | Fleet ruled a red without confirming it; a restart lost a subagent |
| `kit` | The tool set: Skills, MCP servers, sub-agents, agent files, plugins, commands, the allowlist, the models list ([Kit](kit.md)) | A command an agent was refused, or had to ask a person to allow |
| `manifest` | The repository the Job worked in: its `armada.yml` (Checks, Commands, places, when), its tests and its code ([Manifest](manifest.md)) | Browser tests on a fixed 15 s timeout; `test` running every Rust test on a docs-only edit |

**An item names exactly one.** Where a fix spans two places, the retro writes
two items. The Retros page narrows by it, and **an item written before
23.15 names none**: it reads with the field absent and is listed under All
alone, never given a place after the fact.

## Items

**An item has parts, so it can be read at a glance and acted on.** The owner's
decision, 4 Oct 2026, after a retro was rejected as a wall of prose that
named no root cause and blamed the wrong party.

| Part | Is |
| --- | --- |
| `title` | A headline of about eight words |
| `what` | One or two short sentences: what happened, and to whom |
| `fix` | One sentence naming what to change |
| `who`, `lands_in`, `evidence` | As above |
| `id` | Names the item for an act. The Job's id, `-r`, the retro's number, a hyphen and the item's place in its retro |

**`statement` stays, and repeats `what`** on an item written since, so a reader
that predates the parts still has a sentence. **An item written before the parts
has `statement` alone**: they read absent, and nothing writes them after the
fact.

**An item is kept whole or not at all.** One missing a part, one with a dash in
any of the three, one citing nothing the record holds and one naming no place
are each dropped alone, and the items beside them are kept.

## Agree and disagree

**A person answers each item, and starting a Job is the one place a retro acts.**
Every item starts `open`.

| State | Means |
| --- | --- |
| `open` | Nobody has answered it. What the Retros page lists by default |
| `agreed` | A Job was started for it, and `session_proposed` names that |
| `discarded` | Disagreed with. The row stays |

**Start Job agrees with an item by starting a Job on it.**
`start_lesson_session` sends the Job the item as its first message. A
`manifest` fix goes to the repository the item's own Job was started in, and
an `armada` fix to Armada's own repository, the Manifest named `armada`. Where
Fleet does not serve that one it refuses with `fleet.lesson_armada_not_served`
and the item stays open. The other refusals are `fleet.no_such_lesson` and
`fleet.lesson_names_no_place`, and the first press wins: the write that
moves the item off `open` is the claim, so two presses make one Job and send
one message. A `kit` item is refused with `fleet.lesson_session_nothing_to_work_on`,
since Kit is no repository. An item kept before `lands_in` names no place, so it
is refused: where its Job would go is not Fleet's to guess.

The brief is the item: its title, what happened, the fix and a line naming the
source Job's handle, written to the agent who will do the work. **It is sent
at once**, so the process starts and the turn runs. A Job that starts and
refuses the message cannot be taken back, so the item stays `agreed` with the
Job on it and the answer is still a 200, with `send_failed` holding Fleet's
words for why; pressing again answers with that Job, sets `send_failed`
again, and sends nothing. An item already answered, or discarded, answers 409
`fleet.lesson_not_open`.

**Disagreeing never takes back a Job** already started.

## The record it is read from

**The record is assembled from what Fleet already keeps, on every read.** It is
mechanical: it reads nothing a model wrote and stores nothing new, except the
agent's note below. It covers what Fleet kept of the Job since the last retro
ended.

| Row | Read from |
| --- | --- |
| `asks` | The ask rows on a hosted Job's thread: what was asked, the answer and how long it waited |
| `refusals` | Tool results the agent CLI's transcript shows were refused: a hook's `Refused`, the harness's denial, the person declining |
| `restarts` | A hosted Job's process resumed, reattached after Fleet came back, or gone while a turn ran |
| `failed_tools` | The other tool results that errored, each joined to its call: `tried` is the call, `because` the error |
| `corrections` | Every message the person sent, unfiltered. The call reads which of them redirected the agent |
| `subagents` | What went wrong in each subagent's own transcript |
| `notes` | What the agent said got in its way, below |

Every row carries a `cite`, `ask:1` and the like, unique within the record.

**The agent is asked once, and only between turns.** Where the Job is
hosted, its process is running and its turn is idle, Fleet sends it one fixed
message from Fleet, "What got in your way since <time>? One line per thing, or
'nothing'.", and waits a bounded time for the reply, which becomes the note. A
Job mid-turn, asleep or ended is written without. The message is Fleet's
and is never read back as one of the person's corrections.

## The retro call

**One model call per retro, on a model of its own.** It takes the Judge's client
and budget, reads no repository, and is handed only the record. Its instructions
carry the table under *Where the fix lands*, in plain words, and say this is a
conversation with a person. An item citing a row the record does not hold loses
that citation; one citing nothing the record holds is dropped, and so is one
naming no place its fix lands or a place that is not one of the three. **The
place is never defaulted.** An item that will not read is dropped alone, and the
items beside it are kept.

**The model is Sonnet**, the owner's decision on 4 Oct 2026 after the cheap
tier wrote an unreadable retro. It is `settings.retro-model`, overridden by
`ARMADA_RETRO_MODEL`, and the Judge's own dial does not move with it. The call
is not metered: an unwatched call reports no cost, so it counts against no cost
cap, as the Judge's calls never have.

**The instructions refuse to guess.** They say, in plain words:

- Name a cause only where the record shows one. "Unclear" is an allowed
  answer, and a symptom may be written as a symptom.
- Blame the agent only for what the record shows the agent did, its own tool
  call or its own words. A tool that failed for a reason outside the agent's call
  is not the agent's.
- A fault of Armada's goes to `who: fleet` and `lands_in: armada`.
- One item per cause. Fewer, truer items beat many.
- Write the three texts in plain words: short sentences with concrete facts,
  plain verbs, no dashes, no "not X but Y", no lists of three for rhythm, no
  stock words, no vague "associated with", no closing line, no first sentence
  that restates the headline.

## When it is written

**A Job's retro is written on a press** (the Retro icon on the Job header
and on each row of Jobs), **and once when the Job ends.** Each covers from
where the last ended (`covers_from`, `covers_to`).

| State | Means |
| --- | --- |
| `pending` | The Job has no retro yet |
| `written` | The call answered; `items` may be empty |
| `failed` | The call failed or its answer would not read; never tried again |

A press writes the retro and answers when it is written. For a terminal Job
Bridge shows the writing mark until then and opens it; for a hosted Job it
draws the retro as it is written, in *A Job's retro, in its thread*. **A second
press while one is writing is refused** (409), and **a Job with nothing since
its last retro answers with that retro**, or is refused (422) where it has none. A
Job that ends is written a retro, and one with nothing new, or whose call
failed, is given up on and not tried again.

**A hosted Job shows its retro being written, in its thread.** The owner's
request, 12 Oct 2026: after the press, a person should see the retro come, and
not a mark until it is done. Once Fleet accepts a press it puts a `retro` row on
the thread and replaces it by id as the writing goes. A refused press adds none,
and nor does a press that is answered with the last retro. The row stays when it
is done, so a Job reopened later shows it.

| `stage` | Means | What else moves |
| --- | --- | --- |
| `asking_agent` | The live, idle agent was asked what got in its way, and Fleet is waiting up to 90 seconds. **Absent where no agent is asked** | |
| `reading_record` | Fleet is gathering what it kept of the Job | |
| `writing` | The model is writing | `characters`, the answer so far, and `items`, as they settle |
| `written` | Kept | `retro` is its number, for `get_session_retro`. `items` are empty: the kept retro's are the ones to read |
| `failed` | Not kept | `why` is Fleet's words, the refusal's message |

`stage_since` is when it reached the stage and `at` when the press was accepted.
**An item's field appears once its value has ended**, in the order the model
writes them, so a title is never half a sentence (`PartialRetroItem`: `title`,
`what`, `fix`, `who`, `lands_in`, each absent until then). They are the model's
items **before Fleet holds them to the record**, so the kept retro can have fewer:
an item citing nothing the record holds, or with a dash in its text, is drawn
while the model writes and not kept. A row is replaced at most four times a
second; a stage change and the last state are never held back.

**The row is a courtesy and no part of the retro.** The call is the same, the
budget and the model are the same, and a call that fails fails as it did. A row
that was never updated, or a client that missed every frame, keeps the same retro.
A row a Fleet restart left in `asking_agent`, `reading_record` or `writing` is
read as `failed` with "Fleet restarted while it was written", and kept so.
The retro written when a hosted Job ends gets a row the same way. **A terminal
Job has no rows of Fleet's**, so it is written as before, with the writing
mark until the press answers.

## Where Bridge draws it

| Where | What |
| --- | --- |
| **Retros**, a tab (glyph `rewind`, Proposed) | `list_lessons`, narrowed to the picker's pick and every repository on All, under tabs `All · Armada · Kit · Manifest` by where each fix lands. All is the default, an item with no `lands_in` is under All alone, and the tab is remembered for the viewer. Each item is a card; a Job's row reads `Title · s-xxxxxxxx` and opens that Job's retro |
| **Retro**, in a Job's thread | One card that stays, below |

Both read when they open and again when the window regains focus, because
nothing on `/events` says a retro was written. What Fleet serves for acting is
*Agree and disagree*.

**An item reads top to bottom:** one arrow from whose way it got in (`Agent`,
`You`, `Fleet`) to where its fix lands (`Armada`, `Kit`, `Manifest`), both ends
words with their marks beside them and each its own hue, then the `title`, what
happened, the `fix` under the label `What would change`, and the answers.
The cited record rows sit behind an `Evidence` control.
`packages/components/src/compositions/LessonCard`.

**The hues are not status hues.** Whose way: Agent `--tool-change`, You
`--tool-look`, Fleet `--tool-run`. Where: Armada `--accent`, Kit `--diff-add-fg`,
Manifest `--diff-del-fg`. Proposed, for the owner to rule on.

**The buttons carry words for what they do, not the wire's names.** An item whose
fix lands in Armada or the Manifest has `Start Session` and `Reject change`; a Kit
item has `Reject change` alone, since Kit is no repository. The acts are still
`start_lesson_session` and `disagree_lesson` on the wire. The owner's request, 12
Oct 2026: a person who wants to talk the fix through with an agent starts a
Job on it. **It begins work at once**, and its tooltip says so: Fleet sends the
item's request as the Job's first message (the owner's decision, 12 Oct 2026,
over a draft left in the message box). Bridge opens that Job, which shows the
sent message and the running turn, and puts nothing in the composer. Where the
Job started and the send failed, Bridge still opens it and the card says so in
Fleet's words.

**What a press leaves on screen.** Start Job opens the Job and leaves the
card as `Session started` with an `Open Session` link, until the list is read
again, when Fleet no longer lists it as open; in the review it advances like any
answer. Reject change takes the card off at once. A refused Start Job
(`fleet.lesson_armada_not_served` and the like) draws the card's alert in Fleet's
own words and keeps every button, and opens nothing. There is no confirmation: a
mistaken Start Job starts an agent working on the request, and a mistaken
Reject change discards one item. **An item answered already stands as its `state`
says**, on the list as in the review: `Session started` with its link where
`session_proposed` is present, or nothing where discarded, and no buttons. **An
item with no `state`, or kept before `lands_in`, is drawn without the buttons**:
Fleet refuses both acts on one with no place.

**Review is a button on the Retros page, and it shows one item at a
time.** Pressing it reads `review_lessons`, unless this window already holds a review for the pick, and draws the best item first, with
how far through the person is, the model's one sentence on why it is worth their
time, and the same card and answers as the list. An item that stands for repeats
says so and lists them; rejecting it rejects the repeats only where the person
ticked the box, which starts off. Skip and Back move without answering, and the
arrow keys do the same. Each item keeps its own thread of questions to
`ask_lesson`, kept as the person moves between items. The items the model set
aside are folded away with their reasons, and each can be put back in the queue.
Leaving Review reads the list again, since the review answered items the list
still held.

**A review is kept for as long as the window is open, and leaving Retros does not
end it.** The owner's request, 12 Oct 2026: the page unmounted with the rail, and
returning asked the model again from the first card. Bridge holds the answer, the
place in it, each item's thread and half-typed question, what was put back and the
duplicate boxes, per repository pick, and whether the page was on Review or the
list. Returning to Review resumes at the same card and makes no model call. **The
hold makes the call itself**, so a person who leaves while Fleet is reading finds
the answer on return. **What was answered meanwhile is gone on return**: the page
reads the list fresh and drops what is no longer open, and does not rank again.
Another repository pick starts its own review. The review says how old it is,
`Reviewed 12 minutes ago`, beside `Review again`, the one way to ask the model
again; it throws the held place and threads away and has no confirmation. Nothing
is written to disk, so a restart starts a review again, which is also why Review
mode is held and not remembered like the tab: stored, it would start a model call
as Retros opened. `packages/screens/src/lessons/LessonsReview.tsx`.

### A Job's retro, in its thread

The owner's request, 12 Oct 2026: a person who pressed Retro should see it being
written in the Job's thread. **The `retro` row is one card that stays**
(`SessionRetroRow`, drawn from the latest frame only, since Fleet replaces the row
by id and frames can cross). Running, it names the stage in a line (`Asking the
agent what got in the way`, `Reading what Fleet kept of the Job`, `Writing the
retro`) beside the time since `stage_since`; during `writing` the items settle in
as cards in LessonCard's route (whose way, an arrow, where it lands), headline,
what happened and `What would change`, a field at a time, the last marked `Still
being written` with room held for what is still to come, and a quiet count of
`characters`. Written, it folds to `Retro 3 written · 4 items` with the titles, and
`Open retro` opens that retro by its number on the Retros page; the row's `items`
are empty by then, so Bridge reads the kept retro once for the count and titles.
Failed, Fleet's `why` is the card's alert. **The press moves nobody and does not
wait on the answer**: the mark holds while the press is out and while the row
runs, a second press is refused in Fleet's words and the card goes on, and the
Retros page opens only from the link. Two answers have no card to watch and still
open the retro as the press did: a terminal Job's, whose thread is a
transcript, and a press answered with the last retro, which adds no row.

## Where it is served

| Operation | Route | Answers |
| --- | --- | --- |
| `list_lessons` | `GET /lessons?manifest_id=&lands_in=&state=&most=` | Items across Jobs, newest retro first; `lands_in` absent is all three and `state` absent is `open` |
| `get_session_retro` | `GET /sessions/:session_id/retro?n=` | A Job's retro, the one numbered `n` (the `r<n>` of an item's id) or the newest: the record as it stands, the retro's state and items. The wire's `job_id` holds the Job's id and `session` is present |
| `write_session_retro` | `POST /sessions/:session_id/retro` | The same, once the retro is written |
| `disagree_lesson` | `POST /lessons/:lesson_id/disagree` | The `Lesson`, `discarded` |
| `start_lesson_session` | `POST /lessons/:lesson_id/session` | `{ lesson, session_id, brief }`: the item `agreed` with `session_proposed` set, the Job started, and the `brief` Fleet sent as its first message. No body. Helm only |
| `review_lessons` | `POST /lessons/review` | `LessonReview`: the open items grouped, ordered and set aside, from one model call. Body `{ manifest_id? }` |
| `ask_lesson` | `POST /lessons/:lesson_id/ask` | `{ answer }`, from one model call over the item and the record rows it cites. Body `{ question, history }` |

The wire shapes are `crates/ipc/src/retro.rs`, and
`docs/practices/protocol.md` *Protocol 23.12* has the change, *Protocol 23.15*
`lands_in`, *Protocol 23.26* the parts and `state`, and *Protocol 23.74* the
review. Both acts are Helm only: a person presses them in Bridge, and Helm may
when a person asks. So are the two calls under *Reviewing*.

## Reviewing

**The Retros page gets long, so a person can ask for the open items to be read
together.** The owner's decision, 10 Oct 2026, after the page listed every open
item with its repeats and its stale ones. `review_lessons` reads the open items
as `list_lessons` does, with the same narrowing by repository, and asks the
retro model once, over the newest 50 (200 open items took longer than the 120 s
call budget); the rest are read on the `Review again` that follows once those are
answered. Fleet stores nothing: Bridge holds the answer for as long as
the window is open (*Where Bridge draws it*), and asks again only on `Review
again`.

| Part | Is |
| --- | --- |
| `entries` | Best first, with no rank number. Each is the best item of a group (`lesson_id`), the open items that repeat it (`merged_ids`) and one plain sentence on why it is worth the owner's time (`reason`) |
| `set_aside` | Items the model would leave out of the queue, each with `why` |

**The model sees each item's parts, its age in days and the record rows it
cites by name**, and may set an item aside only for a reason those support. A
cause that no longer applies is such a reason; a hunch is not, and an item
with no such reason stays in the queue.

**Nothing an answer says can lose an open item.** An id the model invented is
dropped. An id that leads an entry is kept once, so it is not also merged or
set aside. A set aside with no `why` does not count. **An open item the answer
names nowhere is appended to the end of `entries` as `not ranked`**, so a model
that forgets one leaves it in the queue where a person sees it. With no open
item there is no call and the answer is empty.

**A person can also put a question to one item.** `ask_lesson` hands the model
the item, the rows of its Job's record that the item cites, as the
record reads now, the turns so far and the question. The model answers from the
record only and says plainly where the record does not show it. A question is at
most 2000 characters and an empty one is refused; the history is at most 20
turns of 4000 characters. Fleet holds no thread, so Bridge sends the turns back
each time. An id that names no item is refused as the other acts refuse it.

Both calls go out on the retro's model, client and budget, and are not metered,
as a retro's call is. The two prompts are `prompts.retroReview` and
`prompts.retroAsk`, written to the same rules for plain words.

## What the Logbook takes

**A fact about a repository, the owner or this Mac stops going to the Retros page.** [Logbook](logbook.md) takes it, and a retro keeps only what Armada itself got wrong.
The open items that are really repository facts are moved into the Logbook once, and closed.

## Open questions

- **[retro-rewrite]** Whether a person can ask for a failed retro
  to be written again. Nothing offers it.
