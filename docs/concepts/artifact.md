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
| His revisions, the For line, archive, a plan's sketches, answering several at once | Not built |

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
`withdraw_sketch` and `withdraw_document` write the dismissal his Dismiss writes, so a sketch or a page the Job took back reads `dismissed`.

**Rule.** An artifact that asks names the Needs you item that answers it: `sketch:<title>`, `document:<path>` or `draft:<row id>`.

## Drafts are keyed by title

**Rule.** A draft proposed under an earlier draft's title is the next revision of that draft.
The owner ruled this on 13 Nov 2026, so a draft revises as pages and sketches do.

**Rule.** Where the earlier draft is still open, it settles `withdrawn` and the Job is sent nothing, since it replaced it.
His Needs you list holds one item for the title.

**Rule.** A draft Fleet raises for a workflow change is not an artifact.
