// Fleet's three adversarial review routes, answered the way the daemon answers them, on one Job's plan. The mock Fleet that serves
// Sessions (`sessions-fleet.ts`) and the walk's draft Sessions (`adversarial-review.ts`) both answer through here, so a walk and a test
// press the same rules a person's press meets: `docs/practices/protocol/an-adversarial-review-of-a-plan.md`.

import type { AdversarialFinding, AdversarialReview, DismissFinding, FindingHow, SendFinding, SessionPlan, StartAdversarialReview, WireError } from "@armada/protocol";

export type Refused = { ok: false; why: "refused"; error: WireError };
/** What the routes read of a plan: its version and the review kept on it. */
type Plan = Pick<SessionPlan, "version" | "adversarial">;
export type Answered = { ok: true; review: AdversarialReview } | Refused;

const refused = (code: string, message: string): Refused => ({ ok: false, why: "refused", error: { code, message, run_id: "mock", fields: {}, chain: [] } });

/** The plan the subject names, or Fleet's refusal: no plan, or a version that is not the current one. */
function current(plan: Plan | undefined, version: number): Plan | Refused {
  if (plan === undefined) return refused("fleet.review_subject_unknown", "no such Job, or it has no plan");
  if (plan.version !== version) return refused("fleet.review_subject_stale", "the plan was recorded again since");
  return plan;
}

/** `start_adversarial_review`: running at once, or the review kept for this subject where `again` is not set. */
export function startReview(plan: Plan | undefined, start: StartAdversarialReview, now: string): Answered {
  const held = current(plan, start.subject.version);
  if ("ok" in held) return held;
  const kept = held.adversarial;
  if (start.again !== true && kept !== undefined && kept.subject.version === start.subject.version) return { ok: true, review: kept };
  return { ok: true, review: { subject: start.subject, state: "running", started_at: now, findings: [] } };
}

/** The review read to its end: done with what the adversary found. */
export const finished = (review: AdversarialReview, findings: readonly AdversarialFinding[], now: string): AdversarialReview => ({ ...review, state: "done", finished_at: now, findings: [...findings] });

/** `send_finding` and `dismiss_finding`: the finding settled on the review kept, of whatever version. One already settled is answered as it stands. */
export function settleFinding(plan: Plan | undefined, body: SendFinding | DismissFinding, how: FindingHow, now: string): Answered {
  const held = current(plan, body.subject.version);
  if ("ok" in held) return held;
  const kept = held.adversarial;
  if (kept === undefined || !kept.findings.some((one) => one.id === body.finding_id)) return refused("fleet.finding_unknown", `no finding ${body.finding_id} in this review`);
  return { ok: true, review: { ...kept, findings: kept.findings.map((one) => (one.id === body.finding_id && one.settled === undefined ? { ...one, settled: { how, at: now } } : one)) } };
}
