# Talk to the Operator

**What it is:** "One decision at a time, and a place to talk about it": how you settle what the Operator puts in front of you, and how you talk to it.

---

**Trigger:** The Operator has put up a card, or you have a thought for work already running and would rather say it than open each Job.

**Concepts touched:** [Operator](../concepts/operator.md).

## Flow

In the native Bridge the Operator's page is the right side of the Cockpit, with the Jobs list on the left.

A card is in front → you press an option, or press Discuss and talk → the card settles or the Operator puts up a new one → the next card in Next up comes forward.

| You do | What you see |
|---|---|
| Read the card in front | One decision: a title, its read, two to four options with the Operator's pick first and a cost on each |
| Press an option, or keys 1 to 4 | The Operator is told and settles the card. The next card comes forward |
| Press Discuss (`/`) | The composer under the card takes focus, with a tag naming the card. The card stays in sight |
| Send in the composer | The card folds to one line and the talk runs below it |
| Keep talking | The Operator answers in the talk, may revise the card, and ends by settling it or putting up a new card |
| Press Snooze (`S`) | The card goes back until its time and is hidden until then |
| Press a Job in the list at the left | The Job takes the whole right side. Esc, the Cockpit tab pressed again, or the same row pressed again brings the Operator's page back |
| Press ⌘J | The caret goes to the Operator's box. With a Job open it stays in the Job's chat |
| Open Next up on the right | The cards queued behind, then the ideas the Operator kept. Pressing one brings it to the front |
| Read the feed on the right | What happened since you last looked: what the Operator dispatched, merges, Jobs ended, and what Jobs asked or told it |

**A question or a thought goes in the composer too.**
With no tag, a send is a talk to the Operator about no card, and it answers in the talk.
A screenshot or a file goes with the words: attach it, paste it or drop it on the box, with or without a tag.
Nothing marks what kind of thing it is.
The Operator reads your words: it starts a Job, folds the work into one already running, asks you on a card, or keeps it as an idea.

**Your words go to one project's Operator.**
Fleet picks it from the card you tagged, the screen you are on, the Job you have open, and what your words name.
Where none is sure, Fleet puts up a card, "Which project is this for?", with up to four projects.
Your pick sends the words you wrote to that project's Operator and no other.

**With no session running, nothing hears you.**
The composer and a press on a card are refused with `fleet.operator_session_not_running`, and the card stands. A dispatch starts as you asked.

**A question is answered by reading.**
Ask what a Job is waiting on, what an issue says, or where the code does something, and the Operator reads the Job, the issue or the code of its project before it answers.
A Job, a pull request or a repository of another project is held, and Fleet asks you on a card whether to let it through, hand it to that project's Operator or refuse it.

## What the Operator does without asking

**It depends on the project.**
Each project has one of three values.

| Value | What it does unasked |
|---|---|
| `act_then_tell` | Acts, then says so in the feed |
| `ask_before_starting` | Puts up a card before it starts any Job |
| `only_when_told` | Nothing |

**A project with no value asks once.**
A card offers the three, and your answer is kept as that project's value.
Armada starts at `act_then_tell`.

**A change you would see, or a decision that is yours, is always a card.**
No value lowers this.

**Each project also sets what its Operator may use.**
Starting Jobs, merging pull requests, ending Jobs and filing issues are each on or off, and Fleet refuses what is off.
A project may ask its decisions in the talk instead of on cards.
[Operator](../concepts/operator.md), *What he sets for each project*, lists every setting.

## What stays yours

**A question a Job asks stays in that Job's own Needs you.**
The Operator is not handed it, so only you answer it, or Autopilot while you are away.

**A snoozed Job raises nothing until it wakes.**

**What the Operator writes to a forge is landing a pull request.**
It never comments on, labels or edits an issue or a pull request for you.

## What is already decided

- **Words reach the right project's Operator whichever screen is open.**
  Each project has its own Operator, and Fleet picks the one your words are for from the card, the screen, the Job you have open and what the words name.

## Related

[Operator](../concepts/operator.md) is the concept page: what it is, what it may do, what reaches it and what went.
[The Operator's page, kept by Fleet](../practices/protocol/the-operators-page-fleet-keeps.md) is the wire.
