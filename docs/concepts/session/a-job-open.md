# A Job open

**Part of:** [Job](../session.md).

Since 16 Oct 2026. The owner, 16 Oct 2026, short of room: Progress, no Ledger, more Flight control keys, and an Artifacts view for "everything that was created or used during a session".
Approved on `?walk=compactSessionView`; the calls were the thread's last card (`?walk=compactCallsInThread`) until 17 Oct 2026, when they became the Needs you panel on the composer.
`docs/concepts/cockpit.md` has the layout.

| On screen | From |
|---|---|
| The thread, and its last card | The plan awaiting approval. A question, a judge's concern and the rest are rows in the Needs you panel on the composer |
| The Field | The whole repository from `list_repository_areas`, with the plots its plan's scope and drones name outlined. Its drones from `work.field`, only the Job's own and the subagents at work: one whose `subagent` row is `spent` has left it. Every other live Job in the repository faded. The glow and the slider from its `session.edited` and `session.read` timeline entries. The pad's lights from the thread's `check` rows and a blamed file from a failing row's `target`; a walk from a `window` row with no `ended_at`; thinking from four seconds with no `tool` row or tool entry while the turn works; a call from `session.called`; the strip's subagents from the ledger's `subagent` rows. `docs/concepts/cockpit.md`, *A Job open* |
| Progress | The workflow's steps as a line of glyphs, each a hover card (`Build · current`), and the plan's groups and tasks under it. It is the old Status and Plan as one panel |
| Flight control | The Job's own tiles; Artifacts, Subagents, Checks and Bays open a layer over the thread, and Requests opens the side panel |
| Requests rows | A `requested` row for the person's send and a `saved` row for the agent's: each opens Requests for the Job. `docs/concepts/requests.md` |

| Layer | Key | Reads |
|---|---|---|
| Artifacts | `8` | The Job's rows as they stand: `pr`, `artifact` (pages, files, docs, images), `sketch`, `subagent`, `background`, `studio` and `started_to` (the Jobs it handed a plan group to, each with the group and its state), the files its drones touched (`touches`), and from the thread the commands it ran and the commits it made. **The wire holds no commit record, so a commit is the thread's `git commit -m` line**, not Fleet's observation,. A field at the top narrows every group as it is typed in |
| Subagents | `9` | Every `subagent` row, the running ones first. A press opens its own thread |
| Checks | `0` | The `check` rows of the thread, newest first. A press opens the run on the Checks page |
| Bays | none | Every bay the Job holds, or held last before it stranded, each with its lease and how far it is behind its base, and the Job's write clashes: `slot-trouble.ts`. **Lit only when a bay is stranded or 20 or more commits behind.** A write clash puts the swords glyph at the key's corner and does not light it. A stranded bay is also a call in the thread's last card. A bay behind and a write clash are not: Fleet tells the agent, so they leave the owner nothing to do. Off, with its reason, where the Job holds no bay |

A layer covers the thread and the composer, which stay mounted under it: Back (the key again, the button, or Esc) returns to the thread at the same offset with focus where it was.
The arrows walk its rows, and focus goes in when it opens, to the search field in Artifacts and to the first row elsewhere.
Esc is the layer's while one is open, and a sheet over it keeps its own.
The layer is dropped when another Job is opened.
