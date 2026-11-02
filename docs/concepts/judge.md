# Judge

**What it is:** One reading of a Job's work on the owner's press, by the strongest model, read-only with the checkout.
It looks for what the person or the agent missed, or checks the line the person gave it, and its notes sit inside the artifact on the part they name.
It advises and gates nothing.

---

**Kind:** Policy.

Formalizes the Judge, the semantic tier of verification.
The Judge is referenced on nearly every concept document, and this document is the home of its constitutional rules.

The owner, 23 to 25 Oct 2026, asked for a review of a plan "when it's proposed to me", built "so we can use adversarial reviews for anything", with the findings "directly in the plan group or task they are associated with".
There is one Judge.
It replaces the Session Judge, whose concerns were calls in Needs you, and the plan's adversarial review button.

## What it is

The Judge is a read-only call that [Fleet](fleet.md) makes about one artifact of a Job, or about the Job as a whole, when a person presses it.
It answers the question the mechanical tier structurally cannot: not *did something happen*, but *was it the right thing, and what was missed*.

It is not an actor. It has no job, no memory between calls, no tool that writes, and no ability to transition anything.
Fleet makes the call, reads what it says, and keeps the notes.

| The Judge is | The Judge is not |
| --- | --- |
| One read-only call on the strongest model | A job, a reviewer agent, or an agent that acts |
| Run when a person presses it | A schedule, a boundary or a polling detector |
| Advice, kept as notes inside the artifact | A veto, a vote, a score or an approval |
| Open to a line the person types, or to what was missed | A narrow yes/no question |
| Called Judge, always | The auditor, the reviewer, adversarial review or AI review |

**"Auditor" is retired vocabulary, and so is "adversarial review" for this feature.** See `../contracts/design-system.md` lexicon.

## Why it exists

The core principle from [Workflow](workflow.md): self-report is a signal, not a source of truth. Without the Judge there are two tiers and a gap between them.

| Tier | Answers | Cost | Fails at |
| --- | --- | --- | --- |
| Mechanical Check | Did something happen: diff, exit code, artifact | Free, deterministic | Intent: a weakened assertion still exits 0 |
| Human review | Was it the right thing | Your attention | Volume: it does not scale to a queue |

Evidence gaming is the failure mode a Mechanical Check passes **by design**. Human attention spent on routine work is unavailable for the calls that need it.
The Judge occupies the middle: a second reader that costs a press, with semantic reach.

## Pressing it

A Judge act, a scale glyph, stands on every artifact in a Job's ledger and in the head of the Thread tab, where it judges the Job as a whole.

| Artifact | Where the act stands |
| --- | --- |
| Plan, pull request, commit, file | Its ledger row |
| Walk page, sketch | Its ledger row |
| Document | Its ledger row and the head of its page |
| Command, subagent, issue, note | Its ledger row |
| The Job as a whole | The head of the Thread tab |

On an artifact, the press opens a small box, **Verify that**, for an optional line naming what to check.
Left empty, the Judge looks for **what the person or the agent missed**.

> **Rule.** The Judge runs only when a person presses it.
> Why: it spends the strongest model on an open question, and his press decides when that is worth it. Nothing starts it on a schedule or at a boundary.

The act on a row reads the state of the last reading.

| The act shows | Means |
| --- | --- |
| A mark that moves | A reading is running |
| A dot | Notes are unread |
| A check | The last reading was clean |

No count is drawn beside the act or the notes.
A finished Judge puts a line in the thread saying so, and a press on it opens the artifact at its notes.
The Judge of the whole Job has no artifact to stand in, so its press opens a Judge tab, which asks in a composer of its own and holds the reading ([The Judge](session/the-judge.md)).
**A finished Judge raises no Needs you call**, since it asks nothing of the person.

## What the call is shown

| Shown | Never shown |
| --- | --- |
| The request the person dispatched with | The agent's own messages |
| The artifact | The agent's account of its work |
| The Judge's prompt, with the person's line where one was typed | The agent's transcript |
| The repository, read-only | Any id Fleet keeps for the artifact |

**The artifact is shown by its words and no ids, so a note can never name one.** A note that said *Task W2* would mean nothing to the person reading it.
**The agent's testimony is withheld, because a critic that reads the defendant's account is not independent of it.**

**It may read the repository, and only the repository.** The call reads the checkout Fleet serves, read-only, for a few turns, so it learns what the repository requires of a change the way an agent does: from CLAUDE.md, the docs and the skills.
The fence is the argument list (`crates/adapters/src/judge.rs`): the three read tools and no other, every tool that writes, runs or reaches the network denied, reads held inside the checkout, and Fleet's own `.armada/` refused.
It starts in the checkout Fleet serves and never in a Job's worktree, so an agent cannot rewrite what its Judge reads.

It runs on the review model setting, `models.review` (`crates/config/settings.toml`, row `settings.review-model`), the strongest tier on the roster unless the machine says otherwise.
**An answer that cites a file or quotes words it was never shown is refused.** The reading fails with Fleet's words for why, and no note from it is kept.
Why: a citation is what makes a note persuasive, so an invented one is not a stricter reading but an unfalsifiable one.

## A note

A note is a claim headed **You should know**, with what it rests on.
It sits inside the artifact, on the part it names.

| Artifact | A note sits on |
| --- | --- |
| A plan | The group or task it is about, or under the summary where it is about the whole plan |
| A pull request or a commit | The diff line it names |
| A document | The paragraph, heading, list item or table row it names |
| Any other | The artifact's own view |

A note stands out from the artifact around it, so a reader sees at once which lines are the artifact and which are the Judge's.
**A note names its part by quoting that part's words, and Fleet pins it there.** For a plan Fleet uses the plan's own task matcher, the one that matches the agent's task list to the plan ([The plan](session/the-plan.md), *How far it got is observed*).
A document's part is named by those words, never by a count, because the page Bridge draws is not divided the way Fleet divides its text.
Where the quoted words match no part, the note sits on the artifact as a whole.

Each note carries **Send to Job** and **Dismiss**.
A sent or dismissed note folds to one muted line where it was, so the artifact does not shift under the reader.
[The Judge](session/the-judge.md) owns how a Job draws the reading, and the keys are in `../contracts/design-system/keyboard-and-command-palette.md`.

> **Rule.** The person decides what the agent hears.
> Why: Send to Job sends the claim as his message, naming the part it is about, and the message goes first, so a send that fails leaves the note waiting.
> Dismiss settles it and tells the agent nothing.

A reading of a plan is kept for the plan version it read.
Recording the plan again leaves the reading shown, marked out of date, and the act reads it again on a press.

## Step judges

Fleet's own step judges are a different thing and keep their own rules.
A step judge is a stateless call on the Judge model setting, `models.judge` (cheap by default), that [Fleet](fleet.md) makes against a workflow's `judge_checks` and gates nothing.
It answers narrow yes/no questions about the work, and everything below this heading is about step judges.

| A step judge is | A step judge is not |
| --- | --- |
| A single model call with a narrow question, which may read the repository first | A job, a reviewer agent, or an agent that acts |
| A veto | A vote, a score, or an approval |

Measured in v1 at roughly 4k input and 300 output tokens per judgment, under a cent on Haiku.

### Constitutional constraints

**Loosen any one of these and a step judge becomes a second opinion drawn from the same failure distribution as the agent it is checking.**

**1. Veto-only, never a vote.** A step judge may only refuse; it can never grant anything.
Why: there is no such thing as a Judge pass, only work the Judge raised nothing against, and a model that can grant is a model that can be talked into granting.

**2. Blind to the agent, and shown the request.** A step judge never sees the agent's transcript or self-report, and receives only the original request the person dispatched with, the plan, the diff against the base, and what the repository requires of every change where its Manifest names a file ([Manifest](manifest.md), *What every change here carries*).
Why: a verifier that reads the defendant's testimony is not independent, and one that cannot see what was asked for can only check whether a document agrees with itself, which passes a well-written answer to the wrong question.
Enforced at prompt assembly (`../contracts/agent-prompt.md` section 7). `verification::Work` has no field for a transcript, and the request is not optional.

**3. Narrow question.** Every question is yes/no, never "review this".
Why: open-ended prompts produce agreeable prose, and a narrow question has a wrong answer.

**4. A refusal must cite.** A refusal names the specific evidence it is refusing on.
Why: that citation is the artifact the agent hears and the person reads, so an uncited refusal is unactionable for both audiences.

**And the citation is checked against what the call was shown.** A refusal that puts words in quotation marks is claiming those words appear in the material above it; where they appear in none of it, the call failed and no verdict was reached, the same shape as an answer in prose or an empty one, never a refusal and never a pass.
Why: an invented citation is not a stricter reading but an unfalsifiable one, and a criterion of the form *does this do anything the scope note did not describe* is only as good as the Judge's reading of the note.
The check is containment over the brief and costs no second call; it is deliberately blind to quotations under four words, which are terms rather than claims about wording.

**5. Cold by default.** A step judge does not run on a schedule.
Why: a detector that polls is the wrong design, since the call costs money.

> How a v1 path citation resolves, and why it currently needs a commit hash, is on `../contracts/design-system.md`, Typography of reference.

### What a step judge reads

A brief laid in labelled parts so a citation says where it landed, an answer quoting words not in the brief discarded, a refusal that must say what was expected, produced and what follows (`verification::Brief::about_work`).
**What it is shown is `Work`'s fields and nothing else**: the request the person dispatched with, the plan, the repository's standing rules and the diff against the base, cut at 120,000 characters with a note saying so.
There is no parameter for the transcript, so it cannot read the agent's account of its own work.

**One call for each question.** A step's `judge_checks` give the questions at a boundary. A panel size and a gaming check are ignored here.
**No step judge runs at a step boundary.** How a step judge reads the work is reworked before any runs on its own again.

### Multi-criterion judging

One narrow question per step forces every acceptance condition into a single yes/no, which either makes the question broad (violating rule 3) or silently drops conditions.
A refusal also loses resolution: "the fix does not address the root cause" cannot say *which* of four requirements went unmet.

**`judge_check.criteria[]` replaces `question`.** Each criterion is dispatched as its own narrow yes/no call.
Why: this preserves rule 3 rather than bending it, since the question gets narrower, not broader.
Each criterion that draws a refusal is its own refusal with its own citation, so a person can act on one and set another aside.

```json
"judge_check": {
  "enabled": true,
  "model": "haiku",
  "criteria": [
    {
      "criterion_id": "c1",
      "question": "Do expired tokens refresh once rather than per request?"
    },
    {
      "criterion_id": "c2",
      "question": "Does a failed refresh sign the session out?"
    }
  ]
}
```

### What a refusal holds

A refusal's content is three named fields, never a formatted string:

| Field | What it holds |
| --- | --- |
| `expected` | What should be seen, returned or recorded if the work is right: the value itself |
| `produced` | What will be seen instead |
| `consequence` | What that difference does to whoever consumes it |

`consequence` is the field a person triages on.

Summary prose is prompt plus mechanical lint, governed by `../contracts/agent-copy.md`.
**Substance requirement: every Judge summary must cite a file, line or assertion that could not appear in any other Job's summary.** A summary that reads plausibly under a different Job has failed.

**A refusal's citation is placed, never quoted back.** The words are already in `expected` and `produced`; what a reader lacks is where in the brief they came from.
The labels are recorded as the brief is assembled rather than recognised in it afterwards, so a citation names the part the assembler wrote and not a heading a reader matched.
A refusal that argues in the Judge's own words cites nothing and is a complete refusal under rule 4.

### Configuration

| Field | Purpose |
| --- | --- |
| `enabled` | Per step. Some steps have no semantic question worth asking |
| `model` | Per-step model dial. The rigor-scaling mechanism |
| `criteria[]` | Per-criterion narrow questions, see above |
| `prompt_key` | Reference into the compiled-in prompt set |

**`model` is a deliberate per-step dial, not fixed fleet-wide.** Routine steps use a cheap, fast model for cost and latency; a stronger one is for the step that earns it.
**The fleet-wide default is the cheapest model on the roster**, carried by the Judge model config row.
It is deliberately the floor rather than the middle: the dial is only legible against a default it moves away from, and Bug raising `review` to a stronger model says nothing if every step started there.
A stronger default would not let a step judge see more, since it reads the same checkout under the same turn cap whatever the model, and would only make every criterion cost more and take longer.
The turn cap is the `judge-read-turns` setting.

**Judge prompts are compiled into the binary and are not user-editable.** No Kit library, no Manifest override, so prompt version is the binary version and no prompt hash is recorded on the verdict.
Where the prompt library lives, and whether a prompt version is recorded on the verdict, is tracked in `../contracts/agent-prompt.md`; what a Judge is and is not told is specified there, sections 2 and 7.

**The Judge cost cap per check row still reads `undecided`**, and was reviewed and left open rather than guessed at (see Open questions).
The **Judge confidence threshold** row is superseded and awaiting disposal (see Open questions).

## Dispatch and approval

**A Judge call is not a dispatch.** It is a call with a result: no worktree, no branch, no rail, no Job of its own.
It needs no approval gate, since the person's press is what starts it.

## Open questions

- **[judge-cost-cap-per-check]** Does a Judge call get a cost readout, and is a per-check dollar cap the right shape for the sub-limit?
  Reviewed 28 Aug 2026 alongside the model default and deliberately left open: nothing can read what a step judge call cost, because the call is rendered as one-shot text and emits no result envelope, so the figure a cap would compare against never reaches Armada.
  A number written into the row today would be enforced by nothing while reading as a limit that was set, which is worse than an absent cap because only the absent one is visible as missing.
  The Judge on the owner's press has no cap either: it runs on the strongest model, and what bounds spend is a person's press. The same answer decides both.
- **[judge-confidence-threshold-disposal]** Is the Judge confidence threshold config row deleted or repurposed? It is superseded and currently awaiting disposal.
- **[judge-pull-request-runs]** Do the Reviews tab's agent runs on pull requests join the Judge?
  Both are read-only calls on `models.review` whose answers are checked against what the call was shown, so one engine could serve them.
  They differ in what is kept: a run is keyed by repository, pull request, head commit and kind the reader chose, and a reading by artifact.
  [Pull review](pull-review.md), *Agent runs*, owns the runs as they are.
