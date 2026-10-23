# Saved issue queries and the query writer

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/tracker.md`, *Saved queries and the query writer*.
The protocol ID moves with the files below, and there is no number to take.
Bridge's half is `packages/protocol/src/issues.ts`, written by hand, field for field.

| Where | Side | Change |
| --- | --- | --- |
| `TrackedIssue` (`list_issues`) | Fleet writes, Bridge reads | Gains `author` (absent where the tracker did not say), `state` (`open` or `closed`), `fields` (`{ name, values }`, the tracker's own names) and `holders` (Job ids off the ledger) |
| `IssueRead` (`get_issue`) | Fleet writes, Bridge reads | Gains `fields` and `holders` |
| `search_issues` | Bridge calls | `POST /issue_queries/:repository/search`, body `SearchIssues { query }`, answers `IssueList`. A command in the inventory because the route table ties GET to queries |
| `list_issue_queries` | Bridge calls | `GET /issue_queries/:repository`, answers `IssueQueries { manifest_id, guide?, refused?, saved }` |
| `write_issue_query` | Bridge calls | `POST /issue_queries/:repository/write`, body `WriteIssueQuery { words }`, answers `WrittenIssueQuery { query, row_fields, issues, cost_micros? }` |
| `save_issue_query` | Bridge calls | `POST /issue_queries/:repository`, body `SaveIssueQuery { id?, name, words, query, row_fields }`, answers `SavedIssueQuery`. An `id` edits |
| `forget_issue_query` | Bridge calls | `POST /issue_queries/:repository/:id/forget`, answers `IssueQueries` |

**Errors.**
422 `fleet.issue_query_refused` carries the tracker's own words, from a search, a write or a save.
422 `fleet.issue_query_blank` is blank or over-long words, a blank name, or a blank query.
500 `fleet.issue_query_not_written` is a model call that failed or an answer that could not be used.
404 `fleet.no_such_issue_query` is an id the repository has none of.
500 `fleet.issues_unreadable` is a tracker that cannot be queried at all, said before any model call is made.

**The routes sit under `/issue_queries`** and not under `/issues/:repository/...`, so a reference can never be read as one of them.
**The wire carries the guide's name, its two queries and its field names, and never the prose a model reads.**
**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream.** Untouched: no event carries a query or an issue, so the back-pressure risk below is neither better nor worse.
The lifeboat is untouched.
