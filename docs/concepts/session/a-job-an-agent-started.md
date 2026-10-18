# A Job an agent started

**Part of:** [Job](../session.md).

A Job's agent may start a Job, or fork its own conversation into one, through two tools on its door (23.80). The owner approved it on 12 Oct 2026 on the condition that **the new Job waits for him**.

| | |
|---|---|
| `start_another_session` | `manifest_id`, `title`, `message`. A new Job in any repository Fleet serves |
| `fork_this_session` | `title`, `message`, optional `at`. A copy of the caller's own conversation, to the row `at` or to its latest message of its own that the transcript holds. Allowed while the caller is live, since it is a fork at a message |
| The caller | Placed by the connection of its hosted process, as `show_window` places it. **No call names a Job**, so an agent cannot fork another's conversation, and a call from no hosted Job is refused 422 `fleet.agent_start_unplaced` |
| Not offered | Tune, close, send and every other hosted-session command stay `agent_access` No |

> **Rule.** An agent's Job is created and listed at once, and nothing runs until the owner sends its first message.
> Why: a Job that started itself is a model spending tokens in a repository nobody chose to open. Holding the message makes the start a proposal he reads and may edit.

**The first message is kept as an `unsent` attachment** on the new Job, and read back on every read as a waiting item: id `unsent:<session id>`, text *First message*, source `first_message`, act `message` on the Job.
It has its own source and act so the Cockpit draws no reply box for it and a press opens the Job.
Bridge puts the words in the new Job's message box once, where the box is empty.
His ordinary send (`send_session_message`) sends what the box holds, starts the first turn, and spends the attachment and so the item; editing first is the same call.
`answer_waiting` refuses the item, since sending is the answer.
Ending the Job discards both.
While sleep is on, the item is left on the list and not decided for him.

**The brief is read at creation**, so the proposal (title kept as the agent gave it, workflow, model, effort) is on the new Job before he sends, and his send goes out on it.
A fork is not read (*A Job's proposal*).

**Both ledgers record it.** A start writes `started_by` on the new Job and `started_to` on the caller; a fork writes the `forked_from` and `forked_to` of *A forked job*, with `at` and `cut`.
The caller's thread gets a `started` row naming the Job and whether it is a fork, which Bridge draws as the press that opens it.
The new Job's thread opens under a line naming the Job whose agent started it.

**Nothing bounds how many an agent starts.** Each waits for him, so none spends anything, but a loop would fill the list. A limit waits for one being seen.
