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
  /**
   * What the proposer has decided so far, one row per field, in the order the
   * fields settle.
   *
   * **Resolved by the caller, like every other value here.** What each field is
   * called and how its value reads is Bridge's reading of
   * `ProposalInFlight.settled`; this draws the rows it is handed and knows the
   * names of none of them. Absent is a call that has settled nothing, which is
   * every call before it starts writing.
   */
  settled?: readonly { label: string; said: string }[];
};

/**
 * The wait a Job at `proposing` is in, and the one act on it.
 *
 * **It goes inside the lead**, where a Drone's question and a command it was
 * not given are already answered: `leadOf` says *A model is reading the
 * request* and this says how far the call has got. It is not a destination —
 * the Job's row on the Board is the address, and this is what is found on
 * coming back to it.
 *
 * **Two registers, and the mark chooses between them**: before it the wait
 * says what the call is doing, after it the surface says so rather than leaving
 * somebody to wonder. **The stop is in both.**
 *
 * **Nothing here ticks**, so this and the rest of the window cannot disagree.
 */
export function ProposerWait({
  watch,
  onStop,
  slowAfterMs,
}: {
  /**
   * What Fleet says the call is doing. **Absent is a Fleet that has said
   * nothing about it**, and the moment before the first message lands — the
   * wait is drawn without a reading rather than not drawn.
   */
  watch?: ProposalWatch;
  /**
   * Stop the call. **Kills it rather than stopping the wait** — a wait
   * abandoned leaves the proposer running inside Fleet and spending. Absent
   * draws no control, which is a caller that cannot reach the act.
   */
  onStop?: () => void;
  /** Past this, the wait says so and asks. Absent never asks. */
  slowAfterMs?: number;
}) {
  // The lead unmounts this the moment the Job leaves `proposing`, however it
  // leaves — so a fresh wait always starts unpressed. #1117.
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
      <div className="armada-proposer-wait" role="status">
        <p className="armada-proposer-wait__said">
          The proposer is reading the request. It fills this Job in as it writes.
        </p>
        {stop}
      </div>
    );
  }

  const slow = slowAfterMs !== undefined && watch.elapsedMs >= slowAfterMs;
  const left = Math.max(0, watch.budgetMs - watch.elapsedMs);

  return (
    <div className="armada-proposer-wait" role="status">
      <p className="armada-proposer-wait__head">
        <span className="armada-proposer-wait__what">{REACHED[watch.reached]}</span>
        <span className="armada-proposer-wait__for">{lasting(watch.elapsedMs)}</span>
      </p>
      {/* The ceiling is what makes the elapsed figure mean anything: against
          nothing it can only say "slow", and against the budget it says how
          much of the decision is left. */}
      <p className="armada-proposer-wait__where">
        {watch.model} · {left === 0 ? "out of time" : `${lasting(left)} left`}
      </p>
      {/* Absent rather than zeroed: a call that has not started thinking and
          one thinking about nothing are different things, and `0` draws them
          the same. */}
      {watch.thinkingTokens === undefined ? null : (
        <p className="armada-proposer-wait__count">
          about {watch.thinkingTokens.toLocaleString()} tokens of thinking
        </p>
      )}
      {watch.answeredCharacters === undefined ? null : (
        <p className="armada-proposer-wait__count">
          {watch.answeredCharacters.toLocaleString()} characters of answer so far
        </p>
      )}
      {/* What has been decided, as it is decided. **Inside the wait and not a
          region of its own**: the wait is the one place on this Job's page whose
          subject is the call, and each of these is already on the Job itself —
          the row's Workflow column, the row's title, the Job's criteria, the
          Job's urgency. This says which of them the proposer has got to, which
          is the fact the wait is for.

          Each row appears as its own line ends, so a field is drawn whole or
          not at all. */}
      {watch.settled === undefined || watch.settled.length === 0 ? null : (
        <dl className="armada-proposer-wait__settled">
          {watch.settled.map((one, at) => (
            <div className="armada-proposer-wait__field" key={`${one.label}-${at}`}>
              <dt>{one.label}</dt>
              <dd>{one.said}</dd>
            </div>
          ))}
        </dl>
      )}
      {/* **No `Keep waiting` control**, and the absence is the design: waiting
          is what happens if nothing is pressed, and a button for it would
          perform no act. */}
      {slow ? (
        <p className="armada-proposer-wait__ask">
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
