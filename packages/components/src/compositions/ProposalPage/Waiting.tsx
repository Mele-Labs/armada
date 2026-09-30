import { useState } from "react";

import { Button } from "../../primitives/Button/Button";

/**
 * What the call is doing, while it does it.
 *
 * **Every number here is already resolved by the caller.** Elapsed is a
 * subtraction against a clock, and a component that read one would tick on its
 * own schedule and disagree with every other elapsed figure on screen.
 */
export type ProposalWatch = {
  /**
   * How far the call has got. `starting` is **the one worth telling apart**: a
   * call still there after a minute never reached the vendor at all, which will
   * not resolve by waiting.
   */
  reached: "starting" | "started" | "requesting" | "thinking" | "answering";
  /** How long the call has been out, in milliseconds. */
  elapsedMs: number;
  /** Fleet's own ceiling for this call, in milliseconds. */
  budgetMs: number;
  /** Which model is reading it. What the wait costs, roughly. */
  model: string;
  /**
   * The harness's running estimate of how much the model has thought. **Drawn
   * as an approximation**, because that is what it is.
   */
  thinkingTokens?: number;
  /** How much of the answer has arrived, in characters. */
  answeredCharacters?: number;
};

/**
 * The wait, and what to do about it.
 *
 * **Two registers, and the mark is what chooses between them**: before it the
 * wait says what the call is doing, after it the screen says so rather than
 * leaving somebody to wonder. **The stop is in both** — a screen somebody was
 * sent to in order to watch one call carries the only act on it from the first
 * frame.
 *
 * **Nothing here ticks.** Every figure is resolved by the caller against one
 * clock, so this and the rest of the window cannot disagree.
 */
export function Waiting({
  watch,
  onStop,
  slowAfterMs,
}: {
  watch?: ProposalWatch;
  onStop?: () => void;
  slowAfterMs?: number;
}) {
  // The screen unmounts this the moment `reading` ends, however it ends — so a
  // fresh wait always starts unpressed. #1117.
  const [stopping, setStopping] = useState(false);
  const stop =
    onStop === undefined ? null : (
      <Button
        variant="secondary"
        pending={stopping}
        onClick={() => {
          setStopping(true);
          onStop();
        }}
      >
        {stopping ? "Stopping…" : "Stop the proposer"}
      </Button>
    );

  // Nothing read yet, and no mark to have passed: an older Fleet, or the moment
  // before the first message lands.
  if (watch === undefined) {
    return (
      <div className="armada-proposal-page__wait" role="status">
        <p className="armada-proposal-page__waiting">
          The proposer is reading the request. It answers once, whole.
        </p>
        {stop}
      </div>
    );
  }

  const slow = slowAfterMs !== undefined && watch.elapsedMs >= slowAfterMs;
  const left = Math.max(0, watch.budgetMs - watch.elapsedMs);

  return (
    <div className="armada-proposal-page__wait" role="status">
      <p className="armada-proposal-page__wait-head">
        <span className="armada-proposal-page__wait-what">{REACHED[watch.reached]}</span>
        <span className="armada-proposal-page__wait-for">{lasting(watch.elapsedMs)}</span>
      </p>
      {/* The ceiling is what makes the elapsed figure mean anything: against
          nothing it can only say "slow", and against the budget it says how
          much of the decision is left. */}
      <p className="armada-proposal-page__wait-where">
        {watch.model} · {left === 0 ? "out of time" : `${lasting(left)} left`}
      </p>
      {/* Absent rather than zeroed: a call that has not started thinking and
          one thinking about nothing are different things, and `0` draws them
          the same. */}
      {watch.thinkingTokens === undefined ? null : (
        <p className="armada-proposal-page__wait-count">
          about {watch.thinkingTokens.toLocaleString()} tokens of thinking
        </p>
      )}
      {watch.answeredCharacters === undefined ? null : (
        <p className="armada-proposal-page__wait-count">
          {watch.answeredCharacters.toLocaleString()} characters of answer so far
        </p>
      )}
      {/* **No `Keep waiting` control**, and the absence is the design: waiting
          is what happens if nothing is pressed, and a button for it would
          perform no act. */}
      {slow ? (
        <p className="armada-proposal-page__wait-ask">
          This is taking longer than expected. It is still running — waiting is reasonable,
          and so is stopping.
        </p>
      ) : null}
      {stop}
    </div>
  );
}

/**
 * What each reach is called on screen.
 *
 * **`starting` is the one that says something is wrong.** A call that has not
 * announced itself never reached the vendor, so its sentence names the harness
 * rather than the model — the reading a person needs in order to stop rather
 * than wait.
 */
const REACHED: Record<ProposalWatch["reached"], string> = {
  starting: "Starting the proposer",
  started: "Waiting to reach the model",
  requesting: "Asking the model",
  thinking: "The model is thinking",
  answering: "The answer is arriving",
};

/**
 * A duration, in the coarsest unit that is still true. Seconds under a minute,
 * then minutes and seconds — a wait is read at a glance, and `142s` is a number
 * somebody has to divide.
 */
function lasting(ms: number): string {
  const seconds = Math.max(0, Math.round(ms / 1000));
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  const rest = seconds % 60;
  return rest === 0 ? `${minutes}m` : `${minutes}m ${rest}s`;
}

