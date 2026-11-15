# An aside

**Part of:** [Job](../session.md).

An aside is a side question the owner puts to a Job without interrupting it, as `/btw` does in a terminal.
Since 15 Nov 2026.
The answer is kept beside the Job's record and never in its thread, so the Job's agent does not see it unless the owner tells it.

| | |
|---|---|
| Asked | `ask_aside { session_id, question, quote? }`. `quote` is the text he selected in the thread. Answers 202 with the `Aside`, `asking` |
| The copy | Fleet runs one headless turn of the agent on a **throwaway copy of the Job's conversation**, cut at the Job's last finished turn (`latest_fork_point`), so a live Job is copied as it stood. A Job with no finished turn yet is refused, 422 `fleet.fork_point_unwritten` |
| Held to reading | `Read`, `Grep` and `Glob` and nothing else, `dontAsk`, no MCP server, no plugin, at most eight turns, on the Job's model. **No shell**: whether a command only reads cannot be enforced by a prefix rule, so none is offered. `.armada/` is denied as it is for the Judge |
| Never written | The copy is a fork under an id of its own with `--no-session-persistence`, so it is written nowhere and the Job's transcript is byte for byte what it was. Fleet removes a file under the copy's id afterwards if one exists |
| Answer | Streams as `session.aside`, the aside whole, at most about five a second and always the last. `list_asides` reads the Job's asides, oldest first, with the answer once it has ended |
| Told | `tell_aside { session_id, aside_id }` sends the Job one message of the owner's by the route any message of his takes: the quote where there was one, the question and the answer. Once. An aside that is still asking or failed is refused, 409 `fleet.aside_not_answered`; one told already, 409 `fleet.aside_told` |

> **Rule.** An aside never reaches the Job on its own.
> Why: the point of a side question is that the Job's work is not interrupted or steered by it, and an answer the agent read unasked would be a message it had not been sent.
