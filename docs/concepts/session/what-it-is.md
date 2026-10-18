# What it is

**Part of:** [Job](../session.md).

> **Rule.** A job from a terminal is a first-class row, the same shape a job Bridge hosts will have.
> Why: the question is who holds a thing, and it should not depend on where the job was started.

> **Rule.** A job is reported as it happens and is never read back from.
> Why: the third attempt in [scope](../../scope.md) was a conversation read as a claim. The ledger keeps what a job took and did, as structure, and nothing downstream reads a transcript.

> **Rule.** Fleet never waits on a job and a terminal job never waits on Fleet.
> Why: a terminal job is the person's own, and Fleet being down, slow or gone must cost it nothing. A job Fleet hosts is Fleet's to run, so its write waits on Fleet's hook, and what the CLI does with a hook Fleet does not answer was not measured.

Until the ledger Fleet could not see a job a person started. [Helm](../helm.md) is one of these now: Armada's own conversation with Fleet is a hosted job per repository, marked `helm`.
