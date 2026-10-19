# The delivery question

**Part of:** [Job](../session.md).

Since 19 Oct 2026. A bug or feature Job is stopped at its start and asked how its work leaves the branch, unless its first message already says.
**Three answers.** *Auto land* opens a pull request, watches `ci` and merges it.
*Open PR only* stops at the open pull request.
*Keep local* stops at the commit on the Job's branch, and nothing is pushed.

**A step asks, a workflow does not.** The step key is `asks_delivery`, false where absent, and `bug` and `feature` set it on their plan step.
Entering that step is the start: Fleet does not send the step's `instructions` then.
It tells the agent to stop and not to edit, commit or push, and puts the question on the Job's `waiting_for` as a `delivery` item.
The answer sends the choice and the held instructions in one message from Fleet.

**The first message can answer it.** Fleet reads the Job's first message from the person for a short list of phrases: `keep it local`, `do not push` for Keep local; `open a PR`, `PR only` for Open PR only; `land it`, `auto merge`, `merge it` for Auto land.
Exactly one delivery named is the answer, kept with `named` set, and nothing is asked. No phrase, two different deliveries, or a negation such as `do not merge` is not read: the owner is asked.

**The answer is one `delivery` row in the Job's thread**, replaced by id when the owner answers.
The row is the only place it is kept, so a restart keeps the question standing and the answer read.
Fleet acts on it only by telling the agent. The agent delivers by it at `land`, the last step of `bug` and `feature`, whose instructions name what each answer means.
A change the owner would see in Bridge is walked and approved before any of the three runs.
`land` is declared `human_always`, and nothing in Fleet blocks on that gate.
The agent reads the answer from the `land` step's instructions: Auto land merges on a green `ci` without asking, and a change the owner would see still waits for his walk.
