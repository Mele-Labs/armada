# Editing a sent message

**Part of:** [Job](../session.md).

The person can change the words of a message he sent to a hosted Job.
It sends `edit_session_message` (`POST /sessions/message/edit`) with the Job, the row's id and the new words.
Only his own messages are edited: the agent's rows, another Job's and Fleet's are refused, as is an ended Job and a terminal Job, whose process Fleet does not host.

| Where the turn is | What changes | The agent |
|---|---|---|
| Held by Fleet: behind a proposal's reading, after a process that would not start, or kept for the Resume of a paused Job | The row's words and the held turn's | Reads only the new words. Nothing is sent, and the row is not marked |
| Written to the agent's input, which is where Fleet puts an ordinary message at once | The row keeps the new words, marked `edited_at` | Is sent a new turn saying what changed. Nothing is undone |

**In practice almost every edit is a correction**, since a message is written as it is sent.
The turn the agent reads is Fleet's: "The owner edited a message they sent earlier.", then `Before:` and the old words quoted, `Now:` and the new words quoted, and "Act on the new version where it changes anything."
**It is no row of the thread**: the edited bubble is what the person sees, so a correction is never drawn twice.
It waits behind the turns held for a proposal, and a paused Job keeps it for the Resume, as it keeps any turn.

**Words the same as the row's are no change**, and the edit succeeds with nothing sent.
**The first turn of a Job on its start card has no row yet**, so it is edited on the card, not here.
An edit changes no title, and reads no issue or link from the new words again.
