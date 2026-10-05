# Retro

**What it is:** What got in the way while one Job ran, whose way it got in —
the Drone's, the owner's or Fleet's — and where each fix lands, written once
the Job ends and read on the Lessons page.

---

**Kind:** Entity. Owned by [Job](job.md), at most one per Job.

## What it is

A retro is a list of items. Each item names whose way it got in and where its
fix lands, says in one sentence what got in the way, and cites the rows of the
Job's record that show it.

| Who | Means |
| --- | --- |
| `drone` | The agent doing the work was slowed or stopped |
| `owner` | The person who owns the Job had to step in, wait or redo something |
| `fleet` | Armada itself cost the Job something it should not have |

## Where the fix lands

**Every item names where its fix lands, apart from whose way it got in.** The
owner's decision, 3 Oct 2026. Who it cost and where it is fixed are two
questions: a command a Drone was refused cost the Drone, and is fixed in Kit.

| `lands_in` | Means | For example |
| --- | --- | --- |
| `armada` | Armada itself: Fleet or Bridge | The gate measured a Job's diff from local main; ruled a red without confirming it; showed a Drone running after it ended |
| `kit` | The tool set: Skills, MCP servers, sub-agents, agent files, plugins, commands, the allowlist, the models list ([Kit](kit.md)) | A command a Drone was refused, or had to ask a person to allow |
| `manifest` | The repository the Job worked on: its `armada.yml` (Checks, Commands, places, when), its tests and its code ([Manifest](manifest.md)) | Browser tests on a fixed 15 s timeout; `test` running every Rust test on a docs-only edit |

**An item names exactly one.** Where a fix spans two places, the retro writes
two items. The Lessons page narrows by it, and **an item written before
23.15 names none**: it reads with the field absent and is listed under All
alone, never given a place after the fact.

## Items

**An item has parts, so it can be read at a glance and acted on.** The owner's
decision, 4 Oct 2026, after Job 3's retro was rejected as a wall of prose that
named no root cause and blamed the wrong party.

| Part | Is |
| --- | --- |
| `title` | A headline of about eight words |
| `what` | One or two short sentences: what happened, and to whom |
| `fix` | One sentence naming what to change |
| `who`, `lands_in`, `evidence` | As above |
| `id` | Names the item for an act. The Job's id, a hyphen and the item's place in its retro |

**`statement` stays, and repeats `what`** on an item written since, so a reader
that predates the parts still has a sentence. **An item written before the parts
has `statement` alone**: they read absent, and nothing writes them after the
fact. Job 3's own retro stays as it was written, as the owner's evidence.

**An item is kept whole or not at all.** One missing a part, one with a dash in
any of the three, one citing nothing the record holds and one naming no place
are each dropped alone, and the items beside them are kept.

## Agree and disagree

**A person answers each item, and agreeing is the one place a retro acts.**
Every item starts `open`.

| State | Means |
| --- | --- |
| `open` | Nobody has answered it. What the Lessons page lists by default |
| `agreed` | A Job was proposed for it at the approval gate, and `job_proposed` names it |
| `accepted` | A Kit item the person agreed with. Nothing to dispatch, so it is kept as it is: the person's saved Kit items |
| `discarded` | Disagreed with. The row stays |

**Agreeing is by where the fix lands.** A `manifest` fix proposes a Job on the
repository the item's own Job worked on, and an `armada` fix on Armada's own
repository, the Manifest named `armada`. Where Fleet does not serve that one it
refuses with `fleet.lesson_armada_not_served` and the item stays open. A `kit`
fix proposes nothing. The request is the item: its title, what happened, the
fix and a line naming the source Job's handle. It goes through the call
`propose_from_request` makes, so a person approves the Job where they approve
any other. **An item that is not open answers with the state it stands in**, so
agreeing twice makes one Job. **Disagreeing never takes back a Job** already
proposed.

An item kept before `lands_in` names no place, so agreeing with it is refused:
where its Job would go is not Fleet's to guess.

## The record it is read from

**The record is assembled from what Fleet already keeps, on every read.**
`crates/fleet/src/retro/record.rs` reads the Job the way `scripts/job` does:
its detail, history and evidence as the routes serve them, its Drones'
transcripts and its own log.

| Row | Read from |
| --- | --- |
| `refusals` | A transcript's refused call, joined to the call it answered |
| `failed_checks` | The gate's runs, checks only a transcript kept, the Drone's own `run_checks`, and each red the gate ran again alone — Fleet's friction, not the Drone's ([Manifest](manifest.md), Confirming a red). **A gate failure that names a file carries `paths`**: each file, and whether a tool call of the step's Drones names it. `false` says the failure did not come from the Drone's own calls |
| `not_met` | A Judge criterion `not_met`, with expected and produced |
| `not_done` | What each step's submission said it had not done |
| `said_after` | What a Drone said in prose right after each submission, and last |
| `restarts` | A held Job re-queued, a stopped step run again, a redispatch |
| `asked` | A question put to a person, its answer, and how long it waited |
| `waited` | Each stretch at an `awaiting_*` status |
| `acts` | Every move a person or an agent made, and the door it came through |
| `notes` | What a Drone said got in its way, on submitting |

Every row carries a `cite`, `refusal:1` and the like, unique within the record.

## Three sources

### The record

**Fleet assembles it mechanically.** It reads nothing a model wrote and stores
nothing new except the two facts below, which nothing else kept.

### A Drone's note

**`submit_evidence` takes `in_the_way`, one optional line.** A Drone is asked
in the tool's own description what got in its way on the part it is handing
in. It is never evidence: the gate and the Judge never read it, and it is
kept for the retro alone.

### The retro call

**One model call per ended Job, on the Judge's road and on a model of its
own.** It takes the Judge's client and budget, reads no repository, and is
handed only the record and the owner's linked annotations. Its instructions
carry the table under *Where the fix lands*, in plain words. An item citing a
row the record does not hold loses that citation; one citing nothing the record
holds is dropped, and so is one naming no place its fix lands or a place that
is not one of the three. **The place is never defaulted.** An item that will
not read is dropped alone, and the items beside it are kept.

**The model is Sonnet**, the owner's decision on 4 Oct 2026 after the cheap
tier wrote the retro of Job 3. It is `settings.retro-model`, overridden by
`ARMADA_RETRO_MODEL`, and the Judge's own dial does not move with it. The call
is one per ended Job, made after the Job ended, and it is not metered: an
unwatched call reports no cost, so it counts against no cost cap, as the
Judge's calls never have.

**The instructions refuse to guess.** They say, in plain words:

- Name a cause only where the record shows one. "Unclear" is an allowed
  answer, and a symptom may be written as a symptom.
- Blame the Drone only for what the record shows the Drone did, its own tool
  call or its own claim. A failed check on a file the Drone's calls never name
  is not the Drone's, and the record says so with `paths`.
- A fault of Armada's goes to `who: fleet` and `lands_in: armada`, and so does
  a check that failed with others running beside it and passed alone.
- One item per cause. Fewer, truer items beat many.
- Write the three texts in plain words: short sentences with concrete facts,
  plain verbs, no dashes, no "not X but Y", no lists of three for rhythm, no
  stock words, no vague "associated with", no closing line, no first sentence
  that restates the headline.

## When it is written

**A Job is owed a retro once it reaches a terminal status.** Fleet writes owed
retros one at a time, oldest first, on a task of its own after each turn, so
nothing a Job does waits on one.

| State | Means |
| --- | --- |
| `pending` | The Job has not ended, or its retro is not written yet |
| `written` | The call answered; `items` may be empty |
| `failed` | The call failed or its answer would not read; never tried again |
| `skipped` | No Drone ran on it, or it ended before Fleet wrote retros |

## Who acted

**A person's act that reaches Fleet through any door but Bridge is signed
`helm`.** An agent made the request, and Fleet cannot see the person behind
it. Every move a request makes also keeps the door it came through, as `via`
on `get_job_events`.

| `via` | The request |
| --- | --- |
| `bridge` | Carried `x-armada-caller: bridge`, which Bridge sends on every request |
| `helm` | Came through the agent door from a Helm session Fleet placed |
| `door` | Came through the agent door from any other session, such as `armada mcp` |
| `http` | Named no caller: a script, `curl`, an agent's shell, the `armada` CLI |

**The header is attribution, not authentication.** Any process can send it,
and what it buys is that an agent's `curl` stops reading as a person's press.

## The owner's annotations

**An annotation attaches to a Job's retro by the Job id it carries.** A note
left in Bridge's annotation layer with a Job's detail open names that Job as
`openJobId`; a note that names none is linked to no Job. Fleet never guesses
a Job from the time a note was left.

**Bridge writes the open Job's ULID as `openJobId` when a note is saved.** Job
detail stamps its Job's id on its own root, and the layer reads the page at the
save, so a note begun on one Job and saved on another names the second. Job
detail alone counts as open: a retro read on the Lessons page is not that Job's
detail, and a note left there names no Job. The key is left out, never null,
where none is open. `packages/screens/src/open-job.ts`.

## Where Bridge draws it

| Where | What |
| --- | --- |
| **Lessons**, a rail surface under Work | `list_lessons`, narrowed to the rail's pick and every repository on All, under tabs `All · Armada · Kit · Manifest` by where each fix lands. All is the default, an item with no `lands_in` is under All alone, and the tab is remembered for the viewer. A row opens its Job's retro |
| **Retro**, in the head of a Job's Record | The same sheet, on `get_job_retro` |

Both read when they open and again when the window regains focus, because
nothing on `/events` says a retro was written. Whose way an item got in, and
where its fix lands, are marks named by their tooltips. **What Fleet serves for
acting** is *Agree and disagree*: the Lessons page lists the `open` items, and a
person's saved Kit items are `?state=accepted`. Drawing the two acts is
Bridge's.

## Where it is served

| Operation | Route | Answers |
| --- | --- | --- |
| `get_job_retro` | `GET /jobs/:job_id/retro` | The record as it stands, the retro's state and items, linked annotations |
| `list_lessons` | `GET /lessons?manifest_id=&lands_in=&state=&most=` | Items across Jobs, newest retro first; `lands_in` absent is all three and `state` absent is `open` |
| `agree_lesson` | `POST /lessons/:lesson_id/agree` | The `Lesson` as it now stands, with `job_proposed` where a Job was proposed |
| `disagree_lesson` | `POST /lessons/:lesson_id/disagree` | The `Lesson`, `discarded` |

The wire shapes are `crates/ipc/src/retro.rs`, and
`docs/practices/protocol.md` *Protocol 23.12* has the change, *Protocol 23.15*
`lands_in`, *Protocol 23.23* the parts, the two acts and `state`. Both acts are
Helm only, as `propose_from_request` is: a person presses them in Bridge, and
Helm may when a person asks.

## Open questions

- **[retro-rewrite]** Whether a person can ask for a failed or skipped retro
  to be written again. Nothing offers it.
