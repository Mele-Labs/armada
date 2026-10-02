// Job 2 at the Judge's refusal of its plan — the moment three surfaces told
// three stories on 1 Oct 2026 (`#1748` row 13). Apart from
// `job-detail-fixtures.ts`, which is past its length.

import type { StepDetail } from "@armada/protocol";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { featureOnItsPlan } from "./job-detail-fixtures";

/**
 * Job 2 at the Judge's refusal of its plan, **in the shape `GET /jobs/2`
 * recorded for the plan step on 1 Oct 2026**: `plan_recorded` passed, two
 * criteria judged and `addresses_the_request` not met, the step at
 * `awaiting_human` and the question open on it. The question's own words are
 * a stand-in: the read after the answer no longer carries them.
 */
export function featureAtAPlanRefusal(): JobFixture {
  const base = featureOnItsPlan();
  if (base.watched.state !== "read") return base;
  const whole = base.watched.detail;
  const askedAt = "2026-10-01T20:26:21.196Z";
  const refused = {
    expected:
      "Tasks to retire guides 8 and 20 from the catalogue, add a repository rule that guides' pieces must be " +
      "drawn somewhere, and verify the rule works",
    produced:
      'Tasks T1–T3 address the request, but T4 adds "Document the rule and fix prose that pointed at the ' +
      'retired guides" — documentation updates to design-system.md and prose fixes in docs/concepts',
    consequence:
      "Scope expands beyond what was requested; the original request specifies retiring guides and adding " +
      "the rule, not documenting the rule or maintaining references to it in existing prose",
  };
  const steps = whole.steps.map((one): StepDetail =>
    one.step_id !== "plan"
      ? one
      : {
          ...one,
          state: "awaiting_human",
          check_runs: [{ attempt: 1, name: "plan_recorded", outcome: "passed" }],
          judge_checks: [{ criteria: 2, gaming_check: false }],
          judged: [
            { attempt: 1, criterion_id: "addresses_the_request", verdict: "not_met", ...refused, cited: [] },
            { attempt: 1, criterion_id: "names_what_it_will_touch", verdict: "met", cited: [] },
          ],
          attempts: [{ attempt: 1, outcome: "awaiting_human", started_at: one.entered_at, ended_at: askedAt }],
          updated_at: askedAt,
        },
  );
  const job = { ...base.job, status: "awaiting_review" };
  const detail = {
    ...whole,
    job,
    steps,
    judge_question: {
      step_id: "plan",
      criterion_id: "addresses_the_request",
      question: "Does the plan address what was asked, and nothing beyond it?",
      ...refused,
      asked_at: askedAt,
    },
  };
  return { ...base, job, watched: { ...base.watched, detail } };
}
