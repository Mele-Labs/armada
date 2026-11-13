# His other Macs, Fleet's half: `share_sessions` and seven more

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Two of his Macs pair, and each Mac's Phone Gateway holds a signed link to the other's.
The Gateway runs inside Fleet's process and reaches Fleet only over loopback, so what it sends the other Mac and what it takes from it are Fleet routes like any other.
The pairing and the link are the Gateway's: [Phone](../../concepts/pocket.md).

**What leaves this Mac has passed the share filter** (`crates/fleet/src/share.rs`), read at the moment of sending: only a Job in a project with `shared_with_macs` on, its thread, its live changes and its bug reports.
A Job in an unshared project and a Job in none are one case, and nothing of either leaves.
**What arrives is kept with the sending Mac's id**, in the store's `other_mac_sessions` and `other_mac_bug_reports`, so a Mac that is asleep still has its rows drawn after Fleet restarts.
No thread is kept: it is read through the link each time.

A Job held from another Mac is not one of this Mac's Jobs.
Every act on it is refused as now, as a Job Fleet does not hold, and what is said of it reaches the event stream only: Fleet's own listeners, such as the calls on the person, never hear it.
A row held from another Mac never goes back out through `/other_mac/*`, so what one Mac shares never reaches a third.
The protocol ID moves with the files below, and there is no number to take.
Bridge's half is `packages/protocol/src/macs.ts`, written by hand, field for field.

| Where | Side | Change |
| --- | --- | --- |
| `SessionRecord.mac` | Fleet writes, Bridge reads | `mac?: string`, the id of the paired Mac that holds the Job. **Absent means this Mac.** |
| `BugReport.mac` | Fleet writes, Bridge reads | `mac?: string`, the id of the paired Mac whose Job filed it. Absent means this Mac |
| `list_sessions` | Bridge calls | Lists the rows held from every paired Mac beside this Mac's own, in every view, under the same `manifest_id`, `q` and `state` filters, newest seen first. A `summary` row carries `mac` as the record does, absent for this Mac's own |
| `session.removed` | Fleet sends | `{ session_id }`: a row held from another Mac went, so a client holding it drops it. Sent once for each row a `hold_other_mac_sessions` replace leaves out and each row a `forget_other_mac` drops. Never sent for this Mac's own Jobs, and never sent on to another Mac |
| `list_bug_reports` | Bridge calls | Lists the reports held from every paired Mac beside this Mac's own, newest first |
| `bug_reports.changed` | Fleet sends | Carries this Mac's reports and every other Mac's, whole, whichever Mac's changed |
| `get_job_thread` | Bridge calls | For a Job held from another Mac, read from that Mac's Fleet through the link, as `share_job_thread` there. What it refuses is passed on with its own code and sentence; a link that cannot reach it is 500 `fleet.other_mac_unreachable` |
| `share_sessions` | the Gateway calls | `GET /other_mac/sessions`, answered with `SessionList`: this Mac's own Jobs in a shared project, whole |
| `share_bug_reports` | the Gateway calls | `GET /other_mac/bug_reports`, answered with `BugReports`: this Mac's own reports whose Job is in a shared project |
| `share_events` | the Gateway calls | `GET /other_mac/events`, a WebSocket framed as `/events`: `resync` first, then each event the share filter lets through, whole or cut, at its own cursor; `missed` and `resync` pass as they are |
| `share_job_thread` | the Gateway calls | `GET /other_mac/thread?session_id=&before=&limit=&upto=`, answered as `get_job_thread`. Any Job not shared is 404 `fleet.no_such_session`, worded as for a Job that does not exist |
| `hold_other_mac_sessions` | the Gateway calls | `PUT /other_macs/sessions`, body `OtherMacSessions { mac, sessions }`: every row that Mac shares, replacing all held from it, each stamped with `mac`. `session.changed` for each row new or changed, `session.removed` for each row left out. Answers `MacNamed` |
| `hold_other_mac_bug_reports` | the Gateway calls | `PUT /other_macs/bug_reports`, body `OtherMacBugReports { mac, reports }`, replacing all held from it, then `bug_reports.changed`. Answers `MacNamed` |
| `hold_other_mac_event` | the Gateway calls | `POST /other_macs/event`, body `OtherMacEvent { mac, event }`. `session.changed` is kept and published; `session.row`, `session.subagent`, `job_thread.tail` and `job_thread.changed` are published only for a Job held from that Mac, else 404 `fleet.other_mac_job_not_held`; `bug_reports.changed` is kept as the `PUT`; `projects.changed` is taken and nothing is done; any other kind is 422 `fleet.other_mac_event_not_kept`. Answers `MacNamed` |
| `forget_other_mac` | the Gateway calls | `POST /other_macs/forget`, body `MacNamed`: every row and report held from it dropped, `session.removed` for each row, then `bug_reports.changed`. Answers `MacNamed` |

A blank `mac` on any of the four the Gateway hands in is 422 `fleet.other_mac_unnamed`.

**Only the Gateway hands Fleet another Mac's rows.**
The four routes it hands them in on are in `api::callers::THE_PERSONS_OWN`, so a call to one from inside a Job, around the door, is 403 `api.the_persons_own`: "only the person pairs his other Macs, from Bridge".
This Mac's Phone Gateway calls them from inside Fleet's own process, which no Job holds.
The call is placed by the process holding the connection, never by `x-armada-caller`, and the Gateway runs in Fleet's own process, which no Job holds.

**A row a replace or a forget drops leaves `list_sessions` at once, and `session.removed` says so.**
Electron's main process ignores a kind it does not know (`apps/desktop/src/main/arrivals.ts`, the last lines of `applyArrival`), so Electron keeps drawing the row until its next resync.
