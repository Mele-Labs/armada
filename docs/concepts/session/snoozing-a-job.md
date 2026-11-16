# Snoozing a Job

**Part of:** [Job](../session.md).

**A snooze pauses a Job and keeps it quiet until a time.** `snooze_session` (`POST /sessions/snooze`) takes `session_id` and `until`, an instant, and the row carries `snoozed_until`.
Fleet pauses the Job as `pause_session` does: a turn running is ended as Pause ends it, the work is parked on the branch and the slot goes back to the pool, so the row carries `hosted.paused` too.
Its `waiting_for` items stay on the record, so nothing it raised is lost.
A Job the person paused before snoozing it stays paused as it was, and a Job running in a terminal is snoozed without a pause, since only he can stop it there.
While it stands, Pocket sends no push for the Job's new items and the Operator raises nothing for it.
A client leaves the Job out of its banners, sound and dock count by reading `snoozed_until`.
The presets (an hour, three hours, tomorrow, a time he picks) are the client's to work out; Fleet takes the instant.

**When the time comes, the Job is back with everything it raised, and working again.** Each tick of Fleet's notice loop clears a snooze whose time has passed and publishes `session.changed`.
`snooze_session` with no `until` wakes the Job at once in the same way.
If the snooze paused the Job, Fleet resumes it as `resume_session` does, and a Job with no turn kept meanwhile is told its snooze has ended.
The snooze row keeps the pause marker it set (`session_snooze.paused_at`), and only a Job whose pause is still that one is resumed.
A Job the person paused before the snooze stays paused, and one he resumed during it is left as he left it, paused again or not.
Pocket then sends one push for the Job, titled with the Job and saying its newest item, if it raised anything while snoozed that is still open.
Items still waiting out Pocket's 60 second hold when the snooze began count as raised while snoozed.
Items raised before the snooze were pushed then and are not pushed again.
The Operator reads the Job's list again on that `session.changed`.

**A snooze outlasts what else happens to the Job.** It is kept in its own table (`session_snooze`), so a Fleet restart keeps it and still resumes the Job at its time, and ending, pausing or repairing the Job leaves it standing.
Snoozing again moves the time, and the Job is still resumed when it comes.

| Refused | Code |
|---|---|
| An ended Job ("This Job has ended, so it cannot be snoozed.") | 409 `fleet.session_not_snoozable` |
| An `until` that is not a time | 422 `fleet.snooze_unreadable` |
| No such Job | 422 `fleet.no_such_session` |
| A pause Pause itself refuses, such as a slot another holder now has | 409 `fleet.session_pause_refused`, nothing snoozed |

A wake is never refused for an ended Job, or for one that is not snoozed: it answers the row.
