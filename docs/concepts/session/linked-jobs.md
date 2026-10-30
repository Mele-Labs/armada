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
| The parent keeps track | When a child **starts** (leaves its start card), **ends** or has a **pull request merged**, Fleet sends the parent's agent a message, which wakes it when idle, and writes a `linked` row (`event` `started`, `done` or `merged`) in the parent's thread. The message is news, so it leaves the agent's `all_done` standing. Each is told once. Merged is what the forge says: a `pr` row the mod settled spent off a `gh pr` command while the forge still reads it open is not a merge |
| A child that needs the owner | Is not told to the parent. The call is on the child's own row, and a notice in the parent's thread made the parent's replies pile up there as messages to him, contradicting the child's. The owner, 19 Oct 2026. Rows of `event` `needs` that older threads hold still read and draw |
| Every Job started from another | Is followed in that way and stands under its parent in Work, whether or not it was handed a group. The spine and the Artifacts layer draw every one: a Job handed a group on the group's row, any other in Now while it runs and in Done once it is done or ended. The owner, 30 Oct 2026, of two Jobs a Job had just started that showed nowhere on it |

> **Rule.** A Job follows every Job its agent starts, and the owner hears of a child through the parent's thread where it happened.
> Why: the parent is the one holding the plan, and a child that waited on him with nobody saying so was how work stalled.

Bridge draws it from the wire, with no state of its own:

| Where | What |
|---|---|
| The Blueprint | A group of the plan handed to a Job is that Job's card, as on the spine: the shared Job card titled with the Job's title, the group's title as its second line, and the Job's pull request as the framed chip under it. It has no toggle. A group not handed opens to its tasks alone, unless it goes to a Job of its own (*A group not handed yet*) |
| A handed group under Done | The same card and chip, with no tick track and no task list, since its tasks are the Job's work. The group's own issue and its merged pull requests stay beneath. The Job card carries a bot glyph at its left on every surface, with the tooltip Job |
| A handed group on the spine | Stands where its Job does until the Job finishes: being worked once the Job is under way, still to come while it waits to start, whatever the parent ticked when it handed it. A finished Job leaves the group to its tasks. The Job takes the group's row, open or shut: the shared Job card titled with the Job's title, the group's title as its second line, and the Job's pull request as the framed pull request chip under it. The group has no toggle, since its tasks are the Job's work and opening the Job shows them. The card opens the Job, the chip opens the pull request in the side panel, and each is its own keyboard stop. A group not handed keeps its row and its toggle, unless it goes to a Job of its own (next row). The owner, 30 Oct 2026: "What is the "Name the migrations" versus the job below? Why do we need both?" |
| A group not handed yet | A group that goes to a Job of its own, `ownJob` on the plan's group, has the shared Job card as a placeholder, as a pull request not opened yet is a dashed chip: the same card with a dashed outline and no fill, the bot glyph muted at the left with the tooltip Not started yet, and the title the plan gives the group. It has no state glyph, no second line and no age, holds nothing, and is read and never pressed, so it is no stop for Tab. **The card stands in the group's place, as a started Job's card does.** On the spine's rows of an approved plan it is the card alone, with no tasks and no issue, where the Job's card stands once the group is handed; its issue stays in Open. In a plan being drafted it stands where the group's title row was, with the group's issue, its editable tasks and the chevron under it as on any group, the chevron at the right of the issue or alone on a row; the title is edited in the editor the chevron opens, with the subagent type and the paths. On the thread's plan card and in the plan review sheet the group is a run of its own, headed by the card in the place of the pull request chip, and its `landsIn` is ignored since its Job lands its own pull request. The thread's card grows to hold every run, where a plan card is otherwise cut at a fixed height. The group's line under the card shows its task marks and subagent type without the title, and the review sheet keeps the group's editor under it. When the group is handed the Job's card replaces the placeholder in each place and opens the Job. Fleet does not carry `ownJob` yet, so only the mock sets it |
| A Job started outside the plan | The same Job card with no group line, and its pull request as the framed chip under it: in Now while it runs, in Done once it is done or ended, under Jobs when Done is read By kind |
| The Artifacts layer | The first group, Jobs: each child's title, the group it came from after it where it had one, its state mark and its pull request with the checks mark. A press opens the child, and Esc is Back to the parent |
| The parent's thread | A `linked` row where it happened, `power` for done, `git-merge` for merged, the press that opens the child |
| Work | `docs/concepts/cockpit.md` |
| Keys | Enter on a row opens the child; Esc is Back, to the parent's thread and open groups as they were |

A notice for a parent that ended is dropped.
