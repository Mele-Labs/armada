# Retro

**What it is:** What got in the way while one Job ran, and whose way it got in
— the Drone's, the owner's or Fleet's — written once the Job ends and read on
the Lessons page.

---

**Kind:** Entity. Owned by [Job](job.md), at most one per Job.

## What it is

A retro is a list of items. Each item names whose way it got in, says in one
sentence what got in the way, and cites the rows of the Job's record that
show it.

| Who | Means |
| --- | --- |
| `drone` | The agent doing the work was slowed or stopped |
| `owner` | The person who owns the Job had to step in, wait or redo something |
| `fleet` | Armada itself cost the Job something it should not have |

**Nothing acts on a retro.** No item is filed as an issue, proposed as a
change or put into a brief. The owner reads the Lessons page and decides
what each item is worth.

## The record it is read from

**The record is assembled from what Fleet already keeps, on every read.**
`crates/fleet/src/retro/record.rs` reads the Job the way `scripts/job` does:
its detail, history and evidence as the routes serve them, its Drones'
transcripts and its own log.

| Row | Read from |
| --- | --- |
| `refusals` | A transcript's refused call, joined to the call it answered |
| `failed_checks` | The gate's runs, checks only a transcript kept, and the Drone's own `run_checks` |
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

**One model call per ended Job, on the Judge's road.** It takes the Judge's
client, budget and cheap model, reads no repository, and is handed only the
record and the owner's linked annotations. An item citing a row the record
does not hold loses that citation; one citing nothing the record holds is
dropped.

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

**Bridge does not write `openJobId` yet.** Until it does, no annotation is
linked.

## Where it is served

| Operation | Route | Answers |
| --- | --- | --- |
| `get_job_retro` | `GET /jobs/:job_id/retro` | The record as it stands, the retro's state and items, linked annotations |
| `list_lessons` | `GET /lessons?manifest_id=&most=` | Items across Jobs, newest retro first |

The wire shapes are `crates/ipc/src/retro.rs`, and
`docs/practices/protocol.md` *Protocol 23.11* has the change.

## Open questions

- **[retro-rewrite]** Whether a person can ask for a failed or skipped retro
  to be written again. Nothing offers it.
- **[retro-annotation-link]** What Bridge writes as `openJobId`, and on which
  screens a Job's detail counts as open.
