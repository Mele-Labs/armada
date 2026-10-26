# A Job open

**Part of:** [Job](../session.md).

Since 16 Oct 2026. The owner, 16 Oct 2026, short of room: a right column, no Ledger, more Flight control keys, and an Artifacts view for "everything that was created or used during a session".
Approved on `?walk=compactSessionView`; the calls were the thread's last card (`?walk=compactCallsInThread`) until 17 Oct 2026, when they became the Needs you panel on the composer.
`docs/concepts/cockpit.md` has the layout.

| On screen | From |
|---|---|
| The thread, and its last card | The plan awaiting approval. A question, a judge's concern and the rest are rows in the Needs you panel on the composer |
| The Field | The whole repository from `list_repository_areas`, with the plots its plan's scope and drones name outlined. Its drones from `work.field`, only the Job's own and the subagents at work: one whose `subagent` row is `spent` has left it. Every other live Job in the repository faded. The glow and the slider from its `session.edited` and `session.read` timeline entries. The pad's lights from the thread's `check` rows and a blamed file and line from a failing row's `blame`, never from its `target`; a walk from a `window` row with no `ended_at`; thinking from twelve seconds with no `tool` row or tool entry while the turn works; a call from `session.called`; the strip's subagents from the ledger's `subagent` rows; the lens from `get_session_file_change` for each file a drone of the Job is writing, through `readSessionFileChange`, a refused path drawing none. An arc to another Job's drone, and the strip's `Messaging` or `Messaged by`, from a fresh `session.messaged` entry naming the Job as its subject or its `to`. `docs/concepts/cockpit.md`, *A Job open* |
| The right column | The Job as a story, top to bottom: Workflow, Blueprint, Field and Delivery. The workflow and its steps are a line in the first band, the plan is the Blueprint, the drones are the Field, and what the workflow ends in is Delivery, by `WorkflowSummary.lands`. The stage opens one and folds the rest. *In Bridge*, and `docs/practices/protocol/what-a-workflow-lands-in.md` |
| Flight control | The Job's own tiles, one row; Requests opens the side panel. The Job's head holds Checks, Subagents and Bays, which open a layer over the thread, and Artifacts, which opens one over the right column |
| Requests rows | A `requested` row for the person's send and a `saved` row for the agent's: each opens Requests for the Job. `docs/concepts/requests.md` |

| Layer | Key | Reads |
|---|---|---|
| Artifacts | `8` | The Job's rows as they stand, opened by this key, by the Artifacts icon in the Job's head or by `⇧⌘A`: `pr`, `issue`, `link` (an address the person or the agent wrote in the thread, kept by host and path; loopback addresses and the repository's own issue and pull request links are not), `artifact` (pages, files, docs, images), `sketch`, `subagent`, `background`, `studio` and `started_to` (the Jobs it handed a plan group to, each with the group and its state), the files its drones touched (`touches`), and from the thread the commands it ran and the commits it made. **The wire holds no commit record, so a commit is the thread's `git commit -m` line**, not Fleet's observation,. A field at the top narrows every group as it is typed in |
| Subagents | `9` | Every `subagent` row, the running ones first. A press opens its own thread |
| Checks | `0` | The `check` rows of the thread, newest first. A press opens the run on the Checks page |
| Bays | none | Every bay the Job holds, or held last before it stranded, each with its lease and how far it is behind its base, and the Job's write clashes: `slot-trouble.ts`. **Lit only when a bay is stranded or 20 or more commits behind.** A write clash puts the swords glyph at the key's corner and does not light it. A stranded bay is also a call in the thread's last card. A bay behind and a write clash are not: Fleet tells the agent, so they leave the owner nothing to do. Off, with its reason, where the Job holds no bay |

A layer covers the thread and the composer, which stay mounted under it: Back (the key again, the button, or Esc) returns to the thread at the same offset with focus where it was.
**Artifacts covers the right column instead**, so the thread and the composer stay in view, and Back returns to the story as the stage had it.
The arrows walk its rows, and focus goes in when it opens, to the search field in Artifacts and to the first row elsewhere.
Artifacts has a List and a Graph, switched by the icon toggles in its head or by `V` anywhere in the layer but the field.
The Graph draws how the Job grew, each thing hanging off what produced it, and is [The evolution graph](the-evolution-graph.md).
Esc is the layer's while one is open, and a sheet over it keeps its own.
The layer is dropped when another Job is opened.
