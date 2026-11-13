# What an artifact is for: `for_line` and `ArtifactFor`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 13 Nov 2026. A sketch, a draft or a page says what it is for in one sentence, with links. `docs/concepts/artifact.md`, *The For line*.
**Additive only**: two optional fields on two agent bodies and one optional field on four served shapes. No route, event kind or error kind is new; `fleet.draft_refused` is a new 422 code.
Bridge's half is `packages/protocol/src/job-artifacts.ts`, written by hand. Classic does not read it.

| Where | Side | Carries |
| --- | --- | --- |
| `ShowSketch.for?`, `ProposeDraft.for?` | Agent writes | One sentence, at most 200 characters. Over that is 422 `fleet.sketch_refused` or `fleet.draft_refused`, naming the limit |
| `ShowSketch.links?`, `ProposeDraft.links?` | Agent writes | `ForLink[]`: `label`, `url`. At most 5, each `http://` or `https://`; else 422 naming the link |
| `JobArtifact.for_line?` | Fleet writes, Bridge reads | `ArtifactFor` of the newest revision |
| `ThreadSketch.for_line?` | Fleet writes, Bridge reads | On the latest card of the title only, as `asked` and `settled` are |
| `ThreadDocument.for_line?` | Fleet writes, Bridge reads | The page's, read off its head |
| `SessionRow::Draft.for_line?` | Fleet writes, Bridge reads | The draft's, kept with the row |

**`ArtifactFor`** is `text?` and `links`, the agent's links first. **`ArtifactLink`** is `kind`, `label` and `target`.
`kind` is `link` (the agent's address), `task` (a plan task's id), `plan_step` (that task's group id) or `pull_request` (`#<n>`).

**Kept in the artifact's `detail` under `for`**, as JSON text, so it rides in `SessionRecord.attachments` as any detail key does.
Sketches, pages and drafts rewrite their detail whole on a new revision, so each revision has its own, and the attachment history keeps the earlier ones.

**A page is read, not refused.** Its `<meta name="armada-for">` and `<link rel="armada-for">` tags are read where its title is, a link that is not `http(s)` or past the fifth is dropped, and the Job is told which in the `document_sketch` notice.

**Fleet's links** are read from the Job's plan (the task in progress, newest started where several are, and its group) and its `pr` attachments not given back and not merged or closed.

**The event stream.** No event kind is new and no publish is added: the line rides on the writes that already publish. The unmeasured sink risk is no worse.

**Skew.** The wire surface changes, so the protocol ID moves and a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
