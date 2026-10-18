# Sleep mode

**Part of:** [Job](../session.md).

The owner turns on Sleep from the menu in Bridge's title row before bed (`set_sleep`), and the menu button carries the moon while it is on.
Jobs keep going until only he can unblock them.
`get_sleep` reads the switch and the night's rows; turning it on starts a new night and clears the last one's rows.

While it is on, a pass over every live job's waiting items (`crates/fleet/src/sleeping.rs`, riding the pull-request notice tick) works like this:

| Item | What the night does |
|---|---|
| `ask:` or agent item with a "(Recommended)" option, waiting under two minutes | Nothing. The agent carries on by itself first. |
| `ask:` or agent item with an option labelled "(Recommended)", past two minutes | Answers with that option. Recorded under decided, with the option as what it chose and the job it came from. |
| `ask:` or agent item with no such option: a question with other options, an open question, an errand with an act | Nothing, unless it reads as destructive (next row). It stays in the job's Needs you panel for the owner, and is not recorded under decided or blocked. |
| Destructive or irreversible (delete, drop, remove, force, reset, rm, overwrite, discard, close PR and the like, read from the text and the options; unsure holds) | Never answered. Recorded under blocked. |
| `perm:` permission | Never allowed. Recorded under blocked. |
| `walk:` | Recorded under walks. |
| A pull request the pull watch sees merge | Recorded under landed. |

**The review is cards on the stage, opened with the call key**, not a panel of its own: each decided row is a call, headed by who decided it.
The card says what Sleep chose and keeps its acts on the rail: **Agree** keeps it (`review_sleep`), **Change** sends the owner's words to the agent (`override_sleep`) and reviews the row, **Open the Job** goes to where it was decided, **Later** puts it at the back, and **Dismiss**, under a cover, reviews it for good.
A decision made in a Job ends in that Job's own box, which is how it is changed, and `session_id` on the wire row is the Job it opens.
A correction Fleet refused leaves the row where it is.
Either act marks the row reviewed, and `get_sleep` leaves a reviewed row out of `decided`.
**The first use of Bridge after Sleep was turned on turns it off** (`set_sleep` off).
Blocked and walk rows are already calls: waiting items.

`override_sleep` sends the job `Re: <asked>` and the owner's words, marks the row corrected and reviewed. Every change publishes `sleep.changed` with the night whole.
