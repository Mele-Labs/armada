# A held Job

**Part of:** [Job](../session.md).

A new Job opens held before any agent moves, and Bridge draws it as it draws any Job: its title in the header, renamed in place, an empty thread, and the composer under it.
The composer holds the first message the agent will read, for him to edit.
Workflow and Delivery sit in a tinted strip just above the composer, and Send reads Start.
He changes what he wants and presses Start (`↵`), and the agent begins.

| Part | Filled from |
| --- | --- |
| Title | The proposer's reading of the request, or the title the starter gave |
| Thread | Empty. Where another Job started it, a *Started by* line above it |
| Composer | The first message, as the agent will read it |
| Workflow | The proposer's pick, else the repository's `feature` workflow, else its first. Never blank. A custom Job, which he chose, stays custom |
| Delivery | The landing his words named, else his usual, else Auto land |
| Model, Effort | The proposer's reading, else the machine's own |

**Delivery is in the strip only where the Job's workflow has a step that asks how the work lands.**
`hosted.start.asks_delivery` says so.
A custom Job, and a workflow with no such step, have nothing to settle there.

## Which Jobs are held

| Start | Held |
| --- | --- |
| Dispatch from Bridge | Yes |
| An agent's start, `start_another_session`, and its fork, `fork_this_session` | Yes |
| A start with `after`, once its wait is met | Yes |
| A note's Start job on the annotation layer | Yes |
| A start with `root`, and Helm's | No |

A Job that is not held starts as before and is asked the delivery question at its plan step (*The delivery question*).
A Job that waits on another is not held while it waits, since it has no first message yet (*A Job that waits on another*).

## What the first message is

**The first message is the whole turn the agent will read.**
For a Job started from Dispatch it is what he typed.
For a Job another Job's agent started it is the agent's `message`, with the group's brief and the parent's handoff ahead of it where the agent handed a group (*Linked Jobs*).
For a Job whose wait is met it is the handoff of the Job waited on, then the brief.
For a note it is his words, then the `Pointed at:`, `Candidates:` and `Page:` lines Fleet adds.

**It stays off the thread until Start.**
Fleet keeps it in `held_first_turns`, one row for the Job, with its pictures, so a restart leaves the box as it was.
The first to arrive is the first turn.
Where nothing is kept yet, another Job's message to the held Job becomes its first message, read to the agent as that Job's.

## The marks

**Workflow and Delivery are never blank**, so each mark says where a value came from.

| Mark | On | Meaning |
| --- | --- | --- |
| Quote | Delivery | His words named the landing |
| Repeat | Delivery | His usual for that kind of work in that repository |
| Light bulb | Workflow, model, effort | The proposer's pick |
| None | Delivery | Fleet's own pick, Auto land |

A mark on the proposer's pick goes when he sets that field.

## The usual landing

**The usual is the most frequent of his last ten picks for that kind of work in that repository, once there are three.**
A tie goes to the most recent pick.
Below three picks there is no usual, and Delivery is Auto land.

**A landing named in the request wins over the usual.**
The phrases are the delivery question's.

A pick is kept each time a landing is settled for a Job with a kind: Start, the answer to the plan-step question, and a first message that names one.
A custom Job has no kind and keeps none.

## What the hold keeps

**Nothing runs until Start.**
The request is still read, so Workflow and the other fields are filled, and the reading sends nothing.
A held Job stays in Needs you as *First message* until Start, and Bridge draws it as it draws any Job that calls him.
Ending the Job settles the item and discards the first message.
While sleep is on, the item is left on the list and not decided for him.

**Until Start, the Job is a draft.** It wears the Draft mark, `message-square-dashed` with the tooltip `Draft`, on its list row, its header and its flight pane, and keeps the needs-you tone.
The mark goes at Start.

| Held | Kept |
| --- | --- |
| The hold | With the Job, so a Fleet restart leaves it held |
| The first message | In `held_first_turns`, off the thread |
| A later message | In `held_later_turns`, off the thread, behind the first |

Messages go in the order they came.
Start writes each one's row to the thread in that order, the first first, as it sends them, so the thread reads as the agent does.
A restart loses none of them, since both are in the store.

## Start

**Start sends the box as he left it.**
Where it differs from the kept first message his words replace it, what the agent reads and what the thread keeps, and the pictures stay with it.
Where it does not, the kept turn goes as held.
The turn is written to the thread then, and the agent's first turn carries the workflow line, the step's instructions and the message (*The rail*).

**Start keeps the landing as the answer to the delivery question**, so nothing asks later.
Auto land turns the Job's Auto land flag on, as the delivery question's answer does.
The wire is `docs/practices/protocol/a-job-starts-on-its-start-card.md`.
