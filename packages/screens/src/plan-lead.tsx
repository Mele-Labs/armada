// What the Plan destination leads with: the words the Job is held to, and the
// two acts at the plan's gate under them.
//
// **It leads because the owner asked why it did not** (28 Sep 2026): *Isn't
// this why the plan was formed?* The design board leads with the same card.

import { Button, Card, CardContent } from "@armada/components";

import type { JobSummary, StepDetail } from "@armada/protocol";

import { Eyebrow } from "./InsideAJob";
import { RedirectControl } from "./Redirect";
import type { CriterionView } from "./draft/criterion";

export type PlanLeadProps = {
  criteria: readonly CriterionView[];
  job: JobSummary;
  /** The step that recorded the plan. The gate is drawn only while it waits. */
  step: StepDetail | undefined;
  /** Every control is refused while what is shown is not live. */
  stale: boolean;
  acting: boolean;
  deciding: boolean;
  onApproveReview: (jobId: string) => void;
  onRedirect: (jobId: string, instruction: string) => void;
};

/** Where a criterion's words came from, as one line. */
function originSaid(criterion: CriterionView): string {
  const from =
    criterion.origin.origin === "issue"
      ? `from ${criterion.origin.ref}`
      : criterion.origin.origin === "person"
        ? "written by you"
        : "from the prompt";
  return `${from} · answered by the ${criterion.verified_by}`;
}

/**
 * The two acts at a plan gate. **Approve it, or tell the Drone that wrote it
 * what to change** — the owner's call of 22 Sep 2026. The narrower asks, which
 * change one group or one task without sending the whole plan back, are on the
 * board's own cards and in the task inspector (`#1552`).
 *
 * Drawn only while the step that recorded the plan is waiting on a person, so
 * a Job already implementing offers nothing here to press.
 */
function PlanGate({
  job,
  step,
  stale,
  acting,
  deciding,
  onApproveReview,
  onRedirect,
}: Omit<PlanLeadProps, "criteria">) {
  if (step === undefined || step.state !== "awaiting_human") return null;
  return (
    <div className="armada-plan-tab__gate">
      <p className="armada-plan-tab__waiting">This plan is waiting on you.</p>
      <div className="armada-plan-tab__acts">
        <Button
          variant="primary"
          size="sm"
          disabled={stale || deciding}
          pending={deciding}
          onClick={() => onApproveReview(job.id)}
        >
          Approve the plan
        </Button>
        <RedirectControl
          jobId={job.id}
          drone="holding"
          disabled={stale || acting}
          onRedirect={onRedirect}
        />
      </div>
    </div>
  );
}

/**
 * What the Job is held to, with where each line came from, and the gate under
 * it.
 *
 * **Not the planner's expectation**, which is per task and sits in the
 * inspector. `#1274` is why the two are never one list: a Drone never chooses
 * the cases it is held to, and a task's `expects` is the planner's word.
 */
export function PlanLead({ criteria, ...gate }: PlanLeadProps) {
  return (
    <Card className="armada-plan-tab__lead">
      <CardContent>
        <Eyebrow>What this Job is held to</Eyebrow>
        {criteria.length === 0 ? (
          <p className="armada-plan-tab__criterion-origin">
            Nothing was written down for this Job to be held to.
          </p>
        ) : (
          <ul className="armada-plan-tab__criteria">
            {criteria.map((criterion, at) => (
              <li key={criterion.criterion_id ?? at}>
                <span className="armada-plan-tab__criterion-text">{criterion.text}</span>
                <span className="armada-plan-tab__criterion-origin">{originSaid(criterion)}</span>
                {/* The Job keeps the words it froze and says the issue has
                    moved since — `#1530`, 22 Sep. It never re-reads them. */}
                {criterion.origin_moved_at === undefined ? null : (
                  <span className="armada-plan-tab__criterion-moved">
                    The issue has been edited since these words were frozen.
                  </span>
                )}
              </li>
            ))}
          </ul>
        )}
        <PlanGate {...gate} />
      </CardContent>
    </Card>
  );
}
