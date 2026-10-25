# The start card

**Part of:** [Job](../session.md).

A Job dispatched from Bridge opens held before any agent moves, and Bridge draws it as it draws any Job: its title in the header, renamed in place, and the composer under the thread.
Kind and Lands sit in the composer's bar beside model and effort, each filled with Armada's pick, and Send reads Start.
He changes what he wants and presses Start (`↵`), and the agent begins.

| Row | Filled from |
| --- | --- |
| Title | The proposer's reading of the request |
| Kind | The workflow the proposer picked, or none for a custom Job |
| Lands | The landing the request names, else his usual, else empty |
| Model, Effort | The proposer's reading, else the machine's own |

**Lands is on the card only where the Job's workflow has a step that asks how the work lands.**
`hosted.start.asks_delivery` says so.
A custom Job, and a workflow with no such step, have nothing to settle there.

## Which Jobs get a card

| Start | Card |
| --- | --- |
| Dispatch from Bridge | Yes |
| An agent's start, `start_another_session` | No |
| A fork | No |
| A start with `root`, and Helm's | No |
| A start with `after` | No |

A Job with no card starts as before and is asked the delivery question at its plan step (*The delivery question*).

## The usual landing

**The usual is the most frequent of his last ten picks for that kind of work in that repository, once there are three.**
A tie goes to the most recent pick.
Below three picks there is no usual, and the row is empty.

**A landing named in the request wins over the usual.**
The phrases are the delivery question's.
The card marks which of the two it offers.

A pick is kept each time a landing is settled for a Job with a kind: Start on the card, the answer to the plan-step question, and a first message that names one.
A custom Job has no kind and keeps none.

## What the hold keeps

**Nothing runs until Start.**
The request is still read, so the card's fields are filled, and the answer sends nothing.
A Job on its card needs him, and Bridge draws it as it draws any Job that calls him.

| Held | Kept |
| --- | --- |
| The hold | With the Job, so a Fleet restart leaves it held |
| His messages | Written to the thread at once, sent after Start |
| Another Job's messages | The same, read to the agent as that Job's |

Messages go in the order they came.
After a restart Fleet reads them back off the thread at the first send or Start, before it writes anything else to the Job.

## Start

**Start keeps the landing as the answer to the delivery question**, so nothing asks later.
Auto land turns the Job's Auto land flag on, as the delivery question's answer does.
The wire is `docs/practices/protocol/a-job-starts-on-its-start-card.md`.
