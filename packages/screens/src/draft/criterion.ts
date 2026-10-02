// One thing a Job is held to, and where it came from. Draft, for
// `crates/ipc/src/detail.rs`.
//
// Source of truth today: `Criterion` on `JobDetail.acceptance_criteria` —
// `criterion_id`, `text` and `source`, where `source` is a `CriterionSource`
// (`check`, `judge` or `attested`, `crates/core-model/src/job/fields.rs`).
//
// **`verified_by`, never a second `source`** (#1532, 22 Sep). The wire's
// `source` already means *how this criterion is answered*, and using the same
// word for *where the words came from* would give one name two meanings on one
// row. So the wire's field is renamed on the way in, and the new fact gets a
// name of its own.

import type { Criterion, JobDetail } from "@armada/protocol";

/** How a criterion is answered. The wire's `CriterionSource`, renamed. */
export type VerifiedBy = "check" | "judge" | "attested";

/**
 * Where the words came from.
 *
 * `issue` carries the reference so a surface can say the issue has moved since
 * the Job froze its words — the Job keeps what it froze (#1530, 22 Sep).
 *
 * **`url` is the forge address of that issue, and nothing serves it yet.** A
 * reference reads as `armada/1162`, which the owner could not tell from a
 * branch or a path (`u7y9`, 28 Sep), so a surface that has an address opens
 * the issue and one that has none draws the reference as the text it is.
 * Fleet carries no address on a criterion today, so nothing sets this.
 */
export type CriterionOrigin =
  | { origin: "issue"; ref: string; url?: string }
  | { origin: "prompt" }
  | { origin: "person" }
  // A Job no person dispatched: nothing names where its words came from, so
  // none is drawn rather than one invented (owner, 1 Oct 2026, `#1748` row 17).
  | { origin: "unsaid" };

/** One acceptance criterion, with its provenance. */
export type CriterionView = {
  /** Absent on a criterion nothing has minted an id for yet. */
  criterion_id?: string;
  text: string;
  verified_by: VerifiedBy;
  origin: CriterionOrigin;
  /**
   * When the issue these words came from was last edited, where that is after
   * the Job froze them. **Absent is the ordinary case** — it is present only
   * to say the source has moved, never to date the freeze.
   */
  origin_moved_at?: string;
};

/**
 * The Job origins a person dispatched, by typing or by asking Helm, which is
 * where a requester's words are typed. `ORIGIN`'s keys.
 */
const A_PERSON_ASKED: ReadonlySet<string> = new Set([
  "manual",
  "studio_dispatched",
  "helm_drafted",
  "studio_helm_drafted",
]);

/**
 * Today's wire carries the text, the id and how it is verified, and says
 * nothing about where the words came from — so a criterion derives from the
 * Job's own `origin`: `prompt` where a person dispatched it, and `unsaid`
 * everywhere else. It read *From your prompt* on a Job Fleet found itself
 * (owner, 1 Oct 2026, `#1748` row 17).
 */
export function criterionViewOf(criterion: Criterion, jobOrigin: string): CriterionView {
  return {
    criterion_id: criterion.criterion_id,
    text: criterion.text,
    verified_by: verifiedByOf(criterion.source),
    origin: { origin: A_PERSON_ASKED.has(jobOrigin) ? "prompt" : "unsaid" },
  };
}

/** Every criterion a Job is held to, in the order it was given. */
export function criterionViewsOf(detail: JobDetail): CriterionView[] {
  return detail.acceptance_criteria.map((one) => criterionViewOf(one, detail.job.origin));
}

// The wire leaves every closed set as `string`, so an unrecognised spelling is
// possible. It reads as `judge` — the one that declines to refuse and never
// grants — rather than as `check`, which would claim something mechanical
// answered a criterion nothing ran for.
function verifiedByOf(source: string): VerifiedBy {
  switch (source) {
    case "check":
    case "attested":
      return source;
    default:
      return "judge";
  }
}

/**
 * Where a criterion's words came from: the words, and the issue behind them
 * where there is one.
 *
 * **Two parts rather than one string, so the reference can be a link.** A
 * screen with an address draws `armada/1162` as the issue it opens; a screen
 * with none draws the same reference as text. Flattening them here would put
 * the reference inside a sentence nothing could reach into.
 */
export type OriginSaid = {
  /** The lead words. Reads whole on its own where there is no issue. */
  said: string;
  /** The issue these words came from, where they came from one. */
  issue?: { ref: string; url?: string };
};

/**
 * Where a criterion's words came from, as a person reads it, or `undefined`
 * where nothing names it — the slot stays empty rather than holding a guess.
 *
 * **One sentence, written once.** Three surfaces say it — the classifying
 * screen while it is yours to change, the same screen frozen, and Plan — and a
 * second spelling of "from the issue" is how two of them end up saying
 * different things about the same line.
 *
 * **The word `issue` is on the line since 28 Sep 2026.** It read `from
 * armada/1162`, and the owner asked what that was (`u7y9`): a bare
 * `owner/number` is a repository, a path and a branch as readily as an issue.
 */
export function originSaidOf(criterion: CriterionView): OriginSaid | undefined {
  const origin = criterion.origin;
  switch (origin.origin) {
    case "issue":
      return {
        said: "From issue",
        issue: origin.url === undefined ? { ref: origin.ref } : { ref: origin.ref, url: origin.url },
      };
    case "person":
      return { said: "You wrote this" };
    case "unsaid":
      return undefined;
    default:
      return { said: "From your prompt" };
  }
}

/** The same thing on one line, for a surface that draws text and no link. */
export function originLineOf(criterion: CriterionView): string | undefined {
  const from = originSaidOf(criterion);
  if (from === undefined) return undefined;
  return from.issue === undefined ? from.said : `${from.said} ${from.issue.ref}`;
}

/**
 * What will decide whether this criterion is met.
 *
 * **Future, because none of it has happened.** It read `answered by the
 * check` on a Job nobody had approved, which the owner read as a verdict
 * already in (`f9yw`, 28 Sep) — and on the classifying screen there is not
 * even a Drone yet. `verified_by` names who decides, never what they decided.
 */
export function decidedSaidOf(criterion: CriterionView): string {
  switch (criterion.verified_by) {
    case "check":
      return "A Check will decide it";
    case "attested":
      return "You will decide it";
    default:
      return "The Judge will decide it";
  }
}

/**
 * A criterion somebody typed here, with nothing filled in.
 *
 * **The Judge decides it, because nothing else can.** A Check is declared by
 * the workflow and frozen at creation, so a line added at the approval gate
 * has no Check to run against it; the Judge is the one reader that takes words
 * it was handed. `criterion_id` stays absent — nothing has minted one.
 */
export function criterionWritten(): CriterionView {
  return { text: "", verified_by: "judge", origin: { origin: "person" } };
}
