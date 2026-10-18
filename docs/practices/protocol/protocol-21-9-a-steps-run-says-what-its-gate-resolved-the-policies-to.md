# Protocol 21.9: a step's run says what its gate resolved the policies to

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

One optional field on `StepAttempt`, `resolved`, holding `auto_merge` and `review_gate` as
`armada.yml` writes them. It is set on a run whose ruling read the advance gate (advanced,
finished, or held for review) and absent on every other. Additive, on 18.4's argument: an older
Bridge ignores the key, and an older Fleet never sends it.

**History, beside 18.4's live reading** (#1683). `ManifestSummary`'s two words say what the
repository says now. This says what one gate acted on, written to `job_step_policies` on that run
before the step moved. A Manifest edited afterwards changes the next run's value and leaves this
one as it was. That is what lets the Record say a step held because the repository said
`human_always` at the time.

**Both policies, on every gate that read them.** The owner decided this on 1 Oct 2026: not only the
one that gated. One object rather than two optional words, because they are written together and
neither can be there without the other.

**Absent means nothing was recorded, never a default.** Four causes give the same reading: a run
still going, a run stopped before the gate (a Check failed, the Judge refused, a gaming flag
stood), a run from before 21.9, or an older Fleet. None of them is a resolution, so Bridge draws
no policy rather than `never` or `human_always`. It is `#[serde(default, skip_serializing_if)]
Option` on Fleet's side and `?:` on Bridge's.
