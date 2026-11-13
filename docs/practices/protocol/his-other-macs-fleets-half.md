# His other Macs, Fleet's half: `share_sessions` and ten more

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Two of his Macs pair, and each Mac's Phone Gateway holds a signed link to the other's.
The Gateway runs inside Fleet's process and reaches Fleet only over loopback, so what it sends the other Mac and what it takes from it are Fleet routes like any other.
The pairing and the link are the Gateway's: [Phone](../../concepts/pocket.md).

**What leaves this Mac has passed the share filter** (`crates/fleet/src/share.rs`), read at the moment of sending: only a Job in a project with `shared_with_macs` on, its thread, its live changes and its bug reports, and the shared projects themselves with their tasks.
A Job in an unshared project and a Job in none are one case, and nothing of either leaves.
**What arrives is kept with the sending Mac's id**, in the store's `other_mac_sessions`, `other_mac_bug_reports` and `other_mac_projects`, so a Mac that is asleep still has its rows drawn after Fleet restarts.
No thread is kept: it is read through the link each time.

A Job held from another Mac is not one of this Mac's Jobs.
An act on it is sent to the Mac that holds it, as *Acting across Macs* below says, and is never run here.
What is said of it reaches the event stream only: Fleet's own listeners, such as the calls on the person, never hear it.
A row or project held from another Mac never goes back out through `/other_mac/*`, so what one Mac shares never reaches a third.

**A project held from another Mac is a copy, and the Mac that holds it is its authority.**
A task act on a copy is sent through the link to that Mac, which runs it with its own code and answers its shared projects, and the answer is the copy.
Where that Mac cannot be reached, the act is kept in `other_mac_project_acts`, shown on the copy at once, and sent oldest first when the link next opens, each with `at`, when he made it.
The holder keeps when each field of a task (title, notes, state, place) was last edited, and an act older than a field's time leaves that field as it is.
The protocol ID moves with the files below, and there is no number to take.
Bridge's half is `packages/protocol/src/macs.ts`, written by hand, field for field.

| Where | Side | Change |
| --- | --- | --- |
| `SessionRecord.mac` | Fleet writes, Bridge reads | `mac?: string`, the id of the paired Mac that holds the Job. **Absent means this Mac.** |
| `BugReport.mac` | Fleet writes, Bridge reads | `mac?: string`, the id of the paired Mac whose Job filed it. Absent means this Mac |
| `list_sessions` | Bridge calls | Lists the rows held from every paired Mac beside this Mac's own, in every view, under the same `manifest_id`, `q` and `state` filters, newest seen first. A `summary` row carries `mac` as the record does, absent for this Mac's own |
| `session.removed` | Fleet sends | `{ session_id }`: a row held from another Mac went, so a client holding it drops it. Sent once for each row a `hold_other_mac_sessions` replace leaves out and each row a `forget_other_mac` drops. Never sent for this Mac's own Jobs, and never sent on to another Mac |
| `Project.mac` | Fleet writes, Bridge reads | `mac?: string`, the id of the paired Mac that holds the project, where this is a copy. Absent means this Mac's own |
| `list_projects` | Bridge calls | Lists the copies of every paired Mac's shared projects after this Mac's own, each with `mac`. A task act on a copy's row goes to its Mac, as `project_act_from_mac`. Its name, members, values, place, share switch and deletion, a Job link, a repository move and a move between Macs are 409 `fleet.other_mac_project_held`, "Only Mac <id> can change this project." |
| `projects.changed` | Fleet sends | Carries this Mac's projects and every copy, whole, whichever changed |
| `list_bug_reports` | Bridge calls | Lists the reports held from every paired Mac beside this Mac's own, newest first |
| `bug_reports.changed` | Fleet sends | Carries this Mac's reports and every other Mac's, whole, whichever Mac's changed |
| `get_job_thread` | Bridge calls | For a Job held from another Mac, read from that Mac's Fleet through the link, as `share_job_thread` there. What it refuses is passed on with its own code and sentence; a link that cannot reach it is 500 `fleet.other_mac_unreachable` |
| `share_sessions` | the Gateway calls | `GET /other_mac/sessions`, answered with `SessionList`: this Mac's own Jobs in a shared project, whole |
| `share_bug_reports` | the Gateway calls | `GET /other_mac/bug_reports`, answered with `BugReports`: this Mac's own reports whose Job is in a shared project |
| `share_projects` | the Gateway calls | `GET /other_mac/projects`, answered with `Projects`: this Mac's own shared projects with their tasks; never a copy, never a loose task |
| `share_events` | the Gateway calls | `GET /other_mac/events`, a WebSocket framed as `/events`: `resync` first, then each event the share filter lets through, whole or cut, at its own cursor; `missed` and `resync` pass as they are |
| `share_job_thread` | the Gateway calls | `GET /other_mac/thread?session_id=&before=&limit=&upto=`, answered as `get_job_thread`. Any Job not shared is 404 `fleet.no_such_session`, worded as for a Job that does not exist |
| `share_job_page` | the Gateway calls | `GET /other_mac/page?session_id=&port=`, answered 204, before each page request another Mac sends: the Job shared, and `port` one a window it showed or a server it started that is serving stands on, on loopback. Any Job not shared is 404 `fleet.no_such_session`; a port it does not show is 404 `fleet.other_mac_page_not_shown` |
| `open_other_mac_page` | Bridge calls | `POST /sessions/other_mac_page`, body `OpenOtherMacPage { session_id, url }`, answered `OtherMacPageOpened { url }`: for a Job held from another Mac, an address on this Mac's loopback the Gateway signs through to it, path and query kept. That Mac asleep is 409 `fleet.other_mac_asleep` with `fields.since` and `fields.mac`. See *Pages across Macs* |
| `hold_other_mac_sessions` | the Gateway calls | `PUT /other_macs/sessions`, body `OtherMacSessions { mac, sessions }`: every row that Mac shares, replacing all held from it, each stamped with `mac`. `session.changed` for each row new or changed, `session.removed` for each row left out. Answers `MacNamed` |
| `hold_other_mac_projects` | the Gateway calls | `PUT /other_macs/projects`, body `OtherMacProjects { mac, projects }`, replacing every copy held from it, then `projects.changed` where a copy changed. Edits queued for that Mac are then sent. Answers `MacNamed` |
| `project_act_from_mac` | the other Mac's Gateway calls | `POST /other_mac/project_act`, body `ProjectActFromMac { act, at? }`, `act` one of a copy's task acts tagged by `kind`. Run only in a shared project; any other is refused as a row or project that does not exist. Answers `Projects` as `share_projects` |
| `hold_other_mac_bug_reports` | the Gateway calls | `PUT /other_macs/bug_reports`, body `OtherMacBugReports { mac, reports }`, replacing all held from it, then `bug_reports.changed`. Answers `MacNamed` |
| `hold_other_mac_event` | the Gateway calls | `POST /other_macs/event`, body `OtherMacEvent { mac, event }`. `session.changed` is kept and published; `session.row`, `session.subagent`, `job_thread.tail` and `job_thread.changed` are published only for a Job held from that Mac, else 404 `fleet.other_mac_job_not_held`; `bug_reports.changed` is kept as the `PUT`; `projects.changed` replaces the copies as the `PUT` does; any other kind is 422 `fleet.other_mac_event_not_kept`. Answers `MacNamed` |
| `act_from_mac` | the other Mac's Gateway calls | `POST /other_mac/act`, body `MacAct { id, act, at }`, header `x-armada-mac`. See *Acting across Macs* |
| `list_queued` | Bridge calls | `GET /queued`, optional `?mac=`, answered with `QueuedList { queued }` of `Queued { id, mac, act, since, refused? }`, oldest first |
| `take_back_queued` | Bridge calls | `POST /queued/take_back`, body `TakeBack { id }`, answered with `QueuedList`; 404 `fleet.queued_not_found`, 409 `fleet.queued_in_flight` |
| `queued.changed` | Fleet sends | `QueuedList`, whole, to Bridge alone, whenever an act is kept, sent, refused or taken back |
| `forget_other_mac` | the Gateway calls | `POST /other_macs/forget`, body `MacNamed`: every row, report, project copy and queued edit held from it dropped, `session.removed` for each row, then `bug_reports.changed`. Answers `MacNamed` |

A blank `mac` on any of the five the Gateway hands in is 422 `fleet.other_mac_unnamed`.

**Only the Gateway hands Fleet another Mac's rows.**
The five routes it hands them in on, and `project_act_from_mac`, are in `api::callers::THE_PERSONS_OWN`, so a call to one from inside a Job, around the door, is 403 `api.the_persons_own`: "only the person pairs his other Macs, from Bridge".
This Mac's Phone Gateway calls them from inside Fleet's own process, which no Job holds.
The call is placed by the process holding the connection, never by `x-armada-caller`, and the Gateway runs in Fleet's own process, which no Job holds.

**A row a replace or a forget drops leaves `list_sessions` at once, and `session.removed` says so.**
Electron's main process ignores a kind it does not know (`apps/desktop/src/main/arrivals.ts`, the last lines of `applyArrival`), so Electron keeps drawing the row until its next resync.

## Acting across Macs

An act on a Job held from another Mac goes to that Mac: `send_session_message`, `answer_session_ask`, `answer_waiting`, `answer_waiting_parts`, `approve_plan`, `send_plan_changes`, `answer_sketch`, `answer_document`, `stop_session` and `merge_pull_request_by_number` find that no Job of this Mac has the id, and send the act as `MacAct { id, act, at }` to the Mac that holds it.
**`merge` goes only where no Job of this Mac owns the pull request and a Job held from another Mac does**, and that Job's id rides in the act.
`/sessions/start` and `/sessions/dispatch` read `mac` beside their own body (`StartOn`, `DispatchOn`): absent runs here as before, a Mac this Fleet holds anything from sends the start there, and any other is 404 `fleet.other_mac_not_linked`.
The row that comes back is kept as that Mac's, and answered with its `mac`.
`act` is `{ kind, body }`, `kind` one of `message`, `ask_answer`, `waiting_answer`, `waiting_answer_parts`, `plan_approve`, `plan_changes`, `sketch_answer`, `document_answer`, `stop`, `merge`, `start` and `dispatch`, `body` the body of the route of that name, or `job_message`, which no route of the person's takes (*Messages and alerts*).
The tool-gate hook is not carried, since no person makes it.

**The holding Mac** runs the act through the handler its own route has, so each status and refusal is the one it gives at home, under the door `mac` (`x-armada-caller: mac`, with the sender in `x-armada-mac`; `Via::Mac`, a person's own press on another of his Macs).
It first reads the share filter: a Job not in a shared project, one it does not hold and one held from a third Mac are 404 `fleet.no_such_session`, a start into a project not shared, or into one without that repository, 404 `fleet.no_such_project`, and nothing is written, a stored answer included.
A start that names `fork`, `root` or `after` is 422 `fleet.start_from_mac_reaches`; `on_your_words` is dropped.
It runs each `(Mac, id)` once: the status and body of the first answer under 500 are kept in `other_mac_acts_run` for a week with the door and the act's kind, and a repeat is answered with them.

**The sending Mac keeps an act it cannot send.** A call that failed, a 401 and a 5xx are not answers; the act is kept in `other_mac_project_acts`, the queue the project edits use, with `kind` `job`, its `id` and `refused`, and answered 202 with the `Queued`.
A Mac with acts waiting is never sent a newer act ahead of them.
They are sent oldest first when the Mac next speaks, by the same task as the edits, each under its own `id`.
An answer drops it, and a 4xx keeps it with the Mac's words in `refused`, no longer sent, and the acts after it go on.
`take_back_queued` drops one under the Mac's send guard, and is 409 while that Mac's queue is being sent.

## Messages and alerts

**A Job's `SendMessage` to a Job his other Mac holds crosses as one more act**, `job_message`, body `JobMessage { from, title, to, text }`, through the door and queue above.
`message_reach` allows an address no Job of this Mac has where it is a live Job held from another Mac and the writer is in a shared project; a private writer is refused as for an address no Job has (`refused_to`, `Unreachable::Unknown`), so it learns nothing of what the other Mac holds.
The holding Mac's share filter answers for the receiver, so a message into a private Job is 404 `fleet.no_such_session` as for no Job; one into an ended Job is 409 `fleet.message_job_ended`, and the writer gets a Fleet note with its words.
The receiver's row is the `Message` a Job on the same Mac gets, with `SessionVoice::Session.mac` the sending Mac's id.
**The sender's own row stays the call's `Tool` row**: its `message.to.session_id` names the receiver, whose list row carries `mac`.

**The other Mac's shared Jobs call here as this Mac's do.** `relay` reaches no tap, so the rows held from another Mac are handed to the calls keeper where they are kept: each changed row of `hold_other_mac_sessions` and each `session.changed` of `hold_other_mac_event`.
A Mac's first rows, with none held from it before, are heard quietly: what stood before the link is no news.
A row that comes back unchanged after a sleep is not heard again, and a row a replace or a forget drops leaves the calls.
`StandingCall.mac` names the Mac; `CallArrival` carries none, and Bridge reads it off the call or the row.
`get_needs_you` reads a Job held from another Mac off its held row.

## Pages across Macs

A walk or a dev server of a Job held from another Mac opens on this one through the signed link, and nowhere else.
Bridge asks `open_other_mac_page` with the address the row names, on the other Mac's loopback.
This Mac's Gateway asks the other's `/api/mac/page`, signed, which that Fleet answers as `share_job_page`, and then opens one loopback port here for that Job and page port.
Every request to that port is signed and sent to `/api/mac/page/<job>/<port>/<path>` on the other Gateway, which asks `share_job_page` again and only then sends it on to `127.0.0.1:<port>`.
**A port of its own keeps the page's origin**, so its absolute paths resolve as they do at home.
An unsigned call gets no page, and turning sharing off stops an open page at its next request.
Pages are read whole, no redirect is followed, and a dev server's live-reload socket does not cross: he reloads by hand.

