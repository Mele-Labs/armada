# Artifact

**What it is:** A sketch, a draft or a page a Job made for the person, read as one record with its revisions and where its newest stands.

---

**Kind:** Concept.

**Part of:** [Job](session.md).

Fleet serves a Job's artifacts with `get_job_artifacts` (`crates/ipc/operations/get_job_artifacts.toml`), built in `crates/fleet/src/session_host/artifacts.rs`.
The owner approved the design on 13 Nov 2026.

## Status

| Part | State |
|---|---|
| The record and `get_job_artifacts` | Built in Fleet; native draws it later |
| Drafts kept as artifacts keyed by title, with revisions | Built |
| Archive and restore, by the agent or by him | Built in Fleet; native draws the fold later |
| His revisions, the For line, a plan's sketches, answering several at once | Not built |

## What an artifact is

**Rule.** An artifact is something a Job made for the person to read or answer: a sketch from `show_sketch`, a draft from `propose_draft`, or a page under `.armada/documents/` ([Document](document.md)).
A file, an image, a window or a note shown to him is not one.

**Rule.** An artifact's id is the ledger target it is kept under, and it stays the same across revisions: `sketch:<title>`, the page's path, or `draft:<title>`.

**Rule.** Its form is `sketch`, `draft` or `page`.

**Rule.** Each artifact keeps every revision, numbered from 1, oldest first, as files in Fleet's attachments folder outside every worktree.
A sketch keeps `r<n>.json`, a page `r<n>.html` and a draft `r<n>.md`.
An identical write keeps no new revision.

**Rule.** Every revision today is the agent's.

## Where it stands

**Rule.** The state is for the newest revision: `reading`, `asking`, `approved`, `sent_back`, `withdrawn` or `dismissed`.
A page that does not ask reads `reading`; one that asks and whose file is gone reads `withdrawn`.
`dismissed` is his Dismiss, with no verdict.
`withdrawn` is the Job taking it back with `withdraw_sketch`, `withdraw_document` or `withdraw_draft`, or a draft replaced under its title.

**Rule.** `withdraw_sketch` and `withdraw_document` write the dismissal his Dismiss writes, and mark the attachment `withdrawn_by` `agent`.
The mark is what tells the Job's withdrawal from his Dismiss.
A sketch or page withdrawn before the mark existed reads `dismissed`.

**Rule.** An artifact that asks names the Needs you item that answers it: `sketch:<title>`, `document:<path>` or `draft:<row id>`.

## Drafts are keyed by title

**Rule.** A draft proposed under an earlier draft's title is the next revision of that draft.
The owner ruled this on 13 Nov 2026, so a draft revises as pages and sketches do.

**Rule.** Where the earlier draft is still open, it settles `withdrawn` and the Job is sent nothing, since it replaced it.
His Needs you list holds one item for the title.

**Rule.** A draft Fleet raises for a workflow change is not an artifact.

## Archive

**Rule.** The agent or he archives a stale artifact with `archive_artifact`: one a newer artifact replaced, or one that no longer bears on the work.
Fleet never archives one itself, since it cannot tell whether one artifact replaces another.
The owner ruled this on 13 Nov 2026.

**Rule.** An archived artifact folds in the Artifacts tab, and `restore_artifact` brings it back.
It says who archived it, `agent` or `owner`, and when.

**Rule.** An archived artifact that waited on him leaves his Needs you list with no verdict, by the dismissal his Dismiss writes.
Its state reads as it stood before, never `dismissed`, and it names no item.

**Rule.** Where he archives one that waited on him, the Job is sent `He archived "<title>". It is off his list with no verdict.`
Where the agent archives it, nothing is sent.

**Rule.** A new revision keeps the archive, so only a restore lifts it.
A sketch drawn again or a page revised asks him again all the same.

**Rule.** A restore lifts the archive and the dismissal the archive wrote, and nothing else.
An item answered meanwhile stays answered, so a restore never asks him again what he settled.
The Job is sent nothing.

**Rule.** The archive is kept in Fleet's store apart from the artifact's attachment, which a new revision rewrites whole.
