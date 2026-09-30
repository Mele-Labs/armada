# Whose evidence does the lead say a review gate found?

**Decided 2026-09-30.** The counting half of
`2026-09-30-the-lead-says-the-thing-and-stops.md`.

On `job/reviewAtDelivery` the lead read **Waiting for your review** over an
empty second line. The clause that should fill it is built from the waiting
step's own Checks and criteria, and the waiting step is `Land`: it pushed a
branch and opened a pull request, and `docs/concepts/job.md` says a delivering
step verifies nothing of its own. So a person about to approve a branch was told
something was waiting and nothing about whether the work was any good.

Offered the step's own evidence — which is empty here and correctly so — or the
whole Job's.

## The whole Job's, because what is signed off is the branch

| | Now | New |
|---|---|---|
| Clean | `Waiting for your review` over nothing | `All 3 Checks passed and the Judge met both criteria` |
| Argued with | the same, over nothing | `All 3 Checks passed and the Judge met 1 of 2 criteria` |

**Cost he took, in the words it was put to him in:** *a bigger claim to get
right. A Check that failed and was retried, or a step that was overruled, both
have to count honestly or the sentence lies at the one moment you are trusting
it.*

## What honest counting came to

| The case | How it counts | Why |
|---|---|---|
| A Check retried | Once, on the latest attempt with rows | The gate records every declared Check on every attempt, so the latest attempt is a whole answer. `onlyCurrentAttempt`, the reading `checksOf` already takes |
| A skipped Check | Not at all, in neither figure | It advances the step and measured nothing. Counted as a pass it claims a verification that never happened; counted as a failure it claims one that never failed |
| An outcome this build has no row for | Not at all | Defaulting an unknown outcome into a pass is the one direction a sign-off may not fail in |
| The same criterion on two steps | Once | `panelsOf` answers per step, so counting panels would claim more criteria than the frozen list holds |
| A criterion refused anywhere | Never met | Conservative on purpose: the alternative — the latest step wins — can turn a refusal into a pass |
| A step a person overruled | Its refusal still counts | An override leaves the Judge's rows as they were and is refused where a Check failed, so nothing here launders a human decision into a machine one |
| A step that never ran | Nothing | It has no runs, and a declared Check nothing ran is not evidence |

**A Job with nothing to show draws no second line**, per `default-to-no-text`.

## What cannot be reached, and is written anyway

`10 of 11 Checks passed` is built and no Job Fleet serves can reach it: a step
opens a human gate only once every tier it declared has held
(`core-model`'s step machine), and an override is refused where a Check failed
(`fleet::overruling`). The clause is written because the alternative is a count
that says **All 11 Checks passed** over one that did not, the day either of
those two facts moves.

**Where it landed:** `metSaid` and its two clauses in
`packages/screens/src/lead.ts`, `didPass` in `packages/screens/src/gates.ts`,
`reviewAfterAnOverrule` in `packages/screens/src/fixtures/build/delivering.ts` —
the only Job on any roster carrying a retried Check and an overruled criterion
behind a branch that is out. A claim per rule in `lead.test.ts`, and the line's
presence measured off the DOM in `overview-boards.test.tsx`.
