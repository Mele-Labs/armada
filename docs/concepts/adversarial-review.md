# Adversarial review

**What it is:** An advisory critique of a proposal before any work is done on it, made on the owner's press by the strongest model, whose findings sit in the plan they are about and which vetoes nothing.

---

**Kind:** Policy.

The owner, 23 to 25 Oct 2026: "I want an Adversarial Review button on a plan when it's proposed to me.
Build it so we can use adversarial reviews for anything.
We start with plans but we might allow other things to have quick reviews dispatched for them."
He walked three takes and approved the one where findings sit in the plan: "I think we have the opportunity to build a better interface here where it's easy to see these review claims directly in the plan group or task they are associated with."
"Adversarial review" is his name for it, and the lexicon carries it as written (`../contracts/design-system/voice-copy.md`, *Lexicon*).

A plan waits for his approval ([The plan](session/the-plan.md)).
He presses **Adversarial review**, and Fleet asks one model to break it: to find what the plan gets wrong, leaves out or assumes.
What comes back is a handful of findings, each drawn under the part of the plan it is about, and he sends the ones worth the agent's attention.

## What it is, and what it is not

| An adversarial review is | It is not |
| --- | --- |
| A press on a plan waiting for approval | A schedule, a boundary or a polling detector |
| One read-only call on the strongest tier | A job, a reviewer agent or an agent that acts |
| Advisory critique of a proposal | A veto, a vote, a score or an approval |
| Open-ended by design | A narrow yes/no question |
| Kept for the subject and version it read | The Judge |

**The Judge is a different tier and keeps its own rules.** The [Judge](judge.md) verifies work already done: a veto with a narrow yes/no question, on a cheap model, called Judge, always.
An adversarial review criticises a proposal before any work exists, on the strongest model, and is asked to break the plan rather than to answer a question about it.
Neither is the other's reviewer, and a page that says *Judge* of an adversarial review is wrong.

> **Rule.** An adversarial review runs only when a person presses it.
> Why: it spends the strongest model on an open question, and his press decides when that is worth it. Nothing starts it on a schedule or at a boundary.

The press is **Adversarial review** (key R) on a plan waiting for approval, beside Approve and Not yet.

## What the call is shown

| Shown | Never shown |
| --- | --- |
| The request he dispatched with | The agent's own messages |
| His own later messages in the thread | The agent's account of its work |
| The plan by its words: summary, each group's title and gives, each task's words | The agent's transcript |
| The repository's standing rules | Any id of the plan |

**The plan is shown by its words and no ids, so a finding can never name one.** A finding that said *Task W2* would mean nothing to the person reading it.
**The agent's testimony is withheld for the reason the Judge's is** (`judge.md`, *Constitutional constraints*, rule 2): a critic that reads the defendant's account is not independent of it.

## The call

It is one call on the review model setting, `models.review` (`crates/config/settings.toml`, row `settings.review-model`), the strongest tier on the roster unless the machine says otherwise.
It reads the repository Fleet serves, read-only, with the three read tools and no other, as the Judge's call does (`judge.md`, *What it is*): it starts in the checkout Fleet serves and never in a Job's worktree.
It answers findings as JSON.

**An answer that cites a file or quotes words it was never shown is refused.** The review reads as failed, with Fleet's words for why, and no finding from it is kept.
Why: a citation is what makes a finding persuasive, so an invented one is not a stricter reading but an unfalsifiable one.
The check is containment over what the call was shown, as in the Judge (`judge.md`, rule 4).

## A finding

A finding is a claim in three parts, the Judge concern's own (`session/the-judges.md`), with the places it rests on.

| Part | Holds |
| --- | --- |
| Consequence | What the difference does to whoever consumes the work |
| Expected | What the plan should say or do if it is right |
| Found | What it says or does instead |
| Cites | Each a `path` or `path:line` in the repository |
| About | The plan part it concerns, quoted in that part's words |

**The card heads with the consequence**, then a labelled row each for Expected and Found, each clamped to two lines with a More press under the words it opens, which reads Less once open.

**A finding names its part by quoting that task's or group's words, and Fleet pins it there.** It uses the plan's own task matcher, the one that matches the agent's task list to the plan (`session/the-plan.md`, *How far it got is observed*).
Where the quoted words match no task or group, the finding is pinned to the whole plan.

## Where it shows

Each finding is its own tinted card under the task or group it is about, on the plan card and in the plan in full.
The card stands out from the plan around it, so a reader sees at once which lines are the plan and which are a critic's.
A whole-plan finding sits under the summary.
A group holding findings carries a mark, so a folded group still says it has some.

| Key | Does |
| --- | --- |
| Enter | Sends the finding to the agent |
| D | Dismisses it |
| J, K, arrows | Move between findings |

**Send to agent and Dismiss sit in the card's footer.** A sent or dismissed finding folds to one muted line in place, where it was, so the plan does not shift under the reader.
*In Bridge* owns how the plan card draws it (`session/in-bridge.md`, *The plan card*).

## Send and dismiss

**The person decides what the agent hears.** `send_finding` sends the claim to the agent as his message, naming the plan part it is about and the claim, and settles it.
The message goes first, so a send that fails leaves the finding waiting.
`dismiss_finding` settles it and tells the agent nothing.

## Kept per subject and version

A review is kept for the subject and version it read.
**Recording the plan again leaves the review shown, marked out of date** (`Read v<n>`, the version it read), with **Run again**.

## Subjects

A review has a subject.

| Subject | Wire | State |
| --- | --- | --- |
| A plan version | `plan { session_id, version }` | The only subject |
| A pull request | Not yet | Possible later |
| A doc | Not yet | Possible later |

Nothing about a finding, its card or the call is specific to a plan except what it is shown and where it is pinned, which is what a new subject supplies.

**The Reviews tab's agent runs on pull requests stay as they are** (`pull-review.md`, *Agent runs*).
They are keyed by commit and answer a kind of question the reader chose.
Whether they join this engine is open (*Open questions*).

## On the wire

The review rides on the Job's plan as `adversarial`.

| Name | Is |
| --- | --- |
| `ReviewSubject` | What is reviewed: `plan { session_id, version }` |
| `AdversarialReview` | The review for one subject, with its findings and the version it read |
| `AdversarialFinding` | One claim: its three parts, its cites, its `about` and whether it is sent or dismissed |
| `about` | `plan`, `group { id }` or `task { id }`: where Fleet pinned the finding |
| `start_adversarial_review` | Bridge route that runs the call |
| `send_finding` | Bridge route that sends one finding to the agent |
| `dismiss_finding` | Bridge route that settles one finding with no message |

The ids in `about` are Fleet's. The model was never shown them, and a finding's text never carries one.

| Refusal | When |
| --- | --- |
| `fleet.review_subject_unknown` | The subject names a Job or a plan version Fleet does not hold |
| `fleet.review_subject_stale` | The subject is not the plan's current version |
| `fleet.finding_unknown` | The finding is not in the kept review |

## Open questions

- **[adversarial-review-cost-cap]** Does an adversarial review get a cost cap, and what shape is it?
  There is none: the call runs on the strongest model, and what bounds spend is a person's press.
  It is the same question as `judge-cost-cap-per-check` in `judge.md`, which owns what decides it, and is answered with it.
- **[adversarial-review-pull-request-runs]** Do the Reviews tab's agent runs on pull requests join this engine?
  Both are read-only calls on `models.review` whose answers are JSON checked against what the call was shown, so the engine could serve them.
  They differ in what is kept: a run is keyed by repository, pull request, head commit and kind, and a review by subject and version.
