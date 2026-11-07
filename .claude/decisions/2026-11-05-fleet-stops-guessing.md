# Fleet stops acting on guesses

**Decided 2026-11-05.** Fleet sent Jobs 6,634 messages from 8 Oct to 5 Nov and Jobs
disputed 473 of them. The notices that read an agent's words, or guessed what a
fact meant, drew about 70% of the disputes. He ruled that Fleet stops acting on
guesses. Facts stay: a pull request merged, a build, a slot, a wake, a press.

His rulings, by group of the notices page:

| Group | He ruled |
|---|---|
| Arguing over a reply | Delete: telling an agent what its reply meant, or that it stopped |
| His items on a guess | Delete the guess in each: restart settles, removing or folding Needs you items, closing issues a pull request never worked. A fact that settles an item, such as a merge, may stay |
| Workflow steps | *"Delete now, we can reintroduce if needed and it fits with the rework"*: placing a Job on a step from its acts, telling it what it skipped, the drift list on the workflow page, and step instructions |
| Task list corrections | Delete. Keep one fact: a commit that names a task counts as proof, and Fleet marks the task done, telling the agent nothing |
| Refusing by wording | Stop judging how a Needs you item or `all_done` reads. The question check keeps one rule: refuse a question when the agent wrote nothing at all |
| Usage advice | Delete |
| Facts with a claim attached | Keep each fact, take out the blame or instruction |
| The Operator's next-step notice | Delete |

**Changed, not deleted.**

- Stop nudge: it never fires while anything waits on him, a question or a Needs you
  item. His words: *"if there is a needs me item, thats not silence. Thats waiting on
  me to act."* Otherwise it stays, until a live Job that stops heals by itself.
- Context size: one nudge, only at a quiet moment, never mid-decision.
- Merge waits: an item that is a merge wait is refused when the agent writes it, so
  the agent rewords it. Fleet never removes an item already in his Needs you.

**Left alone.** Facts, plain refusals of unsafe acts, and the push-before-approval
refusal, which is built later once size is settled.

**Decisions this retires.**

- The October skip nudges: *nudge once, nothing blocks*
  (`2026-10-15-sessions-absorb-jobs.md:18`, `2026-10-27-a-feature-job-walks-before-it-tests.md:11`).
  Fleet no longer tells an agent it skipped a step.
- The 22 Oct item clarity check. An unclear item reaches his list.
- The 27 Oct task-mark proof from merges and paths, in `2026-10-27-fleet-keeps-the-task-list-honest.md`.
  Only a commit naming the task still marks it.

**6 Nov 2026 replaces the "Refusing by wording" keep.** The question check, which refused a
question when the agent wrote nothing at all, is deleted outright. It refused text he had
already read whenever the stream lagged behind the door. His words: *"I thought we got rid
of those notices about not writing nothing? They clearly failed lol"*. Nothing refuses a
question on what was written before it.
