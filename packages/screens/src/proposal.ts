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
import type {
  Criterion,
  JobDetail,
  JobSummary,
  Outcome,
  ProposalInFlight,
  ProposalSettled,
  Proposed,
} from "@armada/protocol";

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

export function answeredAs(answer: Proposed): Answered {
  if (answer.ok) return { outcome: null, told: null };
  if (answer.why === "stopped") return { outcome: null, told: null };
  if (answer.why === "unresolved") return { outcome: null, told: NO_WORKFLOW_FITS };
  return { outcome: answer.outcome, told: null };
}

/**
 * The Job a proposal has settled some of, and its detail.
 *
 * **One function, because a field settles in one place.** What the proposer has
 * decided is the Job becoming more complete — the owner's decision of 30 Sep
 * 2026 that a dispatched request *is* a Job is what makes that true — so a
 * settled field is folded onto the row and the detail rather than drawn from a
 * channel of its own beside them. Every reader on the Board and on the Job's
 * page then draws it with no arm for a proposal at all.
 *
 * **Fleet does not call this yet, and that is `in_code = "Not yet"`.**
 * `job-statuses.toml` says no Fleet creates a Job at dispatch, so there is no
 * row on the far side for `proposal.moved` to be folded onto — the mock is what
 * mints the row and what applies this. When Fleet's half lands,
 * `apps/desktop/src/main/arrivals.ts` calls this on the Job the message names
 * and nothing here changes.
 */
export function filled(
  job: JobSummary,
  detail: JobDetail,
  settled: ProposalSettled | undefined,
): { job: JobSummary; detail: JobDetail } {
  if (settled === undefined) return { job, detail };
  const title = settled.title;
  // **The request is not thrown away when the title replaces it.** Until a
  // title lands the row's title *is* the request as it was typed — which is
  // what `whyNoBrief` says on the page — so the moment the title changes under
  // somebody, the words they wrote move into the brief, which is where Fleet
  // puts them when the call answers anyway. Nothing a person typed stops being
  // on screen because the proposer got further.
  const facts = title === undefined ? detail.facts : (detail.facts ?? job.title);
  const moved: JobSummary = {
    ...job,
    ...(settled.workflow_id === undefined ? {} : { workflow_id: settled.workflow_id }),
    ...(title === undefined ? {} : { title }),
    ...(settled.settings === undefined ? {} : { urgency: settled.settings.urgency }),
  };
  return {
    job: moved,
    detail: {
      ...detail,
      job: moved,
      ...(facts === undefined ? {} : { facts }),
      acceptance_criteria: criteriaOf(settled.done_when) ?? detail.acceptance_criteria,
    },
  };
}

/**
 * The lines the proposer has written, as criteria. `undefined` where it has
 * written none, so a fold never replaces criteria with an empty list.
 *
 * **The Judge is the source, and Fleet writes the same value** — these are
 * prose, and prose is what the Judge reads. The id is the position, because the
 * record has not minted one yet and a Judge citation names a criterion by where
 * it sits.
 */
function criteriaOf(doneWhen: readonly string[] | undefined): Criterion[] | undefined {
  if (doneWhen === undefined || doneWhen.length === 0) return undefined;
  return doneWhen.map((text, at) => ({ criterion_id: String(at + 1), text, source: "judge" }));
}

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
): { label: string; said: string }[] {
  if (settled === undefined) return [];
  return [
    ...(settled.workflow_id === undefined
      ? []
      : [{ label: "Workflow", said: settled.workflow_id }]),
    ...(settled.title === undefined ? [] : [{ label: "Title", said: settled.title }]),
    ...(settled.done_when ?? []).map((said) => ({ label: "Done when", said })),
    ...(settled.settings === undefined
      ? []
      : [{ label: "Urgency", said: settled.settings.urgency }]),
  ];
}
