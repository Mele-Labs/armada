# A Job that started Jobs

**Part of:** [Job](../session.md).

A Job that started other Jobs reads where each stands and acts on them through `armada job`.
The case was 4 Nov 2026: a parent started eleven Jobs, could not see their state without reading transcripts, and stopping eight took eight steps by hand.

| | |
|---|---|
| `list_started_jobs` | Every Job below the caller, the ones it started and the ones those started in turn: each child is followed by its own descendants, oldest first at each level. Each is its summary line (id, title, state, branch, pull requests) with `started_by`, the Job that started it, and: whether it is held on its start card or paused; the workflow step it is on and what that step waits on; what it waits on the person for, the items Needs you shows for it, Fleet's own `unsent:<id>` among them; the Job it waits on before it starts, if any; and, once it ended, why and what its agent said with `all_done` |
| `command_started_jobs` | A list of entries `{session_id, act, ...}`, each done as a call of its own would be, in order. A refused entry leaves the others done, and each is answered in the order given: the child as it now stands, or the refusal |
| The caller | Placed by its connection, as `start_another_session` places it. **No body names it**, so a Job cannot read or command another's children. A call that places no hosted Job is refused 422 `fleet.started_unplaced` |
| Whose | A child is a Job the caller's own `started_to` link names. **The read walks the whole tree below the caller; the acts do not. Every act refuses any other Job with 422 `fleet.started_not_yours`**, a grandchild, a sibling and the caller's own parent included. A fork is not a child |
| Not gated | By no Operator switch. Any Job may act on its own children |

| Act | What it does |
|---|---|
| `withdraw` | Ends a child that has not begun, held on its start card or waiting on another Job: no agent ever ran. Its `start:<id>` item leaves the caller's Needs you. Refused 409 `fleet.started_begun` for one that has begun |
| `end` | Needs `why`, which the child is told when it has begun (the `started_job_ended` notice), then closes it as Bridge's End closes it: the slot goes back and the work stays on its branch. A beads issue it claimed through Fleet is handed back, with a note naming the Job that ended it. Allowed for a child that has pushed, since closing loses no work |
| `pause`, `resume` | The same Fleet acts the person's Pause and Resume run. Those two operations stay Bridge's |
| `change` | A new `condition` or `base` for a child that waits on another Job, whichever Job that is, one that is ending included. The Job waited on is told which Job changed it |
| `drop` | The child no longer waits. Its card stays held for the person, with its brief as the first message and no handoff from a Job it no longer waits on. The Job waited on is told |

> **Rule.** A Job acts only on the Jobs it started, and `held`, `workflow`, `model` and `effort` of a child stay the person's.
> Why: the parent knows what it asked for and when that stops being wanted, and the person's picks are what the child will do once he presses Start.

**Withdraw leaves nothing behind.** The child ends, its slot is not leased because none was, and its `start:<id>` item leaves the caller's list on the same read, as any end takes it.
**End hands an issue back only while its Job holds it.** The release is `bd unclaim --if-assignee`, so an issue another machine has since taken is never taken from it, and it is synced at once.
The link from the issue to the Job stays, so a merge of the child's pull request can still close the issue it was for.
A backlog that would not take the release is said in the caller's thread, and the end stands.

Nothing is drawn in Bridge.
The thread of a child that was ended this way reads *ended by its starter* where Fleet told it why.
