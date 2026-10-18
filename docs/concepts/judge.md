# Judge

**What it is:** The semantic tier of verification — a stateless cheap-model call about a Job's work, veto-only, blind to the agent's transcript and run on the owner's press. Judges whether the work satisfies the request and the plan, never whether something happened. Model is a dial, cheap by default.

---

**Kind:** Policy.

Formalizes the Judge — the semantic tier of verification. The Judge is referenced on nearly every concept document, and this document is the home of its constitutional rules.

## What it is

The Judge is a stateless, cheap-model call that [Fleet](fleet.md) makes about a Job's work. It answers one question the mechanical tier structurally cannot: not *did something happen*, but *was it the right thing*.

It is not an actor. It has no job, no memory between calls, no tool that writes, and no ability to transition anything. Fleet makes the call, reads what it says, and writes the result.

**It may read the repository, and only the repository.** Decided 2 Oct 2026: a call can read the repository's own checkout, read-only, for a few turns, so it learns what the repository requires of a change the way an agent does — from CLAUDE.md, the docs and the skills. A Judge that could read only its brief refused a plan task fixing the docs a change made wrong as *scope expansion*, while the agent had planned it because the repository asked for it. The fence is the argument list (`crates/adapters/src/judge.rs`): the three read tools and no other, every tool that writes, runs or reaches the network denied, reads held inside the checkout, and Fleet's own `.armada/` refused. It starts in the checkout Fleet serves and never a Job's worktree, so an agent cannot rewrite what its Judge reads. The turn cap is the `judge-read-turns` setting. What it costs is latency and money on every call, accepted when it was decided.

| The Judge is | The Judge is not |
| --- | --- |
| A single model call with a narrow question, which may read the repository first | A job, a reviewer agent, or an agent that acts |
| The second of two verification tiers | A replacement for human review |
| A veto | A vote, a score, or an approval |
| Run when a person asks | A scheduled or polling detector |
| Called Judge, always | The auditor, the reviewer, or AI review |

**"Auditor" is retired vocabulary.** See `../contracts/design-system.md` lexicon.

## Why it exists

The core principle from [Workflow](workflow.md): self-report is a signal, not a source of truth. Without the Judge there are two tiers and a gap between them.

| Tier | Answers | Cost | Fails at |
| --- | --- | --- | --- |
| Mechanical Check | Did something happen — diff, exit code, artifact | Free, deterministic | Intent — a weakened assertion still exits 0 |
| Human review | Was it the right thing | Your attention | Volume — it does not scale to a queue |

Evidence gaming is the failure mode a Mechanical Check passes **by design**. Human attention spent on routine work is unavailable for the calls that need it.

The Judge occupies the middle: machine cost, semantic reach. Measured in v1 at roughly 4k input / 300 output tokens per judgment — under a cent on Haiku.

**Rejected: spawning a full agent job for review.** That gives the reviewer a job, tools and the ability to act, all of which are liabilities in a verifier, at many times the cost.

## Constitutional constraints

**Loosen any one of these and the Judge becomes a second opinion drawn from the same failure distribution as the agent it is checking.**

**1. Veto-only, never a vote.** The Judge may only refuse; it can never grant anything.
Why: there is no such thing as a Judge pass — only work the Judge raised nothing against, and a model that can grant is a model that can be talked into granting.

**2. Blind to the agent, and shown the request.** The Judge never sees the agent's transcript or self-report, and receives only the original request the person dispatched with, the plan, the diff against the base, and what the repository requires of every change where its Manifest names a file — [Manifest](manifest.md), *What every change here carries*.
Why: a verifier that reads the defendant's testimony is not independent, so the agent's own account is removed as a contamination source — and a verifier that cannot see what was asked for can only check whether a document agrees with itself, which passes a well-written answer to the wrong question.
Enforced at prompt assembly — `../contracts/agent-prompt.md` section 7. `verification::Work` has no field for a transcript, and the request is not optional.

**3. Narrow question.** Every question is yes/no, never "review this".
Why: open-ended prompts produce agreeable prose, and a narrow question has a wrong answer.

**4. A refusal must cite.** A refusal names the specific evidence it is refusing on.
Why: that citation is the artifact the agent hears and the person reads — no separate summary exists, so an uncited refusal is unactionable for both audiences.

**And the citation is checked against what the call was shown.** A refusal that puts words in quotation marks is claiming those words appear in the material above it; where they appear in none of it, the call failed and no verdict was reached — same shape as an answer in prose or an empty one, never a refusal and never a pass. Why: a citation is what makes a refusal persuasive, so an invented one is not a stricter reading but an unfalsifiable one, and a criterion of the form *does this do anything the scope note did not describe* is only as good as the Judge's reading of the note. The check is containment over the brief and costs no second call; it is deliberately blind to quotations under four words, which are terms rather than claims about wording.

**5. Cold by default.** The Judge runs on a person's press, never on a schedule.
Why: a detector that polls is the wrong design — the Judge costs money, and a person's press decides when it is worth spending.

> How a v1 path citation resolves — and why it currently needs a commit hash — is on `../contracts/design-system.md`, Typography of reference.

## A Job's judge

Since 16 Oct 2026. A Job has no step to gate, and the Judge's call reads its work. **A brief laid in labelled parts so a citation says where it landed, an answer quoting words not in the brief discarded, a refusal that must say what was expected, produced and what follows** (`verification::Brief::about_work`).

**What it is shown is `Work`'s fields and nothing else**: the request the person dispatched with, the plan, the repository's standing rules and the diff against the base, cut at 120,000 characters with a note saying so. There is no parameter for the transcript, so it cannot read the agent's account of its own work. It reads no checkout.

**One call for each question.** A step's `judge_checks` give the questions at a boundary, and the tile adds a standing one, whether the diff carries out the plan and stays inside the request. A panel size and a gaming check are ignored here. What it raises is a concern the person sends or dismisses (`session.md`, *The judges*); it gates nothing.

**No judge runs at a step boundary.** The tile runs one when the person presses it. How a judge reads the work is reworked before any runs on its own again.

## Multi-criterion judging

One narrow question per step forces every acceptance condition into a single yes/no, which either makes the question broad (violating rule 3) or silently drops conditions. A refusal also loses resolution: "the fix does not address the root cause" cannot say *which* of four requirements went unmet.

**`judge_check.criteria[]` replaces `question`.** Each criterion is dispatched as its own narrow yes/no call.
Why: this preserves rule 3 rather than bending it — the question gets narrower, not broader.

**A reading raises a concern for each criterion that draws a refusal.** Each is a call in the thread with its own citation, so the person can send one and dismiss another.

### Schema sketch

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

## What a refusal holds

A refusal's content is three named fields, never a formatted string:

| Field | What it holds |
| --- | --- |
| `expected` | What should be seen, returned or recorded if the work is right — the value itself |
| `produced` | What will be seen instead |
| `consequence` | What that difference does to whoever consumes it |

`consequence` is the field a person triages on. The concern card heads with it.

Summary prose is prompt plus mechanical lint, governed by `../contracts/agent-copy.md`. **Substance requirement: every Judge summary must cite a file, line or assertion that could not appear in any other Job's summary.** A summary that reads plausibly under a different Job has failed.

**A concern's citation is placed, never quoted back.** The words are already in `expected` and `produced`; what a reader lacks is where in the brief they came from. The labels are recorded as the brief is assembled rather than recognised in it afterwards, so a citation names the part the assembler wrote and not a heading a reader matched. A refusal that argues in the Judge's own words cites nothing and is a complete refusal under rule 4.

## Configuration

| Field | Purpose |
| --- | --- |
| `enabled` | Per-step. Some steps have no semantic question worth asking |
| `model` | Per-step model dial. The rigor-scaling mechanism |
| `criteria[]` | Per-criterion narrow questions — see above |
| `prompt_key` | Reference into the compiled-in prompt set |

**`model` is a deliberate per-step dial, not fixed fleet-wide.** Routine steps use a cheap/fast model for cost and latency; a stronger one is for the step that earns it. **The fleet-wide default is the cheapest model on the roster**, decided 28 Aug 2026 and carried by the Judge model config row. It is deliberately the floor rather than the middle: the dial is only legible against a default it moves away from, and Bug raising `review` to a stronger model says nothing if every step started there. A stronger default would not let the Judge see more — it reads the same checkout under the same turn cap whatever the model — it would only make every criterion cost more and take longer for a person waiting on a press.

**Judge prompts are compiled into the binary and are not user-editable.** No Kit library, no Manifest override, so prompt version is the binary version and no prompt hash is recorded on the verdict. Where the prompt library lives, and whether a prompt version is recorded on the verdict, is tracked in `../contracts/agent-prompt.md`; what a Judge is and is not told is specified there, sections 2 and 7.

**The Judge cost cap per check row still reads `undecided`**, and was reviewed and left open on 28 Aug 2026 rather than guessed at (see Open questions). What bounds Judge spend today is count-shaped rather than money-shaped: one call for each criterion, and only on a person's press.

The **Judge confidence threshold** row is superseded and awaiting disposal (see Open questions).

## Dispatch and approval

**A Judge call is not a dispatch.** It is a call with a verdict: no worktree, no branch, no rail, no Job of its own, under a cent. It needs no approval gate, since the person's press is what starts it.

## Open questions

- **[judge-cost-cap-per-check]** Does a Judge call get a cost readout, and is a per-check dollar cap the right shape for the sub-limit? Reviewed 28 Aug 2026 alongside the model default and deliberately left open: nothing can read what a Judge call cost, because the call is rendered as one-shot text and emits no result envelope, so the figure a cap would compare against never reaches Armada. A number written into the row today would be enforced by nothing while reading as a limit that was set, which is worse than an absent cap because only the absent one is visible as missing.
- **[judge-confidence-threshold-disposal]** Is the Judge confidence threshold config row deleted or repurposed? It is superseded and currently awaiting disposal.
