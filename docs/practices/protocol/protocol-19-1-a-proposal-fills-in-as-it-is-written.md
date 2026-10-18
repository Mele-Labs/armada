# Protocol 19.1: a proposal fills in as it is written

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`ProposalInFlight.settled` and the two DTOs behind it, `ProposalSettled` and
`ProposalSettings` — additive on a message that already existed, so the minor
moves. A Bridge built before this ignores the field and draws the wait exactly as
it did at 19.0.

The owner, 30 Sep 2026, having looked at a proposing Job on screen: *"I would
really push for us to find a way to make the proposer not report the job whole.
Is there anyway for it to fill in as it goes?"* Four fields, in his order —
workflow, title, done-when, settings — and
`.claude/decisions/2026-09-30-a-proposal-fills-in-as-it-is-written.md` carries the
order's reasoning, the option he turned down and the cost he took.

**The settings are one field carrying two**, because both arrive on one line of
the answer: `urgency`, and `model` — which model a Drone on this Job is spawned
as. The owner took the model over the argument that a model choosing which model
runs the work is the dial every later call's cost hangs off. **It picks from
`list_models`' own set**, and a name that set does not hold refuses the request
through `fleet.proposer_model_not_held` — **its own code, and never
`fleet.no_workflow_fits`**. That refusal's advice is to say the request again
differently, which cannot fix a model name and is about a workflow that was not
wrong; `#334` and `#410`'s rule is that two causes wanting opposite responses
must not share a word. The refusal carries the model asked for and the set this
machine runs, and Bridge mirrors the code the way it mirrors
`fleet.proposer_stopped`.
**Absent stays absent**: a call that names no model reaches `ProposeJob.model`
null, which has always meant configuration decides. Land-as-one is deliberately
not here — the `proposal` module (deleted since)'s header carries the 3 Sep 2026 ruling
that how the work lands follows from having read the code, and the owner kept it
when it was put beside the model.

**Fields that are settled, never a transcript.** Fleet reads the answer's prefix
in one place (the `proposing` module (deleted since), `Settled::of`), and a field
crosses only once its own line has ended — so a client either has a title or has
none, and never has `Say which of the two giv`. The raw text does not cross:
`answered_characters` is still a count, and `crates/ipc/src/proposing.rs (deleted)` carries
the dated correction of the rule that used to make it the whole of what a surface
could say.

**What the correction turns on is the premise, not the reasoning.** *A channel
carrying the answer as it was written would be a second, earlier, worse copy of
the Jobs it minted* was right while nothing existed until the answer landed. A
dispatched request is a Job from the press since 19.0, so a field read early is
that row becoming more complete rather than a rival to it — and the objection
still stands for the text itself, which is why nothing beside the count carries
one.

**Bridge draws each field where the Job already draws it**, through one fold
(`filled`, in `packages/screens/src/proposal.ts (deleted)`): the settled workflow is the
row's Workflow column, the settled title is the row's title, the done-when lines
are the Job's criteria and the settings are its urgency. The wait inside
Overview's lead says which of the four the call has got to, because that is the
one region on a proposing Job's page whose subject is the call.

**Fleet publishes it and nothing folds it yet.** `job-statuses.toml` still reads
`in_code = "Not yet"` for `proposing`, so no Fleet creates a Job at dispatch and
`proposal.moved` names no Job for `arrivals.ts` to fold onto. The mock is what
mints the row and applies the fold; when Fleet's half lands, `arrivals.ts` calls
`filled` on the Job the message names and nothing in `packages/` changes.

**It makes the back-pressure question neither better nor worse.** No new queue,
no new channel and no per-client state: `settled` rides on `proposal.moved`, which
is already on the one bounded drop-oldest broadcast, and a dropped message costs
a reading that the next one supersedes — the field is the whole prefix re-read
rather than a delta, so a client that lost one is not missing a field, it is a
beat behind. What it does add is messages: a field settling publishes
immediately rather than waiting for `TOKEN_TICK`, which is four more messages per
one-Job answer plus one per done-when line. That is bounded by the answer's own
shape and not by the frame rate, which is the property `TOKEN_TICK` exists to
hold.
