# The evolution graph

**Part of:** [Job](../session.md).

**The Artifacts layer's Graph draws how a Job grew, each thing hanging off what produced it.**
It reads the same rows as the layer's List ([A Job open](a-job-open.md)) and the Job's attachment history, and lays them out left to right in the order things first appeared.
The List and the Graph take the same search field.

## What it shows

| Node | Is | Hangs off |
|---|---|---|
| Message | A message of the person's that something followed | Nothing |
| Plan | The recorded plan | The message before the row that recorded it |
| Group | One group of the plan | The plan |
| Artifact | A link, issue, pull request, sketch, page, file, image, doc or child Job | See below |

An artifact is one node however often it changed.
Its versions are a row of marks inside the node, each with a registry glyph and a tooltip naming what changed and when.

## Where each join comes from

| Join | Comes from |
|---|---|
| Artifact to a group | `from_task` of its first version: the plan task in progress when it was written |
| Artifact to a message | `from_message` of its first version: the person's latest message then |
| Artifact to the first message | Neither is known |
| Child Job to a group | The group on its `started_to` row |
| Issue to a pull request | The pull request's `closes`, or the issue's `pull_requests` |
| A later version to what it followed | A second, thin line, where it followed something else than the first version did |

Where both are known the task wins.
`from_task` and `from_message` are Fleet's focus, worked out from what it saw and never from what the agent says ([What is kept](what-is-kept.md), *Every version*).

**`from_message` is the id of a message row of the thread.** Bridge draws the thread from the rows Fleet sends, so the id needs no translating.
A `from_message` that names no message of the person's in the thread falls back to the first message.

## When it reads

Bridge reads `get_attachment_history` when the Graph opens, and again each time the Job's ledger changes while it is open.
It does not poll, and the List never reads it.

When the read fails the Graph draws one node for each row the ledger holds, with no marks and no line from a later version.
It says nothing about the failure.

## Keys

| Key | Does |
|---|---|
| `V` | Switches List and Graph, anywhere in the layer but the search field |
| `Tab` | Enters the Graph at its first node |
| Arrows | Move to the nearest node that way |
| `Enter` | Opens the node's latest version |
| `.` | Enters the node's marks at the first, then steps on |
| `,` | Enters the marks at the last, then steps back |
| Left, Right | Step along the marks |
| `Enter` on a mark | Opens that version; an old sketch opens as it was drawn |
| `Esc` on a mark | Returns to its node |

The search field dims the nodes it does not match and keeps them in place.

## What it does not show

| Left out | Why |
|---|---|
| Commands, subagents, background tasks | They are not artifacts |
| Slots and branches | They are where the Job works, not what it made |
| A message nothing followed | It has nothing to hang |
| A version of a row the ledger has let go | The ledger draws no row for it |
