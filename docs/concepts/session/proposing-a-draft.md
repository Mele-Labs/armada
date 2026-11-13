# Proposing a draft

**Part of:** [Job](../session.md).

**Text the person should OK before the job uses it goes to him as a draft, not into the reply.** A job calls `propose_draft`, a tool on the agent's door, with a `title`, the `text` in markdown and an optional `before`.
The `title` is one short line naming what the draft is and where it goes.
`before` is the text the draft replaces, passed when that text exists.
A pull request description, an issue or pull request comment, a message to someone and a section of a doc are drafts.

| | |
|---|---|
| What the call does | Puts the draft in the job's thread in Bridge as a card and answers at once. **It does not wait**: the job ends its turn, and the answer arrives as a message from the person |
| How it waits | `draft:<row id>` on the review sheet, which the card's Review row opens, however far back the thread holds its card. Selecting it there draws the draft with its answers. Its line under Now opens the card as a tab on the job. The job adds no `waiting_for` item for it, since Fleet lists it |

**He answers it one of four ways, and the agent is told which.**

| He | The job gets |
|---|---|
| Approves it as written | A message to apply the draft as written |
| Fixes the text in the card and approves | A message with the exact text to use |
| Sends it back with a note | A message with the note, to rewrite the draft and propose it again |
| Withdraws it, with or without a reason | `Withdrawn: <title>` and `Do not propose it again.`, with his reason on the next line when he gave one |

**Withdraw is a key at the footer's end, in the card's read and in its note mode, and not while he is editing.** In the note it sends what he typed as the reason; the Needs you item's Dismiss does the same with the reason it asks for.
The card then reads Withdrawn, with the reason quoted under the text.

**The job takes a draft back with `withdraw_draft`** (`draft`, the `draft:<row id>` its receipt gave, and `session_id`) when the draft no longer applies.
The row settles `withdrawn` and the draft leaves the Needs you list; the job is sent nothing, since it withdrew it.
Only an open draft the job proposed can be withdrawn: another job's draft, a settled one or an unknown id is refused.

**A draft proposed under an earlier draft's title is its next revision**, and an earlier one still open settles `withdrawn` with nothing sent: [Artifact](../artifact.md).

**A draft that should have been a Document becomes one on his press**, with `convert_draft_to_document` (`session_id`, `draft_id`).
Fleet writes the draft's text as `.armada/documents/<its title as a slug>.html` in the job's worktree, with `-2`, `-3` and so on where the name is taken, and answers the page's `name` and `path`.
The draft settles `withdrawn`, as `withdraw_draft` settles it, and the page is kept and asked as the job's own write is (*Document*, [document.md](../document.md)).
The job is sent `The owner turned your draft "<title>" into the Document .armada/documents/<name>.html. Revise it there as a Document from now on, and don't propose it as a draft again.`
A draft that proposes a workflow, rule or lifecycle change, a settled draft and a job with no worktree are refused.
