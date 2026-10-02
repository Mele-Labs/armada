// What the repository said, on a run whose gate asked it. #1683.
//
// **Only where the step defers to the repository** (`manifest_rule:*`). Fleet
// records both policies on every run that reached a gate, but on a step gated
// `human_always` the repository decides nothing, and naming it would give the
// run a reason it did not have.
//
// **A run that ended before the advance gate says so in the same breath**, the
// owner's call of 2 Oct 2026: naming the rule alone would read as the rule
// having failed the run, when its Checks, the Judge or a gaming check did.
//
// **Absent draws nothing**: a run from before 21.9, or one that reached no gate.

import { ADVANCE_GATE, AUTO_MERGE } from "@armada/components";
import type { StepAttempt, StepDetail } from "@armada/protocol";

/**
 * `the repository said a person answers`, its undecided form, or nothing.
 *
 * The verbs are the registry's: `review_gate` is spelled with the gate's own
 * two words, so `advance_gate`'s rows name it. A word with no verb draws
 * nothing. `decided` absent is a Fleet that recorded only at the advance gate.
 */
export function repositorySaid(
  step: Pick<StepDetail, "advance_gate">,
  run: Pick<StepAttempt, "resolved"> | undefined,
): string | undefined {
  const resolved = run?.resolved;
  if (resolved === undefined) return undefined;
  const verb =
    step.advance_gate === "manifest_rule:review_gate"
      ? ADVANCE_GATE[resolved.review_gate]?.verb
      : step.advance_gate === "manifest_rule:auto_merge"
        ? AUTO_MERGE[resolved.auto_merge]?.verb
        : undefined;
  if (verb === null || verb === undefined) return undefined;
  return resolved.decided === false
    ? `the repository said ${verb}, but the run ended before that gate`
    : `the repository said ${verb}`;
}
