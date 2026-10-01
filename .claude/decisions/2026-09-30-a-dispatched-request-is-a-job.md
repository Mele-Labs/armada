# What is a dispatched request, before the proposer answers?

**Decided 2026-09-30.**

He dispatches a request, the Job proposer reads it, and until it answers there
is nothing anywhere — no row, no address, nothing to come back to. The reading
happened inside the dispatch dialog, so the only way to watch it was to sit on
the screen.

Asked what should happen instead, after a first attempt built him a destination
to wait on:

> The proposer was a complete miss. I want it to be like a job. Something where
> I can propose multiple things at once and they go off and get proposed. I
> dont need to sit on the screen and watch it. Thats why i wanted it like a job
> so that I could come back to each of them.

**The miss was in the question, not the build.** All four options offered were
about *where the waiting happens*. He had said twice he did not want to wait.

## It is a Job, from the moment Dispatch is pressed

Offered a Job created at dispatch, or the board drawing proposals as a second
kind of row:

**Chosen: Fleet makes it a Job.** A row on the board immediately, with the
request as its title. Everything that already lists Jobs lists these — the
board, search, Stats — with no second concept to learn.

**Cost he took:** a new status in the registry, and for a while a Job with no
workflow, no steps and no plan, which every reader has to draw without looking
broken.

## The objection that was wrong

It was put to him that one request can become several Jobs, so there is no
*the* Job to open. That was not true. `JobSummary.dispatched_by` already exists
and is how a Job names the Job that dispatched it — the train Overview draws.
The request becomes one Job and the extras are its members. The answer was in
the codebase while the argument was being made against it.

## What the status says, and what it does not

`proposing`: `mode = "Working"`, `who_is_acting = "None"`. Working because the
money is being spent while the row sits there; None because the three values
name what a surface can address, a person to prompt or a Drone to poke, and
neither exists — the actor is a model call in Fleet's own process.

**`Person` would have put every dispatch on Needs you the moment it was sent**,
which is the reading that combination exists to avoid.

**It is not a planned-but-not-dispatched Job.** Asked whether an Epic's next
wave could use it, the answer is no on both fields: nothing is running on a
planned child and it is waiting for a person. `awaiting_approval` already says
that, and `propose_from_request` already answers with several Jobs at that
status at once. What such a wave needs is not a status but a way to approve
several in one act, which `approve_dispatch` — *"Releases a Job to spawn"* —
does not offer.

## What it says about him

He asked for the same thing three times in three sessions and was answered with
a screen twice. What he was describing each time was an **address**: a thing
with a place in a list that survives him closing the window. "Like a job" was
not a comparison, it was a specification.

**Where it lands:** the registry and the wire on
`registry/a-dispatched-request-is-a-row`, protocol 19.0. Fleet does not serve
it — `in_code = "Not yet"`, like `awaiting_approval`, `awaiting_repair` and
`awaiting_attestation` beside it, and the mock is where this milestone builds
first. The decline and the fault cannot fire until two escalation reasons are
written, which are named in the row's `open_questions` and deliberately unminted:
each is a badge word a person reads.
