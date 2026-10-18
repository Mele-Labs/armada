# 5. The Drone baseline

**Part of:** [Agent Prompt Contract](../agent-prompt.md).

What ships as `prompts.droneBaseline`; a person's override replaces this
text and nothing else in the brief.

**Revised Aug 2026** after adversarial review. The first cut carried eight
clauses. Three belong here, four belong where their mechanism lives, and
one is deleted.

## The membership test

**A baseline clause must name an action the Drone can take or decline to
take. A clause that names no action fails.**

This is the acceptance test the first cut lacked, and it is the
counterpart to the [Agent Copy Contract](../agent-copy.md)'s substance
requirement — a content rule rather than a shape rule, so it survives P6,
and checkable, so section 5 can be wrong.

A second test decides between the baseline and per-Job assembly: **does
the clause name a mechanism the Drone must reach for, or describe a system
property it cannot act on?** A mechanism the Drone cannot discover any
other way belongs in layer 1. A property belongs beside the thing it
describes, where it is timely.

## Why the baseline is small

Instruction-following degrades non-linearly in the number of simultaneous
instructions — measured elsewhere at roughly 96% down to 20% as stacked
verifier-checked instructions rise from one to twenty, with
inter-instruction tension a principal driver.

The exposure is worse than the clause count suggests, because layer 1 is
only the first of six. A Drone on a Bug step carries the baseline, plus
Voice, **Way I work** and **Expectations**, plus Manifest conventions,
plus WorkflowDef framing, plus `acceptance_criteria[]`, plus the step's
criteria and Evidence Scope policy, plus the exemplar corpus on prose
steps. **Section 3 bounds none of it.** A constant prepended forever to an
unbounded stack is the thing to minimise, which is why four clauses moved
out rather than staying for tidiness.

## The three clauses

| Clause | Action it names | Why layer 1 rather than beside its mechanism |
| --- | --- | --- |
| **Completion is claimed through the Evidence tool, and nowhere else.** Saying a step is done in prose does not advance anything | Call the tool | The v1 failure. A Drone that does not know the sanctioned path invents one, and it needs to know before the first step ends |
| **Stopping and handing back is a legitimate way to finish. Guessing is not.** `escape_hatch` ends autonomous execution and passes the Job to a person | Call `escape_hatch` | The moment it is needed is the moment the Drone has stopped reading the step, so no per-Job assembly gets it in front of the Drone reliably |
| **A denied command is denied, not an obstacle to route around.** Denials are silent, so notice them rather than continuing without the capability | Stop and report on a denial rather than proceeding | Armada's permission tool answers every call outside the toolbelt, and a refusal may come back as a bare error with no reason. The measured v1 failure is work continuing past one |

**Framing is deliberately unspecified on the `escape_hatch` clause.** The
abstention literature finds opt-out rates move materially with the
authority through which the option is framed, and fall as that authority
falls. "Sanctioned" is the maximal framing available.

The clause specifies the content; the authority word waits on
measurement. The poke draft in section 4a reaches for *sanctioned*
already, which decides in a draft what this section leaves open.

**That is incoherent as it stands, and there are three ways out.** Not
picked here — a person picks.

| Way out | What it costs |
| --- | --- |
| **The poke draft drops the word** | The turn loses its framing at the moment 4a says framing matters most — a Drone gone quiet. Nothing else. Reversible in a line |
| **The baseline adopts it** | Answers the open question from a draft, at the top of the range, where a Drone bails on work it could have done. Layer 1 is compiled in, so revising it is a build |
| **The measurement settles it** | Runs, and the time to design them. Blocks nothing while it is pending, provided the draft is not standing in for the answer |

**The contract's own reasoning favours the third, with the first as its
interim.** Three things in this document point the same way. Section 5
states the question is open pending measurement, and a draft is not where
an open question gets answered. Layer 1 is unreachable from config, so a
word chosen there is the most expensive one in the stack to change.
And the baseline may carry no claim about consequences that is not true —
an authority nobody has established is exactly such a claim.

**The third has now been run.**
[Spike 7](../../spikes/007-will-a-drone-take-the-escape-hatch.md) measured
the word against a task that cannot be done, on a fixed resolution
declared before the runs. Framing moved abstention from half to all of
them, monotonically in the height of the word, at no bail rate the design
could detect on work that could be done. It cannot separate *legitimate*
from *sanctioned*, so the milder word is not a choice against evidence.

**The measurement supports the word. It does not sanction it** — that is
still a person's, and the clause and the poke draft stay marked as drafts
until one says otherwise. Two things the spike found bear on this section
directly. The mechanism sentence on its own moved nothing measurable,
which makes the clause the carrier and the word the payload rather than
two separable decisions. And [Pilot](../../concepts/pilot.md)'s mark now
turns an unbidden pull into an escalation, so the quantity this section's
premise is written about — an opt-out rate — is one the mechanism no
longer produces.

## The four that moved, and the one deleted

| Clause | Now lives | Why not the baseline |
| --- | --- | --- |
| Insufficient evidence gets a clarification round, not a failure | **The clarification reprompt itself** — section 4a | Describes what Fleet will do. It is timely at the moment Fleet does it, and stating it in advance spends a constant on a case most Jobs never hit |
| Work stays inside the declared `context_paths` | **Layer 6**, per step, per `context_source` | `context_paths` is a Drone **output** under `drone_declared`, not an input. The obligation is to declare, and when — which is `declare_plan_at`, a step field |
| Secrets are brokered and never held | **Baseline — retained** | The review sited this at layer 3 with the brokered scope. Kept here instead: a Drone that does not know secrets are brokered may search the repo for one **before** reaching any secret-needing step, so the clause has to precede its own mechanism. It names an action — stop and report rather than search |
| Fleet verifies independently. Self-report is a signal, never the source of truth | **Nowhere, pending measurement** | Names no action, and the counter-hypothesis is at least as plausible: a model told an independent verifier will re-derive the truth has less reason to invest in evidence, not more. The Judge is blind to the Drone, so better prose cannot influence the verdict anyway — which makes spending tokens on the diff instead the **rational** response to being told this |
| The allowlist is not a sandbox | **Deleted** | Names no action and offers none. A Drone has no channel by which to establish that an allowed operation is safe — worktree confinement is enforced by the filesystem and by a VCS type with no push method, neither of which a Drone can consult. The sentence it derives from is addressed to a human designer |

**What the baseline must not contain.** No shape rules — P6 applies to
input as much as output, and a structural instruction gets satisfied
identically forever. No role-play framing. No claims about consequences
that are not true. **And no clause that fails the action test**, which is
what deleted the sandbox line.

**A caveat this section must carry about itself.** A baseline written as a
numbered list of rules is the shape a model pattern-matches and then
satisfies formally. The table above specifies content; the wording is
below, and whether the same content lands better as prose or as worked
examples is unmeasured.

## The M1 renderings

**The Kit and Manifest layers are absent, because M1 has neither.** The
six-layer order in section 3 is unchanged. Eight decisions fell out of
writing these, and each is visible in the text.

**The reporting clause is load-bearing, not a reminder.** Given one MCP
tool and a task, told nothing about reporting, a Drone fixed the code and
wrote a tidy sentence saying so — four times out of four, no tool call.

A Drone does not reach for the Evidence tool on its own, so the baseline's
second paragraph is the difference between a working gate and a Job that
stalls on every step. Measured in
[spike 6](../../spikes/006-will-a-drone-use-the-evidence-tool.md).

- **The tool is described, not named.** "The evidence submission tool you
  have been given" — the MCP tool's own description carries its name, so
  the prompt and the tool cannot drift apart.
- **"Parts", not "steps".** Deliberate: `step` is Armada's word for a plan
  artifact, and a Drone that learns the system's vocabulary can reason
  about the machinery. This orients it without teaching it the schema.
- **The stop sits inside the list, not after it.** Where the line falls
  *is* the boundary. Later parts are marked not-yours with the specific
  prohibition, because "do not run it" prevents a concrete thing that
  "stop here" does not.
- **The part before is quoted *and* its file is named.** Quoting alone
  hands the next Drone a sentence a Drone typed about a document, which is
  what a step's product being a real file exists to stop. Naming the path
  alone spends a tool call to read two lines. Where the earlier part
  declared no file — most steps, whose product is the diff — the block
  points at the branch instead, and no path is invented.
- **An earlier part with nothing recorded says so.** The two absences are
  not the same absence: on part 1 the block is not rendered at all, because
  the rail directly above marks nothing done and there is nothing an
  absence could be read as answering. On a later part the rail *does* mark
  the earlier one done, so silence there reads as a block that was
  answered, and it is spoken instead.
- **What the earlier part did not claim crosses too, and it is the only
  field that had to.** `claimed` summarises a document that is a real file
  in the same worktree, and the block names the path, so a Drone that wants
  the whole of it opens the file. `not_claimed` is nowhere else. It is also
  the field bearing most directly on what this part must not spend its turn
  on — a gap the earlier part left on purpose is the thing a later part is
  likeliest to go and close unasked. So it is framed as context and never
  as work, in the sentence under it, and a Drone that read it as a to-do
  list would do the exact thing the field exists to prevent.
  **An empty one renders nothing at all** — `agent-copy.md` makes empty
  legal and absent illegal, and a label with nothing under it turns a part
  that left nothing behind into a part that declined to answer. The Code
  Review rendering below draws that case; the Bug rendering above draws the
  other.
- **The receipt paragraph says which outcome comes back and which does not.**
  It used to promise a later turn with the reason whenever the work did not
  pass, and one outcome sends one: a Check that failed with an attempt left.
  A refusal, a gaming flag and a gate that could not decide each stop the step
  and go to a person, and none of them carries a message; a step that passes
  ends its Drone as well. Written only about refusals it would leave a Drone
  whose work passed reading its own silence as one, so the promise is stated
  positively — a turn where the part is coming back, and every other ending
  named as an ending.

The rule they all serve — that no block a Drone is given is written from a
Check's command — is on [Drone](../../concepts/drone.md), since it governs
every Drone-facing surface rather than only these samples. **A Check's
name is not its command**, and the asked-run offer names the step's Checks
by name for the reason the offer exists at all: a Drone that cannot tell
what a call would check can only spend one to find out.

### Bug's plan step — added for `#894`

A plan step declares no `deliverable` — `record_plan` is the recording,
not a file — so `Delivering` never fires for it. `WHAT THIS PART DELIVERS`
is the block that fills the gap, on the one step that gets it instead of a
file path.

**Drafted wording. Not sanctioned.**

```
┌─ BASELINE ──────────────────────────────────────
│ [identical to the sample above, on every step of
│  every Job. Mechanics, never task content.]
└──────────────────────────────────────────────
┌─ JOB BRIEF ───────────────────────────────────
│ Repository: armada
│
│ The store's cursor reads one row past the end. Stop
│ it at the end, and cover the bound with a test.
└──────────────────────────────────────────────
┌─ WHERE YOU ARE ────────────────────────────────
│ This work runs in three parts. You are on part 1.
│
│   1. Plan the change      ← you are here
│   ─────────────────────────────────────────────
│   ▌ STOP. Submit when part 1 is done, then wait.
│   ─────────────────────────────────────────────
│   2. Implement            ✗ not yours — do not do it
│   3. Hand off             ✗ not yours — do not do it
│
│ Parts 2 and 3 happen after you submit, and doing them
│ yourself does not move the work forward. Leave the
│ branch in a state they can start from.
└──────────────────────────────────────────────
┌─ STEP: Plan the change ──────────────────────────
│ What you claim should be what the work now does, not
│ that you finished. An adjacent problem you notice and
│ leave alone goes under Not claimed.
└──────────────────────────────────────────────
┌─ WHAT THIS PART DELIVERS ─────────────────────
│ This part's product is the Job's plan. Record it
│ with record_plan: an approach in a paragraph, then
│ the tasks it breaks into, in the order they will be
│ done. Recording again replaces the whole plan, so
│ correct one by recording it again. The checks each
│ part must pass run on their own when that part is
│ submitted, so the plan carries no task for running
│ them. Recording does not finish this part —
│ submit_evidence still does.
└──────────────────────────────────────────────
```

No `THE PLAN` block: the step recording the plan is never shown the plan
it is about to replace — `fleet::crossing::ThePlan`'s own rule.

### Bug, part 2 of 4

```
┌─ BASELINE ──────────────────────────────────────
│ You are working in a git worktree on a branch of your
│ own. You cannot push, open a pull request, or run
│ commands this repository has not declared.
│
│ When you have finished the work described below, you
│ must report it using the evidence submission tool you
│ have been given. It is the only way to report. Work
│ you do not submit is work no one sees, and nothing
│ moves on.
│
│ Submitting returns "recorded". That is a receipt, not
│ a verdict — your work is checked after you submit. A
│ later turn comes only where the part is coming back to
│ you: a check failed and there is another attempt, or
│ what you submitted could not be read. Wait for it
│ rather than submitting again. Every other outcome ends
│ your part where it stands and sends you nothing — work
│ that runs and is refused for what it is goes to a
│ person, and you are not asked about it again.
└──────────────────────────────────────────────
┌─ JOB BRIEF ───────────────────────────────────
│ Repository: armada
│
│ Dispatching two Jobs against the same repo leaves the
│ second one's worktree at a path the first already
│ registered, and it fails with a message naming neither.
└──────────────────────────────────────────────
┌─ WHERE YOU ARE ────────────────────────────────
│ This work runs in four parts. You are on part 2.
│
│   1. Plan the change      ✓ done
│   2. Implement            ← you are here
│   ─────────────────────────────────────────────
│   ▌ STOP. Submit when part 2 is done, then wait.
│   ─────────────────────────────────────────────
│   3. Run the suite        ✗ not yours — do not run it
│   4. Summarise            ✗ not yours — do not write it
│
│ What part 1 produced:
│   "The worktree path is derived from the repo name
│    alone, so a second Job collides. It should carry
│    the job id. worktree.rs, add() — the path is built
│    at line 40."
│
│ Its work is on the branch you are in.
│
│ What part 1 did not claim:
│   "The sweeper matches on repo name too, and this
│    plan does not change it."
│
│ That is everything its claim does not cover — a gap it
│ left on purpose, or something it changed that nobody
│ asked for. It is context for this part and not a list
│ of work this part owes.
│
│ Parts 3 and 4 happen after you submit, and doing them
│ yourself does not move the work forward. Leave the
│ branch in a state they can start from.
└──────────────────────────────────────────────
┌─ THE PART BEFORE THIS ONE ─────────────────────
│ Plan the change passed the checks that gate it, and
│ its work is on the branch you are in. It is settled:
│ it is not yours to do again, to review or to improve
│ on. Start this part from it.
└──────────────────────────────────────────────
┌─ STEP: Implement ────────────────────────────────
│ Make the smallest change that addresses the cause
│ identified in part 1. Do not fix adjacent problems you
│ notice — say so under Not claimed instead.
│
│ Read before you write. The rust-conventions skill
│ covers error handling and module layout here.
│
│ What you claim should be what the work now does, not
│ that you finished.
└──────────────────────────────────────────────
```

The step's closing line is where the work submission's `not_claimed` field
comes from — an adjacent problem noticed and left alone has somewhere to
land.

### Code Review, part 2 of 3 — the inverted case

The test of the layering, since the diff is the Drone's **input** rather
than its output.

```
┌─ BASELINE ──────────────────────────────────────
│ [identical to the sample above, on every step of
│  every Job. Mechanics, never task content.]
└──────────────────────────────────────────────
┌─ JOB BRIEF ───────────────────────────────────
│ Repository: armada
│
│ Review PR #218, which reworks how worktree paths are
│ built. It touches the dispatch path, so pay attention
│ to what happens when two Jobs start at once.
└──────────────────────────────────────────────
┌─ WHERE YOU ARE ────────────────────────────────
│ This work runs in three parts. You are on part 2.
│
│   1. Read the changes     ✓ done
│   2. Assess               ← you are here
│   ─────────────────────────────────────────────
│   ▌ STOP. Submit when part 2 is done, then wait.
│   ─────────────────────────────────────────────
│   3. Deliver              ✗ not yours — do not post it
│
│ What part 1 produced:
│   "6 files, 340 lines. worktree.rs add() now takes a
│    job id and builds the path from it. dispatch.rs
│    passes it through. Three call sites updated, two
│    tests changed, one added."
└──────────────────────────────────────────────
┌─ STEP: Assess ───────────────────────────────────
│ The changes are below in full. You did not write them
│ and you are not fixing them — you are reviewing them.
│
│ Write your findings to REVIEW.md. Tie every finding to
│ a specific file and line. A finding that would apply
│ to any diff is not a finding.
│
│ "No issues" is a legitimate conclusion on a small,
│ clean change. It is not a legitimate conclusion you
│ reach quickly on a large one.
│
│ Do not edit the code you are reviewing.
└──────────────────────────────────────────────
┌─ THE CHANGES ──────────────────────────────────
│ diff --git a/crates/vcs/src/worktree.rs ...
│ [340 lines, injected]
└──────────────────────────────────────────────
```

**What the inverted case forced.** A fifth block — the diff is injected as
its own section *after* the step instructions, because it is reference
material rather than an instruction. And `Do not edit the code you are
reviewing`, which stops a review Job quietly becoming a fix Job: a coding
Drone's instinct on seeing a diff is to improve it.

Everything above the step block is structurally identical between the two
samples. Only the step text and the injected material differ, which is the
layering doing its job.

### A task's Drone — added 2 Oct 2026

On a step declaring `drone_per_task`, THE PLAN block carries the plan as
every later step's does, and then names the task this Drone was put on.
`fleet::crossing::ThePlan::for_task` renders it. It carries none of the
`update_task` sentence a step following the plan is given, because the
Drone is offered no plan tool.

**Drafted wording. Not sanctioned.**

```
┌─ THE PLAN ──────────────────────────────────────
│ Approach: Stop the reader at the end, then cover it
│
│ Tasks:
│   T1 [handed_in] Stop the reader at the end
│       shown: read::stops passes
│   T2 [working] Cover the last row
│   T3 [open] Note the bound in the module
│
│ YOUR TASK
│
│ This part's tasks are worked one at a time, each by
│ an agent of its own, and yours is T2: Cover the last
│ row. Its line in the plan above says what it touches
│ and what should show it is done. Do T2 and nothing
│ past it: the tasks handed in or done before it are
│ already on the branch you are in, and the open ones
│ after it are each another agent's.
│
│ When T2 is done, submit it with the evidence
│ submission tool. What you claim, and what shows it,
│ are about T2 alone. That submission is your hand-in,
│ and it ends your work on this Job. The plan's states
│ are kept from it, so you mark nothing yourself.
└──────────────────────────────────────────────
```

**"Ends your work on this Job"** is the one thing a task's Drone does that a
step's does not, and the rail's *Submit when this part is done, then wait*
still holds beside it: a hand-in that is not the last is followed by Fleet
ending the Drone, and the last one's Drone waits for the outcome as any does.

### Files another Job is fixing — added 2 Oct 2026

Where a test this Job's checks failed on is claimed by another Job's fix
(#1673), every sample above gains one block after the peer block and before
the step, for every Drone the Job puts on. `fleet::fixing::HeldOff::text`
renders it, naming every file and never counting them away.

**Drafted wording. Not sanctioned.**

```
┌─ FILES ANOTHER JOB IS FIXING ───────────────────
│ A test this Job's checks failed on is another
│ Job's to fix, and these files are that fix's.
│ Until it lands and reaches your copy they are
│ outside what this Job may change: a declaration
│ naming one is refused, and so is an edit to one.
│ Do the rest of your part around them, and say in
│ your evidence that the test still fails until
│ the fix lands.
│
│ - "fix the parser on main" (7-fix-the-parser)
│   is fixing `parser::takes_it`: `src/parse.rs`.
└──────────────────────────────────────────────
```

**"Reaches your copy"**, not "lands": a fix that has merged is still absent
from this Job's branch until the next part's catch-up, and the block keeps the
line, as *landed its fix, and your branch could not take it yet*, where that
catch-up could not replay.
