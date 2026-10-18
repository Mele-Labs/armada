# The judges

**Part of:** [Job](../session.md).

Since 16 Oct 2026. **A judge is a stateless call**, on the Judge model setting (`judge.md`, *A Job's judge*), put about a Job's work.
It is shown the request the person dispatched with (the Job's first message from him), the plan with the state Fleet observed for each task, the repository's standing rules and the diff against the base, and **never the transcript**.
It is advisory: nothing it says blocks the Job.

**Two things run one.** The **tile** (`judge_session`, `POST /sessions/judge`) runs a judge now: it asks whether the work carries out the plan and stays inside the request, and beside that the `judge_checks` of the step the Job is on.
It answers the row at once with `judging` true, and the reading lands on it as `judge` `{at, concerns}`; zero is how Bridge says *No concerns*.
A **boundary judge** runs in the background when the current step moves past a step that has `judge_checks`, with those checks as the questions, once for each step.
Only the step just left is read, not steps passed over.
**Boundary judges are off since 16 Oct 2026** (`BOUNDARY_JUDGES` in `rail_watching.rs`), the owner's decision: most concerns were useless or too technical, and how a judge reads the work is to be reworked before any judge runs on its own.
Only the tile reads.

**A concern is a call in the thread.** Each refusal the judge read becomes a waiting item, id `judge:<n>`, source `judge`, with its text (what was expected, produced and what follows, joined in one string), `concern.from` (the tile, or the step it came from), and `cites`: the files and lines of the diff its quotation landed on.
A concern worded as one still waiting is not raised twice.
**A call that fails or cannot be read raises nothing**, leaves `judge` as it was, and says why as an envelope line on Fleet's stderr.

**A concern is a claim, and its parts are kept apart.** The refusal's `expected` (what should be seen), `produced` (what was seen instead) and `consequence` (what the difference does) are carried on the wire, in `concern`, and in the store (`session_concerns`, three columns, empty where the concern was raised without them), beside `text`.
**`text` is unchanged**: the one string `send_concern` sends the agent is still the joined refusal, so what he sends is what the judge wrote.
A concern kept before the parts existed has none, and the card draws its `text` on its lines.
The card heads with the consequence, then a labelled row each for Expected and Found, each clamped to two lines with a More press for the rest, which reads Less once open.
**The Job's judge is asked to write each of the three as one short sentence** and not to restate the task, the plan or the diff (`BREVITY` in `session_work.rs`).

**The person decides what the agent hears.** `send_concern` sends it to the agent as his message (naming where it came from, the text and what it cites) and settles it; the message goes first, so a send that fails leaves the concern waiting.
`dismiss_concern` settles it and tells the agent nothing.
A settled concern leaves `waiting_for` and stays on the record in `settled`, a bounded window of the last 40, for the thread to draw as one line.
`answer_waiting` and `dismiss_waiting` refuse a `judge` item, and sleep mode leaves it for the morning.

**Held in memory:** `judging`, as a Fleet that restarts reads nothing as reading.
Kept in the store: the rail's last step, the steps nudged and judged, the concerns and the last reading (`session_rail`, `session_concerns`, `session_judge_reading`).
