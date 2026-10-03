// One thing a Job is held to, and where it came from. Draft, for
// `crates/ipc/src/detail.rs`.
//
// Source of truth today: `Criterion` on `JobDetail.acceptance_criteria` —
// `criterion_id`, `text` and `source`, where `source` is a `CriterionSource`
// (`check`, `judge` or `attested`, `crates/core-model/src/job/fields.rs`), and
// since 23.8 `origin` and `origin_moved_at` (#1642).
//
// **`verified_by`, never a second `source`** (#1532, 22 Sep). The wire's
// `source` already means *how this criterion is answered*, and using the same
// word for *where the words came from* would give one name two meanings on one
// row. So the wire's field is renamed on the way in, and the new fact gets a
// name of its own.

import type { CriterionOriginKind } from "@armada/components";
import type {
  Criterion,
  CriterionOrigin as WireOrigin,
  CriterionWritten,
  JobDetail,
} from "@armada/protocol";

/** How a criterion is answered. The wire's `CriterionSource`, renamed. */
export type VerifiedBy = "check" | "judge" | "attested";

/**
 * Where the words came from.
 *
 * `issue` carries the reference so a surface can say the issue has moved since
 * the Job froze its words — the Job keeps what it froze (#1530, 22 Sep).
 *
 * **`url` is the forge address of that issue**, served on `Criterion.origin`
 * since 23.8 and the only address Bridge's main process opens. A reference
 * reads as `armada/1162`, which the owner could not tell from a branch or a
 * path (`u7y9`, 28 Sep), so a surface that has an address opens the issue and
 * one that has none draws the reference as the text it is.
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
 * One criterion as the wire carries it.
 *
 * **Where the words came from is Fleet's since 23.8** (`Criterion.origin`).
 * A criterion that says nothing — a Job kept before 23.8, or one no person
 * dispatched — derives from the Job's own `origin`: `prompt` where a person
 * dispatched it, and `unsaid` everywhere else. It read *From your prompt* on a
 * Job Fleet found itself (owner, 1 Oct 2026, `#1748` row 17).
 */
export function criterionViewOf(criterion: Criterion, jobOrigin: string): CriterionView {
  const view: CriterionView = {
    criterion_id: criterion.criterion_id,
    text: criterion.text,
    verified_by: verifiedByOf(criterion.source),
    origin: originOf(criterion.origin) ?? { origin: A_PERSON_ASKED.has(jobOrigin) ? "prompt" : "unsaid" },
  };
  if (criterion.origin_moved_at !== undefined) view.origin_moved_at = criterion.origin_moved_at;
  return view;
}

/**
 * The wire's origin in the draft's spelling, or `undefined` where it names
 * none. An `issue` with no reference is nothing a surface could name, and a
 * `kind` no Bridge knows reads as nothing rather than as a guess.
 */
function originOf(origin: WireOrigin | undefined): CriterionOrigin | undefined {
  switch (origin?.kind) {
    case "issue":
      if (origin.ref === undefined) return undefined;
      return origin.url === undefined
        ? { origin: "issue", ref: origin.ref }
        : { origin: "issue", ref: origin.ref, url: origin.url };
    case "prompt":
      return { origin: "prompt" };
    case "person":
      return { origin: "person" };
    default:
      return undefined;
  }
}

/**
 * One criterion as the approval and `edit_job` take it back (`CriterionWritten`,
 * #1641): its id where it has one, so a reworded line is still that line and
 * keeps its origin, and how it is answered.
 */
export function criterionWrittenOf(criterion: CriterionView): CriterionWritten {
  return {
    ...(criterion.criterion_id === undefined ? {} : { criterion_id: criterion.criterion_id }),
    text: criterion.text.trim(),
    source: criterion.verified_by,
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
 * Where a criterion's words came from: which origin, and the issue behind it
 * where there is one.
 *
 * **Two parts rather than one string, so the reference can be a link.** A
 * screen with an address draws `armada/1162` as the issue it opens; a screen
 * with none draws the same reference as text.
 */
export type OriginSaid = {
  /** Drawn as its mark, `CriterionOriginMark`, which names itself on hover. */
  kind: CriterionOriginKind;
  /** The issue these words came from, where they came from one. */
  issue?: { ref: string; url?: string };
};

/**
 * Where a criterion's words came from, or `undefined` where nothing names it —
 * the slot stays empty rather than holding a guess.
 *
 * **A mark, not a sentence, since 3 Oct 2026** (the owner: a state is never
 * text). It read *From issue*, *From your prompt* and *You wrote this*; those
 * words are now each mark's name, written once in `CriterionOrigin`, so the
 * three surfaces that draw it — the approval panel, the proposal screen and
 * Plan — cannot say different things about the same line.
 */
export function originSaidOf(criterion: CriterionView): OriginSaid | undefined {
  const origin = criterion.origin;
  switch (origin.origin) {
    case "issue":
      return {
        kind: "issue",
        issue: origin.url === undefined ? { ref: origin.ref } : { ref: origin.ref, url: origin.url },
      };
    case "person":
      return { kind: "person" };
    case "unsaid":
      return undefined;
    default:
      return { kind: "prompt" };
  }
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
