# Talk to the Operator

**What it is:** "I want to hand over some words, and have them reach the right Job": what Fleet does with what you type to the Operator.

---

**Trigger:** You have a thought for work already running, or a goal to break into Jobs, and you would rather say it than open each Job.

**Concepts touched:** [Operator](../concepts/operator.md).

**What is built.**
Fleet's side is real.
The panel and its box are not: the panel is `armada-ehhu.10` and the box is `armada-ehhu.9`.
This journey says what Fleet does, and leaves the look to those two.

## Flow

Type words into the Operator's box → Fleet reads them against every live Job in every repository → a clear match is sent to that Job as a message → otherwise the words become a Job started on your words.

| You type | What Fleet does |
|---|---|
| A thought for a Job already running, such as "also cover the jobs table" | The shortlist finds the Job by its title, and Delegate sends the words with `tell_session`. The Job's thread shows a row of where they came from |
| A goal, such as "split this feature into Jobs" | The Operator starts a Job on your words, which breaks it into Jobs |
| A workflow to author | A Job on your words works it out with you and ends with the definition saved, in this repository or in Kit |
| A Studio to drive | A Job on your words drives it |

The Operator holds no tools and runs no Job of its own, so none of this holds a slot until a Job on your words first writes.

## What stays yours

**A question a Job asks stays in Needs you.**
The Operator may add its read and the option it would pick, and your press on that option is the one press.
Where its Answer role is On, it settles the ones Fleet's rules allow, and the Job's thread says what was answered and why.
**Nothing it rules is hidden.**
Every ruling is a row of its Log, whether or not Fleet acted on it.

**What you typed yourself is never filed as an issue.**
It is started, delegated or held.

## What is already decided and landed

- **Words reach the right repository whichever one is open.**
  The work a thought belongs to is often in another repository, so the read covers every repository's live Jobs.
  One Operator serves the machine, so there is no repository to point it at.

## Related

[Operator](../concepts/operator.md) is the concept page: what reaches it, the roles, Fail open, the Log and what a Job on your words is.
