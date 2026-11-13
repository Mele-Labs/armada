# Snoozing a Job

**Part of:** [Job](../session.md).

**A snooze keeps a Job quiet until a time, and the Job keeps working.** `snooze_session` (`POST /sessions/snooze`) takes `session_id` and `until`, an instant, and the row carries `snoozed_until`.
Its turns, Checks and watches run as before, and its `waiting_for` items stay on the record, so nothing it raises is lost.
While it stands, Pocket sends no push for the Job's new items and the Operator raises nothing for it.
A client leaves the Job out of its banners, sound and dock count by reading `snoozed_until`.
The presets (an hour, three hours, tomorrow, a time he picks) are the client's to work out; Fleet takes the instant.

**When the time comes, the Job is back with everything it raised.** Each tick of Fleet's notice loop clears a snooze whose time has passed and publishes `session.changed`.
`snooze_session` with no `until` wakes the Job at once in the same way.
Pocket then sends one push for the Job, titled with the Job and saying its newest item, if it raised anything while snoozed that is still open.
Items raised before the snooze were pushed then and are not pushed again.
The Operator reads the Job's list again on that `session.changed`.

**A snooze outlasts what else happens to the Job.** It is kept in its own table (`session_snooze`), so a Fleet restart keeps it, and ending, pausing or repairing the Job leaves it standing.
Snoozing again moves the time.

| Refused | Code |
|---|---|
| An ended Job ("This Job has ended, so it cannot be snoozed.") | 409 `fleet.session_not_snoozable` |
| An `until` that is not a time | 422 `fleet.snooze_unreadable` |
| No such Job | 422 `fleet.no_such_session` |

A wake is never refused for an ended Job, or for one that is not snoozed: it answers the row.
