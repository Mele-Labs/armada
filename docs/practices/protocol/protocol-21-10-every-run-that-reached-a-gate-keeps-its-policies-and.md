# Protocol 21.10: every run that reached a gate keeps its policies, and says whether they decided

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`StepAttempt.resolved` gains `decided: boolean`, and is now set on every run a gate stopped as well
as on one that reached the advance gate. Additive, on 21.9's argument: an older Bridge ignores the
key, and an older Fleet never sends it.

**The owner's decision of 2 Oct 2026** (#1683): a run whose Checks failed, whose Judge refused, or
whose gaming check flagged it keeps what `auto_merge` and `review_gate` said at that moment too. His
reason: "Isn't it helpful to record them so we know what ran?" 21.9 kept them only where the
advance gate was read.

| Run | `resolved` | `decided` |
|---|---|---|
| Held for review, advanced or finished | set | `true` |
| Checks red (handed back or failed), Judge refused, Judge asking a person, gaming flagged, gate could not decide | set | `false` |
| Submission of the wrong kind, still going, from before 21.9, or an older Fleet | absent | — |

**`decided` is read off the ruling's variant in `fleet`, never carried**, so a run cannot be
recorded as decided when it stopped before the rule. The two words still say what the rules were
at the gate the run stopped at; `decided: false` says the rule never answered for it.

**A 21.9 payload has `resolved` without `decided`, and reads `true`.** 21.9 kept only runs that
reached the advance gate, which is also why V89's column defaults to `1` for every row V87 wrote.
Absent `resolved` still means nothing was recorded, never a default.
