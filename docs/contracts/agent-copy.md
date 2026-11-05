# Agent Copy Contract

**Kind:** contract. **Governs:** text written at runtime by Drones and the
Judge — PR bodies, commit messages, Judge summaries, evidence
summaries. Sits under the
[Design System — UI & Voice](design-system.md) contract. Read before
writing or changing any lint, prompt or template governing Drone- or
Judge-authored text.

**Purpose:** governs text written at runtime by Drones and the Judge. The
[Design System — UI & Voice](design-system.md) page governs static UI
chrome and is the parent contract; everything here sits underneath it and
may not contradict it.

**Sibling:** the [Agent Prompt Contract](agent-prompt.md) governs text
going the other way — every prompt Armada assembles and injects. Same
parent, opposite direction. The exemplar corpus task below is consumed at
its layer 3.

**Why it is separate:** the Design System page is pasted into design
sessions. Prompt-lint rules for Drone-written commit messages dilute that
input without helping a design tool.

---

## Scope

The surfaces are the register below, ordered by whether the text leaves
Armada. Each carries who writes it, who reads it, what enforcement it draws,
and its samples. A surface whose Samples column reads None has no sanctioned
copy, which is what a lint asserting every surface has copy would fail on.

| Surface | Written by | Read by | Leaves | Enforcement | Shape | Samples |
| --- | --- | --- | --- | --- | --- | --- |
| Commit message | Fleet | Whoever reads the history later, including people who did not ask | Yes | None, composed from the record | Prose | Worked |
| PR description | Fleet | A reviewer deciding where to spend attention | Yes | None, composed from the record | Four headings, then prose | Worked |
| Work submission | Drone | A person, on escalation and at review — never the Judge | No | Prompt only | Named fields | Worked |
| Escape hatch narrative | Drone | The engineer taking over | No | Prompt only | Named fields | Worked |
| Judge record — refusal | Judge | A person triaging, and a Drone on retry, each a different projection | No | Lint warning, not built | Named fields | Worked |
| Judge record — advisory review | Judge | A human reviewer at the gate | No | Lint warning, not built | Prose | Worked |

The commit message surface is written down in the `commit-message` skill, at
`.claude/skills/commit-message/SKILL.md`, and has no section here.

Text going the other way is registered in the
[Agent Prompt Contract](agent-prompt.md) — the same split as the two
contracts.

---

## Enforcement by surface

**PR descriptions and commit messages — not linted.** Fleet composes both
from the Job's record, in `crates/fleet/src/review.rs` and
The `landing` module (deleted since), and admits no Drone sentence to either. A
Drone is denied `git`, so nothing it writes leaves Armada this way.

**Judge summaries — prompt plus lint, warning only.** The lint is not
built. The text stays inside Armada, and style-scoring the verification
path risks bouncing a sound judgment for tone.

The lint catches the phrase-level tells that cluster in generated text:

- Importance puffery: "marks a pivotal moment", "a testament to"
- Weasel attribution: "experts agree", "studies show"
- Faux-insight setups: "what nobody tells you", "the part everyone misses"
- Superficial analysis: "highlighting the team's commitment to..."
- Throat-clearing openers: "here's the thing", "let me be clear"
- Binary contrasts: "it's not X, it's Y"
- Colon reveals: "the best part: it learns"
- Dramatic fragments: "That's it. That's the whole thing."
- Fake-profound endings

**Evidence summaries — prompt only. Deliberately not linted.**

Evidence needs to be checkable, not well written. Layering style scoring
onto the verification path risks bouncing substantively sound evidence for
tone, which is the wrong failure mode to introduce into a gate.

---

## What a lint cannot do

A phrase blocklist will not hold against **shapes**. A clause welded into
a noun ("the read-vs-write split", "the fix-the-operations angle")
generates a new string every time, so no blocklist catches it. Ban the
grammar, not the phrase.

Shape-level rules live in the prompt. The real defence is P6 from the
parent contract, restated here because it is the operative rule for
everything on this page:

> Generated text is specified by what it must contain, never by what
> shape it takes. A structural rule ("state the finding, then the
> evidence, then the recommendation") gets satisfied identically forever
> and produces twenty interchangeable paragraphs.

**The substance requirement.** Every Judge summary must cite a specific
file, line or assertion that could not appear in any other job's summary.
A summary that would read plausibly under a different job has failed.

---

## PR description

Written by Fleet from the Job's record, read by a reviewer deciding where
to spend attention. Fleet's body keeps the four headings below, and its
outcome says what the record proves, because it admits no Drone sentence.

Same rule as a commit message — say what the diff cannot — with one job a
commit message does not have: telling a reviewer what needs them.

**It names no file and no Check that passed or was skipped.** The outcome
counts the files and the evidence counts each step's Checks, naming only those
that did not pass. The forge already lists the files, and the Job in Armada
holds every Check (#2161).

### Four headings, in this order, always

`.github/PULL_REQUEST_TEMPLATE.md` is the shape and this is why it has one.

| Heading | Answers | The failure it prevents |
| --- | --- | --- |
| `## Why was the change needed?` | What a person could not do, or what was wrong | A body that opens on the edit, so a reviewer never learns what it is for |
| `## What is the outcome of the change` | What the end result is, as a person meets it | A narration of the diff, which the reviewer already has |
| `## Risks` | What could still be wrong, and what was left alone | A leftover discovered in review instead of in the body |
| `## Checks Evidence` | What proves it | `Tests pass`, which proves nothing a reader can check |

**A short section keeps its heading.** Position is how a reviewer skimming a
queue finds the part they need; a body that drops a heading has dropped the
part the diff cannot supply. `Nothing` under Risks is an answer and it is a
different answer from silence.

**The order is not arrangeable.** Cause, result, cost, proof is the order a
reviewer decides in, and the two most commonly dropped — Risks and Checks
Evidence — are last precisely because they are the two a writer runs out of
patience for. Their position is what makes their absence visible.

### Show the outcome, do not describe it

The outcome section takes an example. The sentence that now appears, the
screen that now draws, the command that now answers — quoted, so a reviewer
reads what a person will read rather than a paraphrase of it.

**A code block is not an outcome.** A diff hunk pasted into the body is the
diff again, one scroll higher. What belongs there is what the code
*produces*.

### Bullets are legal, and they are the middle

A section made only of bullets has dropped the cause and the caveat, which
are the two parts a reviewer needs and the two the diff cannot supply.
Rendered markdown scanned on a screen is a different medium from `git log`.

### A heading beats a colon reveal

"Worth a look:" is a dramatic setup where a heading is the honest
structure, and this contract bans colon reveals for exactly that reason. The
four above are the structure; a fifth heading inside one of them is legal
and a colon reveal is not.

### Naming a leftover is not asking for a decision

A risk needs the cost, the reason it was not fixed, and the ways forward. A
reviewer who cannot act on a caveat skims past it. This is the whole of why
`## Risks` is a heading rather than a closing sentence.

### Sample

```
## Why was the change needed?

Two Jobs against the same repo resolved to the same worktree path, and
the second failed inside libgit2 naming neither Job.

## What is the outcome of the change

A second Job against a repo one is already working now gets its own
worktree, and the failure it used to raise is gone. Where it used to
stop, it now reports:

> armada/01K3Q… is holding crates/api. Dispatched to a worktree of
> its own.

## Risks

Cleanup still keys off the repo name, so a killed Job now leaves a
worktree directory nothing removes. One per kill.

Fixing that means changing the sweeper's match pattern, which is the
retention path too, so it is wider than this change. Ship this and
file the leak separately, or send it back and it goes in here.

## Checks Evidence

- `test_concurrent_dispatch` drives two Jobs against one repo and
  asserts two worktree paths.
- The existing single-Job tests are unchanged and still green — a
  reviewer who knows that does not go looking for a change that is
  not there.
- `cargo test --workspace` green.
```

One line under Checks Evidence says what was left alone. Compare the
outcome section against the shape-to-avoid sample in the `commit-message`
skill: "Updated add() to accept job_id parameter" narrates the edit; the
quoted sentence above is what a person now meets. Same form, opposite
substance — the `claimed` distinction again.

---

## Work submission

Written by a [Drone](../concepts/drone.md), read by a person on escalation
and at review. The [Judge](../concepts/judge.md) never reads it.

Deliberately not linted. Evidence needs to be checkable, not well written,
and style-scoring the verification path risks bouncing sound evidence for
tone.

| Field | Holds |
| --- | --- |
| `claimed` | What the work now does, as an observable — behaviour, never a description of the change |
| `shown_by` | The artifact demonstrating it — a named test, a command and exit code, a rendered string |
| `not_claimed` | Everything the claim does not assert — the gap, and the side effect |
| `what_changed` | Attempts after the first only |
| `in_the_way` | Optional. One line on what got in the Drone's way |

### Not evidence

`in_the_way` is the one field the gate, the Judge and a reviewer of the work
never read. It is kept for the Job's [retro](../concepts/retro.md), so a
Drone saying what slowed it costs the work nothing.

### Not circular

`claimed` is behaviour and `shown_by` is an artifact. Where both are the
same sentence — "the test passes", shown by the test passing — nothing has
been evidenced.

### Named

"Tests pass" is not an artifact. `test_concurrent_dispatch` green with
`cargo test -p vcs` at exit 0 is.

### Run, on a captured step

Where a step's `evidence.captured` is true, `shown_by` is not only read.
Fleet runs the file it names through this repository's `evidence.run` once
the Drone submits — `docs/concepts/manifest.md`'s Evidence section says what
that command expects and what it leaves behind. The step's own instruction
carries this to the Drone, composed in the `terms` module (deleted since); it is not
repeated here.

### Two kinds in one field

`not_claimed` holds both a gap and a side effect — what the work leaves
undone, and what it changed that nobody asked for. A reviewer acts on those
differently, and both belong to the reader rather than to the claim.

### Empty is legal, absent is not

A Drone saying it left nothing behind is not a Drone declining to answer.

### No gaming payoff

Padding `not_claimed` buys nothing at the gate: the Judge never reads it and
the mechanical tier does not parse it. Its only reader is a person, and a
person cannot be talked into advancing a step.

### Samples

**Feature · refine.** The record that would have made a later refusal
unnecessary — the Drone noticed the width and said so.

| | |
| --- | --- |
| Claimed | The row shows quota percent on a personal machine and dollars against the cap on a work machine |
| Shown by | Both modes render in `job_row.stories.tsx`; `npm test` exit 0, 42 passing |
| Not claimed | The column width is unchanged, and the dollar string is longer than the quota string |

**Bug · repro.** `claimed` is the symptom, not the test. "A failing test was
added" describes the change and evidences nothing.

| | |
| --- | --- |
| Claimed | A second Job against the same repo dies at worktree registration |
| Shown by | `test_concurrent_dispatch` fails with `worktree path already registered`; `cargo test -p vcs` exit 1 |
| Not claimed | The test dispatches two Jobs, not three |

**Bug · fix.** Where the Drone prompt's "do not fix adjacent problems you
notice — say so under Not claimed instead" lands.

| | |
| --- | --- |
| Claimed | Two Jobs against one repo now take separate worktrees and both start |
| Shown by | `test_concurrent_dispatch` green; `cargo test -p vcs` exit 0, 34 passing |
| Not claimed | The sweeper still matches on repo name, so a killed Job's directory now survives |

**Refactor.** A refactor claims an absence, which is the hardest thing to
evidence. `shown_by` has to name the tests that would have caught a change,
since a green suite proves nothing about behaviour it never asserted.

| | |
| --- | --- |
| Claimed | Manifest resolution moves behind a store with no change to what callers receive |
| Shown by | `cargo test` exit 0, 214 passing, the three `no armada.yml` cases among them |
| Not claimed | The store reads once per repo where the old path re-read per call |

**Code Review · assess.** The inverted workflow holds without a special
case: `claimed` stays what a reader can do, and the work product is the
review rather than the code.

| | |
| --- | --- |
| Claimed | An author of PR 218 has four findings to act on, each on a changed line |
| Shown by | `REVIEW.md`, findings at `worktree.rs:40`, `:52`, `dispatch.rs:88`, `:104` |
| Not claimed | The two test files in the diff were read and not assessed |

**Feature · tests · attempt 2**, after a refusal. `what_changed` is not a
rule a Drone has to remember — it is the answer to the question the refusal
reprompt asks.

| | |
| --- | --- |
| Claimed | The suite goes red when a work machine's row shows quota percent |
| Shown by | `spend_mode_render` fails on the parent commit and passes here; `npm test` exit 0, 43 passing |
| Not claimed | Empty |
| What changed | The test asserts the rendered string where it asserted the call |

---

## Escape hatch narrative

Written by a [Drone](../concepts/drone.md), read by the engineer taking
over. Prompt only.

**Not Evidence.** Evidence is proof tied to an advance gate; this states
that no proof is coming. It lands in the handoff bundle a
[Pilot](../concepts/pilot.md) session opens with.

The Drone's only contribution to the handoff. Fleet assembles everything
else.

| Field | Holds |
| --- | --- |
| `trying_to` | What the step was meant to produce |
| `blocked_by` | The specific thing preventing it |
| `tried` | What was attempted, and what each attempt produced |

### `blocked_by` is the field that has to be specific

It is the one a person acts on. "I am stuck" names nothing to unblock. A
denied command names an allowlist entry someone can change in a minute.

### The denial is reported, never routed around

That clause is in the baseline, and this field is where a Drone honouring it
puts the result.

### Sample — Bug, fix

| | |
| --- | --- |
| Trying to | Stop a second Job colliding at worktree registration |
| Blocked by | Clearing the stale registration needs `git worktree prune`, which the allowlist denies |
| Tried | Building the path three ways; each one still collides at registration |

---

## Judge record — refusal

Written by the [Judge](../concepts/judge.md), read by a person triaging and
by a Drone on retry — but each sees a different projection. Lint warning.

The record is a fixed set of named fields. Labels are Fleet's fixed copy,
where uniformity is the point, so P6 governs the values and stops governing
the shape.

| Field | Holds |
| --- | --- |
| `expected` | What should be seen, returned or recorded if the work is right — the value itself |
| `produced` | What will be seen, returned or recorded instead, as the value itself |
| `consequence` | What that difference does to whoever consumes it; the triage field |

### Same kind

`expected` and `produced` must be the same class of artifact — a rendered
string against a rendered string, a response body against a response body, a
stored row against a stored row. Where the change renders nothing, the
observable is the caller's.

### Action

`expected` names the action and its result together, never the result alone.
A value with no behaviour attached — a string, a count, a message — can be
produced by a change that does not do the work. This carries the whole
weight, because `consequence` never reaches the Drone.

### In flight

On a mid-step pass for thrashing or plan drift, `produced` is what the
branch does as it stands, and `consequence` carries the deterministic facts
that fired the trigger. The Judge never reads the Drone transcript. It reads
the branch and is told the count.

### Plan

Where the evidence is a plan, a note or a document, the observable is what a
reader can do with it. A later step consuming it makes `produced` what that
step would build; a person consuming it makes `produced` the decision they
can and cannot reach.

### One line

A field that will not fit in a line is a finding that has not been made yet.

### Not a refusal

A criterion drawing no refusal writes no prose at all. Its verdict word is
on [Design System — UI & Voice](design-system.md) under Status grammar, with
the rest of the verdict vocabularies.

### Samples

**Feature · tests · criterion refusal**

| | |
| --- | --- |
| Expected | Suite red when a work machine's row shows `68% quota left` |
| Produced | Suite green on a row rendering no spend at all |
| Consequence | An empty spend column ships as verified |

**Feature · tests · gaming check.** The gamed artifact is a terminal line
that reads as success, and quoting it is the entire argument.

| | |
| --- | --- |
| Expected | A suite run that includes this Job's tests |
| Produced | `41 passing, 0 failing`, none of them this Job's |
| Consequence | Green reads as verified and nothing was |

**Refactor · behaviour changed.** Where expected and produced are meant to
match, the same-kind rule carries everything: both halves are what the
caller receives.

| | |
| --- | --- |
| Expected | Dispatch refuses with `no armada.yml in /repos/api` |
| Produced | Dispatch proceeds against an empty Manifest |
| Consequence | An unconfigured repo runs a Job with no Checks |

**Code Review · assess.** The inverted workflow. The observable moves from
what the code renders to what the author reads, and both halves stay the
review document.

| | |
| --- | --- |
| Expected | Findings the author of PR 218 can act on |
| Produced | `error handling could be more robust`, and one more like it |
| Consequence | A review round that changes nothing |

**Bug · fix · one of three refused.** Unanimity needs no extra copy. The two
who did not refuse produce no record, so a panel refusal reads exactly like
a single one, and the count sits beside the record rather than inside it.

| | |
| --- | --- |
| Expected | N concurrent Jobs, N distinct worktree paths |
| Produced | Two Jobs work, three collide as before |
| Consequence | The reported failure returns one Job later |

**Bug · fix · thrashing.** Thrashing is the absence of change in `produced`.
The turn count is not the finding — it is what makes an unchanged observable
mean something.

| | |
| --- | --- |
| Expected | A second Job starts on its own path |
| Produced | `worktree path already registered`, unchanged since turn 9 |
| Consequence | 32 turns and three attempts have moved nothing |

**Feature · scope · plan-shaped evidence.** The note renders nothing, so
`produced` is what the next step builds from it.

| | |
| --- | --- |
| Expected | One changed row on the Job Board |
| Produced | A row, a detail column, a tooltip and a setting |
| Consequence | A settings surface nobody asked for ships |

---

## Judge record — advisory review

Written by the [Judge](../concepts/judge.md), read by a human reviewer at
the gate. Lint warning.

`gates_advancement: false`. It cannot refuse, so silence would leave it with
no output at all, and pointing a person at something is its entire job.

### Prose, plus the record for the finding

The summary answers what the diff does; the three fields carry the one thing
a person has to act on, in the same shape they read everywhere else.

### Sample — Feature, spend renders by billing mode

> On a personal machine the row shows `68% quota left`, which is the figure
> gating dispatch there and the one the scope note asked for. Both modes
> read the machine's billing mode, and the approximation mark sits on the
> dollar figure only, matching how the two numbers are sourced.
>
> One thing to look at. The column was sized to the quota string and the
> dollar string is longer.

| | |
| --- | --- |
| Expected | `~$2.40 of $20` |
| Produced | `~$2.40 of $2…` |
| Consequence | The gating number reads ten times tighter than the cap |

### Sample — Refactor, nothing wrong

The case most likely to produce filler, and the record absorbs it. A diff
with genuinely nothing to report leaves the record empty, which is the
honest output and reads as one.

> Sixteen call sites move from `Config::get()` to the store. The three cases
> that produced `no armada.yml in <path>` still produce it character for
> character, and exit codes are unchanged.
>
> One behaviour does differ.

| | |
| --- | --- |
| Expected | The Manifest read once, when the Job is created |
| Produced | Read once per repo, held for the daemon's life |
| Consequence | A Manifest edited between two Jobs lands one Job late |

---

## The Helm reply is retired

The Helm reply surface went with Helm, and nothing here governs a reply written in real time to the person.
Its samples and rules are at `git show a78c90d93c:docs/contracts/agent-copy.md`, under *Helm reply*, and what governs the [Operator](../concepts/operator.md)'s read and reasons is not written yet.

---

## Deferred: the rubric

Scoring generated text with a cheap model against a small rubric
(directness, density, specificity) is the right eventual answer to
queue-level sameness, which is invisible one message at a time and
obvious across twenty.

Not yet. It costs a model call per artifact, and comparing a PR
description against the last thirty requires thirty to exist. Revisit
once the corpus does — tracked below.

---

## Task: build the exemplar corpus

Seeding the prompt with real samples beats adding rules. Measured
evidence from the field: AI-generated templates used words the real
author had never once used, and the fix was samples rather than another
rule.

- **Find the AI cutoff in the commit history.** Do not assume a date.
  AI-written messages have a shape: conventional-commit prefix plus a
  bulleted body, or a paragraph restating what the diff already shows.
  Inspect a window either side.

```bash
git log --since=2025-06-01 --until=2026-01-01 --format='%ad %s%n%b%n---' --date=short
```

- **Target roughly 50 commit messages and 10 PR bodies.** Commit messages
  are one line. This is not a 100,000-word corpus problem, and at that
  size every sample can be read before it goes in.
- **Curate, do not dump.** The failure mode with old commits is the
  opposite of slop: half will be "wip" and "fix typo", and dumping them
  teaches a Drone to write "wip". The ones worth keeping are where
  something was actually explained, which is a small fraction.
- **Fallback.** If the pre-cutoff corpus is thin or poor, hand-write 8 to
  10 exemplars. About an hour, and arguably better, since the awkward
  cases (a revert, a partial fix, a change that turned out wrong) can be
  covered deliberately.

**Where it fits:** the M0 v1 harvest step, since `v1-archive` is already
open at that point.

**Status:** method decided Aug 2026 — the date is not chosen up front. M0
finds it by inspecting a window either side of the suspected boundary and
**records the method and the date alongside the corpus**.

---

## Open questions
- **[copy-rubric-scoring]** Should generated text be scored against a rubric by a cheap model?
  Scoring against directness, density and specificity is named as the
  right eventual answer to queue-level sameness — invisible one message at
  a time, obvious across twenty — since a phrase blocklist can't catch a
  clause welded into a noun that generates a new string every time.
  Deferred on a stated trigger rather than rejected: it costs a model call
  per artifact, and comparing a PR description against the last thirty
  requires thirty to exist first. Revisit once the corpus does.

Also bearing on this document, and written where it belongs: `[copy-lint-surface-narrowing]` in `configuration.md` — whether a Manifest may narrow which surfaces this lint covers.
It lives there because the answer turns on the config direction rule, which that document records as withdrawn.
