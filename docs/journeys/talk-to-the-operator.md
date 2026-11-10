# Talk to the Operator

**What it is:** "I want to hand over some words, and have them reach the right Job": what Fleet does with what you type to the Operator.

---

**Trigger:** You have a thought for work already running, or a goal to break into Jobs, and you would rather say it than open each Job.

**Concepts touched:** [Operator](../concepts/operator.md).

**What is built.**
Fleet's side is real.
The goal is one chat, on its own page or in a pane from anywhere, in the native Bridge, and it is not built.
The Electron Bridge no longer shows the Operator.
This journey says what Fleet does with the words, and leaves the look to [Operator](../concepts/operator.md).

## Flow

Type words into the Operator's chat → Fleet reads them against every live Job in every repository → a clear match is sent to that Job as a message → otherwise the words become a Job started on your words.

| You type | What Fleet does |
|---|---|
| A thought for a Job already running, such as "also cover the jobs table" | The shortlist finds the Job by its title, and Delegate sends the words with `tell_session`. The Job's thread shows a row of where they came from |
| A goal, such as "split this feature into Jobs" | The Operator starts a Job on your words, which breaks it into Jobs |
| A workflow to author | A Job on your words works it out with you and ends with the definition saved, in this repository or in Kit |
| A Studio to drive | A Job on your words drives it |

The Operator runs no Job of its own, so none of this holds a slot until a Job on your words first writes.

**A question is answered by reading.**
Ask what a Job is waiting on, what an issue says, or where the code does something, and the Operator reads before it answers: the Job's thread and plan, the open issues, and the code of the repositories Fleet hosts.
It reads and changes nothing, since the only tools it holds are Read, Grep and Glob.
Where you want something changed, it offers a Job to make the change.
Why: on 9 Nov you found it could not answer what you asked without reading.

## What stays yours

**A question a Job asks stays in Needs you.**
The Operator is never handed it, so only you answer it, or Autopilot while you are away.
**Nothing it rules is hidden.**
Every ruling is kept, whether or not Fleet acted on it.

**What you typed yourself is never filed as an issue.**
It is started, delegated or held.

## What is already decided and landed

- **Words reach the right repository whichever one is open.**
  The work a thought belongs to is often in another repository, so the read covers every repository's live Jobs.
  One Operator serves the machine, so there is no repository to point it at.

## Related

[Operator](../concepts/operator.md) is the concept page: the goal, what is built, what reaches it, Fail open and what a Job on your words is.
