// What `proposeFromRequest` answered, read into the one thing the app still has
// to do about it.
//
// **The press leaves the composer**, so no surface waits on this answer: the
// Jobs the request became are rows on the Board, and a Job still being read is a
// row at `proposing` carrying the request as its title.

// # Four answers, and only two of them say anything
//
// `ok` says nothing: every Job it names exists, and the Board is where they are.
// `stopped` says nothing either — somebody pressed the stop this app offered
// them, and `crates/fleet/src/refusing.rs` declares that code apart so a client
// does not draw a person's own press as Armada breaking.

// `unresolved` is Fleet reading the request and declining: no workflow fits and
// no Job was created. **It is not an error** — no code and no red, because
// Armada worked — so it is told rather than raised.

// `faulted`, and a refusal of a command before any of this, are failures. They
// go back as an `Outcome` and the app draws them where it draws every other one,
// so a proposer refused for being disconnected reads exactly like an approval
// refused for it. `renderer/src/failing.ts` is what turns the `WireError` inside
// a fault into something quotable.

import type { ProposalWatch } from "@armada/components";
import type { Outcome, ProposalInFlight, ProposalSettled, Proposed } from "@armada/protocol";

export type { Proposed };

/** What the app has to do about an answer nothing is waiting for. */
export type Answered = {
  /** A failure the app draws in its own pipeline, or `null` where none is. */
  outcome: Outcome | null;
  /**
   * A sentence to tell somebody, or `null`. **Told and never raised**: the one
   * answer that reaches here is Fleet declining to find a workflow, which is
   * Armada working and must not wear the error treatment.
   */
  told: string | null;
};

/**
 * What a decline says. **Its own sentence rather than the wire's**: the code is
 * `fleet.no_workflow_fits`, `said()` has no arm for it, and a notice built from
 * the `Outcome` would draw a red card with Fleet's own prose in it.
 */
const NO_WORKFLOW_FITS =
  "No workflow in this repository fits that request, so no Job was created. " +
  "Rephrase it and dispatch again.";

/**
 * What a model nothing holds says. **The sentence above is the one it must not
 * say**: the request was read, the workflow was fine, and rephrasing cannot
 * change which models this machine runs. It names what was asked for and what
 * is available, both off the envelope's own fields, so the next act is on
 * screen rather than somewhere to go and look up.
 */
function modelUnavailable(outcome: Outcome): string {
  const fields = outcome.ok ? {} : "error" in outcome ? outcome.error.fields : {};
  const asked = fields["model"];
  const held = fields["models"];
  const named = typeof asked === "string" && asked !== "" ? `\u201c${asked}\u201d` : "a model";
  const runs =
    typeof held === "string" && held !== ""
      ? ` This machine runs ${held}.`
      : " This machine names no model at all.";
  return (
    `The proposer asked for ${named}, which this machine does not run, so no Job was created.` +
    `${runs} Dispatch again once it is available, or pick a model yourself.`
  );
}

export function answeredAs(answer: Proposed): Answered {
  if (answer.ok) return { outcome: null, told: null };
  if (answer.why === "stopped") return { outcome: null, told: null };
  if (answer.why === "unresolved") return { outcome: null, told: NO_WORKFLOW_FITS };
  // Told rather than raised, for the decline's reason: Armada worked, and the
  // machine not holding a model is not a failure to draw in red.
  if (answer.why === "model_unavailable") {
    return { outcome: null, told: modelUnavailable(answer.outcome) };
  }
  return { outcome: answer.outcome, told: null };
}

export { filled, movedOnto } from "@armada/screens/src/filling";

/**
 * How long a proposal may run before the reading asks whether to keep waiting.
 *
 * **A prompt and not a limit.** Nothing happens at this mark: the call keeps
 * running until Fleet's own budget or until somebody presses stop. What it
 * decides is when the question is put in front of a person rather than left for
 * them to wonder about.
 *
 * Two minutes, chosen against the wait it replaced — Bridge used to abort the
 * request at five seconds, and before that a proposal that took this long was
 * simply lost. It is deliberately well inside Fleet's own proposer budget
 * (`PROVISIONAL_PROPOSER_BUDGET`, ten minutes): a question asked as the call
 * dies is not a question, it is an epitaph.
 *
 * **Unmeasured, like the budget it sits inside.** What would settle it is a
 * distribution of real proposal latencies, which nothing collects yet.
 */
export const PROPOSAL_IS_SLOW = 120_000;

/**
 * What Fleet says about the call in flight, as `ProposerWait` draws it.
 *
 * `null` where nothing is out, or where the instant will not read — a wait that
 * cannot say how long it has been is drawn as a wait with nothing known about
 * it rather than as one that has been running since the epoch.
 *
 * **The clock is the caller's.** Elapsed is resolved here, against a `now` the
 * app already ticks for everything else, so this figure and every other elapsed
 * figure on screen come from one reading.
 */
export function watchOf(proposing: ProposalInFlight | null, now: number): ProposalWatch | null {
  if (proposing === null) return null;
  const since = Date.parse(proposing.since);
  if (Number.isNaN(since)) return null;
  return {
    reached: proposing.reached,
    // Never negative. A clock a few milliseconds behind Fleet's would otherwise
    // draw a call that has not started yet.
    elapsedMs: Math.max(0, now - since),
    budgetMs: proposing.budget_ms,
    model: proposing.model,
    ...(proposing.thinking_tokens === undefined
      ? {}
      : { thinkingTokens: proposing.thinking_tokens }),
    ...(proposing.answered_characters === undefined
      ? {}
      : { answeredCharacters: proposing.answered_characters }),
    ...(settledRows(proposing.settled).length === 0
      ? {}
      : { settled: settledRows(proposing.settled) }),
  };
}

/**
 * What the proposer has decided, as rows the wait draws.
 *
 * **The owner's order, and nothing sorts it** — the fields are written in that
 * order, so the rows are built in it: the workflow decides the Job's shape, the
 * title is what makes the row recognisable, done-when is the goal, and the
 * settings are the part he can still change.
 *
 * **A row per criterion rather than a count.** Never draw a count beside the
 * items it counts (29 Sep 2026), and the lines arrive one at a time anyway.
 */
function settledRows(
  settled: ProposalSettled | undefined,
): { label: string; said: string; prose?: boolean }[] {
  if (settled === undefined) return [];
  return [
    ...(settled.workflow_id === undefined
      ? []
      : [{ label: "Workflow", said: settled.workflow_id }]),
    ...(settled.title === undefined ? [] : [{ label: "Title", said: settled.title }]),
    ...(settled.done_when ?? []).map((said) => ({ label: "Done when", said, prose: true })),
    ...(settled.settings?.urgency === undefined
      ? []
      : [{ label: "Urgency", said: settled.settings.urgency }]),
    ...(settled.settings?.model === undefined
      ? []
      : [{ label: "Model", said: settled.settings.model }]),
  ];
}
