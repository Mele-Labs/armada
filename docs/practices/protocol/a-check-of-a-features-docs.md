# A check of a feature's docs against its code: `check_docs`, `dismiss_doc_flag`, `start_doc_job` and `check` on `FeatureRead`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 24 Nov 2026. `docs/concepts/feature-catalog.md`, *Librarian*.
**Additive only**: three operations and routes, one optional field on an existing type; no event kind.
There is no number to bump: the protocol ID moves with `crates/ipc/src`, the operation files and the route table.
Only the native Bridge reads it; `packages/protocol` mirrors no feature type, and Classic draws nothing of it.

| Where | Side | Carries |
| --- | --- | --- |
| `check_docs`, `POST /features/docs/check` | Bridge sends `CheckDocs`, Fleet answers `DocsChecked` | `project` and `slug`. The answer: `checked` (each a `CheckedDoc`, `target` and `check`), `unchecked` (docs left for want of a call in the night's share) and `calls` |
| `dismiss_doc_flag`, `POST /features/docs/dismiss` | Bridge sends `DocFlag`, Fleet answers it | `project`, `slug` and `target`, the doc as the page's `docs` name it |
| `start_doc_job`, `POST /features/docs/job` | Bridge sends `DocFlag`, Fleet answers `DocJobStarted` | `project`, `slug`, `target`, `session_id`, `title`, `brief` and `send_failed`, absent where the first message was sent |
| `FeatureRead.check` (`get_feature_page`'s `docs`) | Fleet writes, Bridge reads | Absent where no press checked the doc. A `DocCheck`: `at`; `disagrees`, absent where they agree, else `doc_quote`, `code_at` (`path:line`) and `code_quote`; `dismissed` where he dismissed it |

**Only his press runs a check.** Each call is spent from `features.librarianModelShare`, counted with the librarian's runs, so a press leaves a run that much less and a run a press.
Where none is left the press is a 409 `fleet.librarian_share_spent`, and Bridge shows Fleet's sentence in the Docs card's header line.
Its other refusals are 404 `fleet.feature_unknown`, 422 `fleet.feature_project`, `fleet.feature_no_repository` and `fleet.no_doc_to_check`, and 500 `fleet.doc_check_failed`; Dismiss and Start a Job are a 409 `fleet.doc_not_flagged` where no disagreement stands.

**All three are the person's**, as `answer_retire` is: the Operator through the door, Bridge on the port, and a 403 `api.the_persons_own` from inside a Job.
Start a Job's brief is Fleet's, written from what it kept, so Bridge sends only which doc.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
An older Fleet's page has no `check`, which a newer Bridge reads as a doc no press has checked.
**The event stream.** No new event: the answer carries what the press found, and the next page read carries it after a restart.
