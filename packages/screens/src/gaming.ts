// A step's gaming check, read once for the rail and the step panel. #1079.
//
// **Its own reading, never a tier's.** The gaming check is counted in neither
// Checks nor Judge on purpose — a flag is not a verdict, and a recount would
// make a green tier read red — so nothing here touches `gates.ts`' counts.
//
// **A flag a second reading cleared is still a flag**, and it never holds a
// step: it is drawn in the panel as cleared, with why, and nowhere else.

import type { Flagged, StepDetail } from "@armada/protocol";

import { onlyCurrentAttempt } from "./facts";
import { didNotPass, NOT_REACHED } from "./gates";

/** One attempt's flags, split into those that hold the step and those cleared. */
export type FlagsRead = { held: Flagged[]; cleared: Flagged[] };

/** The newest attempt's flags, or the rows a caller already narrowed. */
export function flagsOf(step: StepDetail, rows?: readonly Flagged[]): FlagsRead {
  const flags = rows ?? onlyCurrentAttempt(step.flagged);
  return {
    held: flags.filter((flag) => flag.cleared === undefined),
    cleared: flags.filter((flag) => flag.cleared !== undefined),
  };
}

/** Whether the step declares a gaming check at all. */
export function declaresGaming(step: StepDetail): boolean {
  return (step.judge_checks ?? []).some((judge) => judge.gaming_check);
}

/**
 * Every pattern the step's gaming check looks for, in the order declared, or
 * `undefined` from a Fleet that does not say — which reads as the flags alone.
 */
export function declaredPatterns(step: StepDetail): string[] | undefined {
  const judges = step.judge_checks ?? [];
  const spelled = judges.filter((judge) => judge.gaming_patterns !== undefined);
  if (spelled.length === 0) return undefined;
  return [...new Set(spelled.flatMap((judge) => judge.gaming_patterns ?? []))];
}

/**
 * Whether the gaming check ran on the step's newest attempt.
 *
 * **Read off the record, never assumed.** It ran where it flagged anything,
 * where the step advanced, or where the gate ruled with every Check passing —
 * a Check that failed stops the gate before the gaming check is asked.
 */
export function gamingReached(step: StepDetail): boolean {
  const attempt = step.attempts.at(-1)?.attempt;
  const mine = <T extends { attempt: number }>(rows: readonly T[]): T[] =>
    attempt === undefined ? [...rows] : rows.filter((row) => row.attempt === attempt);
  if (mine(step.flagged).length > 0 || step.state === "advanced") return true;
  return mine(step.verdicts).length > 0 && !mine(step.check_runs).some(didNotPass);
}

/** `1 flag`, `2 flags`. */
function flagsSaid(count: number): string {
  return `${count} ${count === 1 ? "flag" : "flags"}`;
}

/**
 * The rail's `Gaming check` fact — `1 flag · stopped here` — or `undefined`
 * where nothing was flagged. **Replaces `Verdict · evidence disputed`**, which
 * named the trigger and not the check that pulled it.
 *
 * `stopped` is whether these flags are what holds the step now; a step a
 * person carried on past still carries them, and says only how many.
 */
export function gamingStands({ held, cleared }: FlagsRead, stopped: boolean): string | undefined {
  if (held.length > 0) return stopped ? `${flagsSaid(held.length)} · stopped here` : flagsSaid(held.length);
  if (cleared.length > 0) return `${flagsSaid(cleared.length)} cleared`;
  return undefined;
}

/**
 * What the step panel's Gaming check row says folded — `1 of 6 flagged ·
 * stopped the step`.
 *
 * **Against every declared pattern where Fleet says which**, counting patterns
 * flagged rather than flags, and against nothing where it does not: an older
 * Fleet sends only whether a gaming check is declared, and a denominator then
 * would be invented.
 */
export function gamingSummary(
  { held, cleared }: FlagsRead,
  reached: boolean,
  stopped: boolean,
  declared?: readonly string[],
): string {
  const any = held.length + cleared.length > 0;
  if (!any && !reached) return NOT_REACHED;
  const parts = [
    ...(declared !== undefined
      ? [`${new Set(held.map((flag) => flag.pattern)).size} of ${declared.length} flagged`]
      : held.length > 0
        ? [`${held.length} flagged`]
        : []),
    ...(cleared.length > 0 ? [`${cleared.length} cleared`] : []),
  ];
  if (held.length > 0 && stopped) parts.push("stopped the step");
  return parts.length > 0 ? parts.join(" · ") : NOTHING_FLAGGED;
}

/** What a gaming check that ran and found nothing says. */
export const NOTHING_FLAGGED = "nothing flagged";

/** What a declared pattern the check did not find says on its row. */
export const NOT_SEEN = "not seen";
