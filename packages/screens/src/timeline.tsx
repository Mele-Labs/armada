// One run's turns, out of a step's whole record.
//
// **An attempt is the spine.** `StepAttempt` carries the only times a step's
// insides have: when the attempt began and when it ended. Turns carry their own
// instant, so they fall into an attempt's window by their timestamp.

import type { StepAttempt, StepDetail, Turn } from "@armada/protocol";

/**
 * Every run of the step, oldest first.
 *
 * **A step with no attempts still has one.** Fleet sends none until the first
 * is recorded, and a step a person is looking at has begun.
 */
function spineOf(step: StepDetail): StepAttempt[] {
  return step.attempts.length > 0
    ? [...step.attempts].sort((a, b) => a.attempt - b.attempt)
    : [{ attempt: 1, outcome: step.state, started_at: step.entered_at }];
}

/**
 * One run's turns, by the attempt's own window.
 *
 * **For a sheet opened from an earlier attempt.** An attempt ends when the next
 * begins, so its turns are the ones stamped inside that span; unknown attempt
 * is the step's whole record rather than nothing, because a reading that
 * silently empties is worse than one that is wider than asked.
 */
export function turnsOfAttempt(
  step: StepDetail,
  attempt: number,
  turns: readonly Turn[],
): Turn[] {
  const spine = spineOf(step);
  const at = spine.findIndex((one) => one.attempt === attempt);
  const run = spine[at];
  if (run === undefined) return [...turns];
  const ended = run.ended_at ?? spine[at + 1]?.started_at;
  return turns.filter(
    (turn) =>
      (turn.step === undefined || turn.step === step.step_id) &&
      turn.ts >= run.started_at &&
      (ended === undefined || turn.ts < ended),
  );
}
