# What does a job's Overview lead with?

**Decided 2026-09-29.**

Overview was drawing the job's configuration — the frozen proposal, the gates,
the tiers, the cap — which never changes after approval. He said it should draw
what is happening instead, and listed five things: what is running, what has
failed and needs review, questions a human needs to answer, refusals a human
needs to review, and a summary to review a job by. The symptom he hit first:
*"Looking at group failed, the failed group is buried so deep its not even
obvious that job has a failing group."*

Five equal lists is another wall to read. Asked whether Overview leads with one
thing or shows the five as peers.

## It leads with one thing

*"It should lead with one thing. Something waiting for me to respond is highest
priority because it might unblock a failure."*

That reasoning is the rule, not only the ordering it produced — **a thing you
can act on outranks a thing you can only watch**, because acting may resolve the
watching:

| | |
|---|---|
| 1 | waiting on you — a question, a refusal, an approval |
| 2 | failed — needs review, but answering (1) may already fix it |
| 3 | running — you can only watch |
| 4 | nothing |

## Two things waiting at once

A Judge refusal and a Drone question, together. Offered the oldest first, a
fixed order by kind, or ranking by what each releases:

**Chosen: the one blocking the most work.** The lead names it and says what it
is holding up (*"Nothing in group 4 starts until you answer"*); the rest sit
under it as a short list to jump to.

**Cost he took:** Armada does not have the input. It knows what a group boundary
blocks, because the boundary is what stops the next group. It does not reliably
know what a Drone's question blocks — a question is asked mid-task and nothing
records which other work is waiting behind the answer. Until that exists the
screen falls back to the newest, so some of the ranking is unbuilt and the lead
will sometimes be the wrong one of two.

## Nothing waiting, nothing failed

Offered what is running, a forward look at the next gate, or collapsing the slot
so its presence is itself the signal:

**Chosen: what is running** — the group, how far through, which Drone is on
what. The slot keeps its place and loses its button.

**Cost he took:** the lead then repeats what Plan already draws, and a slot
built around an act draws with nothing to press. He turned down the forward look
(*next: you review group 3, after 3 more tasks*) though it was buildable today —
`human_always` is declared per step in `crates/config/src/workflow/step.rs:68`
and the workflow is frozen onto the job, so which step will next stop for a
person is already known; when it arrives is not.

## What it says about him

He ranks by consequence, not by recency and not by kind, and he will take a
screen that is right later over one that is predictable now. Twice in one
conversation he chose the arrangement that says what a thing costs over the
arrangement that is cheaper to compute.

**Where it lands:** the Overview reframe, unfiled. Its other half is
`2026-09-29-settings-holds-the-frozen-configuration.md` — Overview stops holding
the configuration, and Settings takes it, editable before approval and a reading
after.
