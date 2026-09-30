# How does Overview's lead say what it says?

**Decided 2026-09-30.** Three notes in one sitting, all on `JobLead`.

On `Land is waiting on you to approve it.` his whole note was four words:

> "Waiting for your review"

On the line under it, `Nothing in this workflow is behind it.`:

> "This makes no sense. I hate this kind of AI verbiage. Its getting on my
> fucking nerves"

And on `0 of 8 tasks are through.`:

> "Again this makes no sense. What does it mean to say 0 of 8 tasks are through?
> Shouldnt it be more like 8 tasks are planned or 0 of 8 tasks started?"

Offered the headline alone or both lines. **Chosen: both**, against this:

| | Now | New |
|---|---|---|
| Headline | `Land is waiting on you to approve it.` | `Waiting for your review` |
| | `A Drone wants to run a command it was not given.` | `A Drone wants to run a command` |
| | `cargo_nextest failed on Regression check.` | `cargo_nextest failed` |
| | `On Implement, 4h in. Nothing needs you.` | `Implementing · 4h in` |
| Second line | `Nothing in this workflow is behind it.` | nothing — the slot stays empty |
| | `0 of 8 tasks are through.` | `8 tasks planned, none started` |
| | `5 of 8 tasks are through.` | `5 of 8 tasks done` |

**The headline names the thing and stops** — no step it is happening on, no
verb phrase, no full stop.

**The second line carries facts or it is empty**, which is `default-to-no-text`
applied to a clause. The test is whether the clause asserts something: *Nothing
in this workflow is behind it* and *No step after this one stops for you* are
both the absence case of a clause whose whole job is to name what is there, so
both go. *Write tests and 2 more do not start until you answer* names something
and stays.

**Cost he took:** a reader who wants to know what a running Job's step is
called reads the headline for it, and the four escalated leads lose the
reassurance that nothing else on the Job is burning a slot.

## One sentence was wrong, not wordy

A Job at `awaiting_repair` read *Regression check is waiting on you to approve
it* under a badge saying **Needs repair**. Nothing is being approved:
`job-statuses.toml` says a step spent its gate-failure retry budget and the
work is unfinished. It reads `Out of retries` now, over the Check that failed.

## What is not a gerund

The preview's `Implementing` is not built. A step's label is whatever the
workflow's author wrote, and no registry carries a verb form of it, so
`Implement · 1h 58m in` is the built shape — the same arrangement, without this
file inventing English out of a string off the wire.

**Where it landed:** `packages/screens/src/lead.ts`, and
`packages/screens/src/lead.test.ts` holds a claim per sentence.
