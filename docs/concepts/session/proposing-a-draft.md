# Proposing a draft

**Part of:** [Job](../session.md).

**Text the person should OK before the job uses it goes to him as a draft, not into the reply.** A job calls `propose_draft`, a tool on the agent's door, with a `title`, the `text` in markdown and an optional `before`.
The `title` is one short line naming what the draft is and where it goes.
`before` is the text the draft replaces, passed when that text exists.
A pull request description, an issue or pull request comment, a message to someone and a section of a doc are drafts.

| | |
|---|---|
| What the call does | Puts the draft in the job's thread in Bridge as a card and answers at once. **It does not wait**: the job ends its turn, and the answer arrives as a message from the person |
| How it waits | `draft:<row id>` in the Needs you panel. The job adds no `waiting_for` item for it, since Fleet lists it |

**He answers it one of three ways, and the agent is told which.**

| He | The job gets |
|---|---|
| Approves it as written | A message to apply the draft as written |
| Fixes the text in the card and approves | A message with the exact text to use |
| Sends it back with a note | A message with the note, to rewrite the draft and propose it again |
