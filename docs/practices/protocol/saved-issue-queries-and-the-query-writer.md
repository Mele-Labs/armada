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

## A missing tool, named, and installed when asked

The protocol ID moves with `crates/ipc/src`, the operation file, the route table and `packages/protocol/src`, and there is no number to take.

| Where | Side | Change |
| --- | --- | --- |
| `fields.missing_tool` on the error of `search_issues`, `list_issues` and `get_issue` | Fleet writes, Bridge reads | A `WireValue` string, `bd`, where the tracker could not run a program that is not on the PATH Fleet runs with. Absent for every other refusal; `code` and `message` are as they were |
| `fields.needs_setup` on the same errors | Fleet writes, Bridge reads | A `WireValue` string, `bd`, where the program ran and found no beads project in the repository's main checkout. Absent for every other refusal |
| `install_tool` | command | `POST /issue_queries/:repository/install_tool`, body `InstallTool` (`tool`), answers `InstalledTool` (`tool`, `done`, `said`) |
| `fleet.tool_not_installable` | 422 | A `tool` Fleet does not install. Only `bd` is taken, and nothing is run for any other |

**Additive.** A Bridge that reads past `fields.missing_tool` and `fields.needs_setup` draws the refusal's message as before.

**Whether it worked is a 200.** `done: false` carries `said`: Homebrew's own words, or `Homebrew is not available`.
Fleet looks for `brew` in `/opt/homebrew/bin` and `/usr/local/bin` before its PATH, since Bridge starts it and a program
started from the Dock does not carry the shell's PATH.

**`install_tool` also sets `bd` up.** It installs only where `bd` is absent, then does what `scripts/beads-setup` does in the main checkout.
Where origin holds no `refs/dolt/data` it answers `done: false` with that in `said` and changes nothing else.
