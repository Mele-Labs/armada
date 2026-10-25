// An adversarial review: one read-only model call that tries to break what a person is about to approve,
// its findings pinned to the part they are about. `crates/ipc/src/adversarial_review.rs`,
// `docs/concepts/adversarial-review.md`.
//
// The header rules in `protocol.ts` hold here. Only a press starts one, and a finding reaches the agent only if
// the person sends it. The latest review of a Job's plan is `SessionPlan.adversarial`, so it arrives with every
// `session.changed` and no event carries it.

/** What a review read. A plan is the only kind so far; a pull request or a document may follow. */
export type ReviewSubject = { kind: "plan"; session_id: string; version: number };

/** How far a review has got. `failed` is never a result. */
export type AdversarialReviewState = "running" | "done" | "failed";

/** The part of the subject a finding is about. Ids are the subject's own: for a plan, its group and task ids. */
export type FindingAnchor = { kind: "plan" } | { kind: "group"; id: string } | { kind: "task"; id: string };

/** How the person settled a finding: `sent` to the agent as his message, or `dismissed` with the agent told nothing. */
export type FindingHow = "sent" | "dismissed";

/** A finding once settled. Absent while it waits. */
export type FindingSettled = { how: FindingHow; at: string };

/** One way the subject breaks, in the three parts a judge's concern has. */
export type AdversarialFinding = {
  /** Unique within its review. */
  id: string;
  about: FindingAnchor;
  /** What breaks, in a line. Heads the finding. */
  consequence: string;
  expected: string;
  found: string;
  /** What a person using the thing sees or cannot do because of it. Absent on a finding kept before it was asked for. */
  impact?: string;
  /** `path` or `path:line`. Absent where it cites nothing. */
  cites?: string[];
  settled?: FindingSettled;
};

/** One review of one subject, going or kept. */
export type AdversarialReview = {
  subject: ReviewSubject;
  state: AdversarialReviewState;
  started_at: string;
  finished_at?: string;
  /** Why a failed review failed, in Fleet's words. */
  failed?: string;
  /** None is an answer when the state is `done`: it read the subject and found nothing. */
  findings: AdversarialFinding[];
  /** What the vendor billed, in millionths of a dollar. Absent where the call ended without saying, which is not a cost of zero. */
  cost_micros?: number;
};

/**
 * `start_adversarial_review`'s body (`POST /adversarial_review`). Answers an `AdversarialReview` at once: running, or
 * the one kept for the subject when `again` is not set. 404 `fleet.review_subject_unknown` for no such Job or no plan,
 * 409 `fleet.review_subject_stale` for a plan version that is not the current one.
 */
export type StartAdversarialReview = {
  subject: ReviewSubject;
  /** Run it again, whatever is kept. */
  again?: boolean;
};

/** `send_finding`'s body (`POST /adversarial_review/send`): the finding goes to the agent as the person's message. Answers the review. 404 `fleet.finding_unknown` for an id it does not hold; a finding already settled is answered as it stands. */
export type SendFinding = { subject: ReviewSubject; finding_id: string };

/** `dismiss_finding`'s body (`POST /adversarial_review/dismiss`): the finding is settled and the agent is told nothing. Answers the review. The same refusals. */
export type DismissFinding = { subject: ReviewSubject; finding_id: string };
