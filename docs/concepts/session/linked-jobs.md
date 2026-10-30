# Linked Jobs

**Part of:** [Job](../session.md).

A Job splits its work into **linked Jobs it follows**, one for each group of its plan.
The owner approved the design on 16 Oct 2026 (walk `linkedSessions`), with the groups on the Plan and a Linked section in the Ledger both drawn.
The Plan is the Blueprint now and the Ledger is gone from a Job's view; the same Jobs are a group in the Artifacts layer (*A Job open*).

| | |
|---|---|
| Handing a group | `start_another_session` with `group`, the agent's, held for the owner like any start. `hand_to_session` sends the same brief for a press of the owner's, but Bridge draws no **Hand to a Job**: he never pressed it, and on a group already in flight it would start a second Job on the same tasks. The owner, 28 Oct 2026. The new Job is in the same repository, titled with the group's title |
| The brief | The group's tasks word for word, its paths, then the parent's handoff: its title and id, slot, branches, open pull requests and the last six messages of its thread. The agent's own `message` follows when it gave one |
| The link | `started_by` on the child and `started_to` on the parent, as for any started Job, with `detail.group` on both. A group is handed once (409 `fleet.group_handed`), and a group the plan lacks is refused (422 `fleet.group_unknown`) |
| The parent keeps track | When a child **starts** (leaves its start card), **ends** or has a **pull request merged**, Fleet sends the parent's agent a message, which wakes it when idle, and writes a `linked` row (`event` `started`, `done` or `merged`) in the parent's thread. The message is news, so it leaves the agent's `all_done` standing. Each is told once |
| A child that needs the owner | Is not told to the parent. The call is on the child's own row, and a notice in the parent's thread made the parent's replies pile up there as messages to him, contradicting the child's. The owner, 19 Oct 2026. Rows of `event` `needs` that older threads hold still read and draw |
| Every Job started from another | Is followed in that way and stands under its parent in Work, whether or not it was handed a group. The Blueprint and the Artifacts layer draw only those with a group |

> **Rule.** A Job follows every Job its agent starts, and the owner hears of a child through the parent's thread where it happened.
> Why: the parent is the one holding the plan, and a child that waited on him with nobody saying so was how work stalled.

Bridge draws it from the wire, with no state of its own:

| Where | What |
|---|---|
| The Blueprint | Under a group of the plan handed to a Job, that Job's row of two lines: the `link` glyph and its title, then in the mono muted line its state mark and its pull request chip with the checks mark. A group not handed opens to its tasks alone |
| A handed group on the spine | The Job takes the group's row, open or shut: the shared Job card titled with the Job's title, the group's title as its second line, and the Job's pull request as the framed pull request chip under it. The group has no toggle, since its tasks are the Job's work and opening the Job shows them. The card opens the Job, the chip opens the pull request in the side panel, and each is its own keyboard stop. A group not handed keeps its row and its toggle. The owner, 30 Oct 2026: "What is the "Name the migrations" versus the job below? Why do we need both?" |
| The Artifacts layer | The first group, Jobs: each child's title, the group it came from after it, its state mark and its pull request with the checks mark. A press opens the child, and Esc is Back to the parent |
| The parent's thread | A `linked` row where it happened, `power` for done, `git-merge` for merged, the press that opens the child |
| Work | `docs/concepts/cockpit.md` |
| Keys | Enter on a row opens the child; Esc is Back, to the parent's thread and open groups as they were |

A notice for a parent that ended is dropped.
