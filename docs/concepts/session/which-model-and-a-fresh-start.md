# Which model, and a fresh start

**Part of:** [Job](../session.md).

**A Job's composer names the model it runs on.** Fleet launches `claude` with `--model` only where a person picked one, and the CLI's own default applies otherwise, so the pick alone says nothing.
Fleet reads the model off each `init` line of the stream (`DroneEvent::Started`, one per turn) and carries it as `HostedFacts.resolved_model` (23.85): `claude-opus-5-5`, which Bridge names `Opus 5.5`.
**It is cleared when the process is let go** and absent before the first turn, because the next process has not said.
The composer's Model select reads it: the selected option, the picked alias or the default, is labelled `Opus 5.5` once the resolved model is of that option's family.
Until it has, or after another model is picked, the select reads the alias or the default as it did, **never a guess**.
The head draws no model.
A terminal Job is named only where its mod reported a model.

**Past 300k tokens of context or a day since it started, a quiet mark (`move-right`) sits after the context tank.** Hover or focus opens its card and a press keeps it open, as the context tank's does: a fact `Context` of `612k tokens`, `Started 3h ago` in its head, and two acts.
**Fork** forks the conversation at its latest row (`start_session` with `fork.at`, 23.79).
**New Job** asks the old Job for its handoff, which is a message of the person's sent with `send_session_message`: the goal, where it stands, open pull requests and branches, and what is next.
Its next reply, once the turn is over, is the handoff, and the new Job opens under the same title with `Handoff from <title>` and that reply as its first message, sent for the person.
While the old Job writes it the mark pulses and New Job waits.
**If it ended, the send was refused, or no reply came within 90 seconds**, the first message is the ledger's facts instead: the title, its slot, its branches and its open pull requests.
Nothing is typed.
Both are Bridge's alone (`session-handoff.ts`), over operations that already exist; the cost is one turn of the old Job, which re-reads its whole context once.
