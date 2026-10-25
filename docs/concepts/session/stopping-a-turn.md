# Stopping a turn

**Part of:** [Job](../session.md).

**A Stop sits beside the working mark at the end of a hosted Job's thread**, and the same press is available wherever that mark is drawn with a turn running.
It sends `stop_session` (`POST /sessions/stop`, 23.84).
Fleet ends the process as it does after a quiet spell, withdraws a question the agent was held on, sets the turn to `idle` and publishes the row.
**The row stays live and the slot stays held**: nothing is parked or committed, which is what separates it from `close_session`.
The next message resumes the conversation.
The thread keeps every row the agent had written, and Stop draws no row of its own.

**A terminal Job has no Stop.** Fleet does not host its process and refuses the act.

**Debug Bridge starts a held Job with no turn.** `start_session` writes the row and runs nothing, so Bridge starts one for the note (*A held Job*).
The note is its first message, off the thread: his words, then the `Pointed at:`, `Candidates:` and `Page:` lines Fleet adds, with the picture.
The first turn is the person's Start, with whatever he added to the box.
`docs/practices/running-locally.md`, *Annotating Bridge*.
