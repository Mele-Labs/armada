# An issue is named by its tracker's reference

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/tracker.md`.
An issue crossed the wire as a number, which names nothing once a tracker has more than one project and drew as `#123` for Jira's `ABC-123`.
The protocol ID moves with the files below, and there is no number to take.

| Where | Side | Change |
| --- | --- | --- |
| `TrackedIssue.number` (`list_issues`) | Fleet writes, Bridge draws | Replaced by `reference`: the tracker's own name as a person writes it, `#123` on the forge and `ABC-123` on Jira. Bridge draws it as it is and never builds one |
| `IssueRead.number` (`get_issue`) | Fleet writes, Bridge reads | Replaced by `reference`, as above |
| `get_issue` path | Bridge calls | `GET /issues/:repository/:reference`, percent-encoded. A forge takes `214` or `#214`; Jira takes a key. A reference the tracker does not use, such as a bare number on Jira, is a 404 `fleet.no_such_issue` |
| `AutopilotPick.issue_number` | Fleet writes, Bridge draws | Replaced by `issue_reference`. The store keeps `issue_number` for picks written before the column, and reads one with no reference as `#` and its number |
| `getIssue` (preload) | Bridge | Takes the reference as text where it took a number |

**The `#` picker takes a key as well as digits.** An issue's tag id is its key where the tracker names issues by one, and the picker matches what is typed against the reference, ignoring case.
A lookup by number reads only numbers.
**The event stream is untouched**: no event carries an issue, so the back-pressure risk below is neither better nor worse.
The lifeboat is untouched.
