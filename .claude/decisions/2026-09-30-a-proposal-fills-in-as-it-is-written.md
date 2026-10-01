# How does a proposal reach the screen — whole, or a field at a time?

**Decided 2026-09-30.** The next half of
`2026-09-30-a-dispatched-request-is-a-job.md`, taken after looking at what that
one built.

A dispatched request is a Job from the press, so there is a row the moment
Dispatch is pressed. What that row had was its status, its request as a title,
and a wait saying how far a model call had got — and then, all at once, an
answer. He looked at it and asked for the rest:

> Now that we have this I would really push for us to find a way to make the
> proposer not report the job whole. Is there anyway for it to fill in as it
> goes?
>
> 1. Workflow
> 2. Title
> 3. Done When
> 4. Settings

## The order is his, and it is not arbitrary

**Workflow** decides the Job's shape — every other decision on the screen hangs
off which workflow this runs under. **Title** is what makes the row
recognisable, and until it lands the row's title is the request as he typed it.
**Done when** is the goal. **Settings** are the part he can still change, so
they are the part it costs least to learn last.

That order is now the order the answer format asks the model to write in, which
makes it a contract rather than a layout — `crates/fleet/src/proposing.rs`,
`ANSWER_FORMAT`.

## One call, read as it arrives

Offered four smaller calls, one fact each, against one call read as it is
written.

**Chosen: one call, read as it arrives.** The proposer stays one model call with
one reading of the request behind it — *which workflow, what to call it and
whether it is one Job or several answer the same question* — and what changes is
that Fleet reads the answer's prefix instead of only counting it.

**What four calls would have cost, and why he turned it down:** four calls is
four latencies, four budgets and four chances to fail, and the first three
answers would each be a decision taken without the reading the fourth is about
to do. It also makes the proposer four things instead of one, and the thing it
is — a Policy, not an Agent, with no toolset and nothing it can transition — is
what keeps it cheap.

**Cost he took**, in the words it was put to him in: reading a half-written
answer is fragile, the field order becomes a contract with the model, and a call
that dies halfway leaves a Job with a workflow and no title.

## What is done about the half-written answer

Named here because the cost above is real and is paid, not avoided.

| The risk | What holds it |
|---|---|
| Half a field on screen | A field settles only once its own line has ended. A newline is what says a value will not change |
| A field drawn that the finished answer contradicts | The prefix is re-read whole each time rather than folded, and one test asserts what settled equals what the plan ends up holding |
| A workflow nothing holds | Only a workflow this repository holds ever settles — the same rule that refuses the finished answer, one field early |
| A call that dies after the workflow | `proposing -> escalated`, with the workflow kept and no title. The row's title is still the request, so it reads as words somebody wrote rather than as a blank |

## What the fourth field holds, which took a second question

*Settings* is a block with several dials on it, and the four-field answer did not
say which of them this call writes. The 23 September ruling had already named
three — a model, an urgency and a land-as-one switch — as things *"the proposer
writes for you"*. Asked which of the three it actually decides:

| Setting | Answer | Why |
|---|---|---|
| Urgency | **It writes it** | A fact about the request itself: something is broken for people right now, or it is not |
| Model | **It writes it**, over an argument against | Below |
| Land as one | **It does not**, on an older ruling he kept | Below |

### The model, taken over the objection

It was put to him that the model is the dial deciding what every later call
costs, and that letting the proposer pick it is a model choosing which model runs
the work. **He chose it anyway.** The guardrails he took with it:

- **It picks from what this Fleet holds**, never a name it invents. The question
  carries `list_models`' own set and the answer names one of them.
- **A name that set does not hold is refused as a workflow id that is not** —
  `Unresolved::NotHeld`, the request comes back — rather than nearest-matched or
  given a second shape of wrongness.
- **Absent stays absent.** A call that names no model reaches `ProposeJob.model`
  null, which has always meant configuration decides, and never a default this
  call picked.

### Land as one, ruled out on a decision he kept

`atomic` was asked for in the same breath and was not taken. Put beside the
model with both rulings in view, he kept **3 September 2026**:
`docs/concepts/job-proposer.md`'s *Scope is not among them* says *"How the work
lands is not among them either… this call has read no code"*, and
`crates/fleet/src/proposal.rs`'s own header records that four documents were
corrected against that module rather than the other way round.

**Why the two settings split where they do**: urgency and the model are facts
about the request and about this machine, which this call can read; how the work
lands follows from having read the code, which it has not. `write_targets` is
out for the same reason and always was.

**Cost taken:** land-as-one is the one field on the dispatch card with no
proposed value to react to. The 23 September rule itself is unchanged — every
decision is still overridden on the Settings block — and that record now carries
a dated line saying which of its three the proposer writes.

## The title changes under a reader, and that was asked for

It is the second field, so the row's title stops being the request while he may
be reading it. **What keeps it honest is that nothing he typed leaves the
screen**: the moment the title replaces the request, the request becomes the
Job's brief, which is where Fleet puts it when the call answers anyway. On the
Job's page the header changes and the words appear one region down; on the Board
the row's title changes in place.

## What this reverses, and what it does not

`crates/ipc/src/proposing.rs` said of `answered_characters`: *a count and never
the text — what the proposer decided arrives as the Jobs it minted, and a
channel carrying the answer as it was written would be a second, earlier, worse
copy of that.*

**That was correct when it was written.** Nothing existed until the answer
landed, so anything read early would have been a rival to the real thing.

**The premise changed, not the reasoning.** A dispatched request is a Job from
the press, so a field read early is not a second copy of that row — it is that
row becoming more complete. **The objection still stands for the text itself**,
which is why what crosses the wire is fields that are settled and never a
transcript, and why the count is still a count. The DTO carries the old sentence
with its date beside the new one.

## What is still open

The model refusal is a whole-request refusal: a proposer that spells a model
slightly wrong refuses the dispatch, and the sentence a person reads for it is
`fleet.no_workflow_fits`'s — *no workflow in this repository fits that request,
rephrase it and dispatch again* — which is not what happened and not what would
fix it. That follows directly from reusing the workflow's own refusal rather
than minting a second shape, which is what was asked for. Worth a look once the
proposer has run against a real catalogue.

## What is owed

Fleet publishes `settled` and nothing folds it onto a Job yet, because
`job-statuses.toml` still reads `in_code = "Not yet"` for `proposing` — no Fleet
creates a Job at dispatch, so `proposal.moved` names no Job to fold onto. The
mock is what mints the row and applies the fold. When Fleet's half lands,
`apps/desktop/src/main/arrivals.ts` calls the same fold on the Job the message
names.
