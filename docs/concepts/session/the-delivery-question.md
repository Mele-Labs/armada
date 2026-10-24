# The delivery question

**Part of:** [Job](../session.md).

Since 19 Oct 2026. A bug or feature Job is asked how its work leaves the branch before it edits anything, unless its first message already says.
**Three answers.** *Auto land* opens a pull request, watches `ci` and merges it.
*Open PR only* stops at the open pull request.
*Keep local* stops at the commit on the Job's branch, and nothing is pushed.

**The start card is the first place it is answered.**
A Job dispatched from Bridge opens on [the start card](the-start-card.md), whose Lands row holds the answer, filled with a landing the request names or else his usual for that kind of work in that repository.
Start keeps the row as the answer, so the plan step asks nothing and the question never reaches the thread.
The usual, the picks it is read from and the Jobs that get a card are in that page.

**A Job with no card is asked at its plan step.**
An agent's start, a fork, a start with `root` and a start with `after` have no card, and the question below stands for them.

**A step asks, a workflow does not.** The step key is `asks_delivery`, false where absent, and `bug` and `feature` set it on their plan step.
Entering that step is the start: Fleet does not send the step's `instructions` then.
It tells the agent to stop and not to edit, commit or push, and puts the question on the Job's `waiting_for` as a `delivery` item.
It tells the agent the question is asked already, so the agent never puts a second copy on `waiting_for` or asks it with AskUserQuestion.
The answer sends the choice and the held instructions in one message from Fleet.
A Job whose card answered it is sent the choice with the step's instructions in its first turn, and the step asks nothing.

**The first message can answer it.** Fleet reads the Job's first message from the person for a short list of phrases: `keep it local`, `do not push` for Keep local; `open a PR`, `PR only` for Open PR only; `land it`, `auto merge`, `merge it` for Auto land.
Exactly one delivery named is the answer, kept with `named` set, and nothing is asked.
No phrase, two different deliveries, or a negation such as `do not merge` is not read: the owner is asked.

**The answer is one `delivery` row in the Job's thread**, replaced by id when the owner answers.
The row is the only place it is kept, so a restart keeps the question standing and the answer read.
Auto land, answered or named, turns the Job's Auto land flag on, the same flag `set_session_auto_land` sets, so the forge's auto-merge follows it. Open PR only and Keep local leave the flag as it is.
A forge refusal does not undo the answer: the forge's words go to the agent.
**Auto land merges a pull request however it was attached (#2679, 21 Oct 2026).** One the agent opened itself is reported by its mod, and nothing asked the forge then.
So the pull reading asks for auto-merge on each pull request a Job with Auto land holds, once its checks pass, it conflicts with nothing and nothing merges it.
It asks once a commit: a commit it saw queued or on auto-merge is not asked again, so a merge somebody stopped stays stopped.
Fleet otherwise acts on it only by telling the agent. The agent delivers by it at `land`, the last step of `bug` and `feature`, whose instructions name what each answer means.
A change the owner would see in Bridge is walked and approved before any of the three runs.
`land` is declared `human_always`, and nothing in Fleet blocks on that gate.
The agent reads the answer from the `land` step's instructions: Auto land merges on a green `ci` without asking, and a change the owner would see still waits for his walk.
