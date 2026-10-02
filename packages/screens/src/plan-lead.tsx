// What the Plan destination leads with: the words the Job is held to, and the
// two acts at the plan's gate under them.
//
// **It leads because the owner asked why it did not** (28 Sep 2026): *Isn't
// this why the plan was formed?* The design board leads with the same card.

import { useState, type ReactNode } from "react";
import { ChevronDown, ChevronRight } from "lucide-react";

import { Button, Card, CardContent, Skeleton, SkeletonText, Textarea } from "@armada/components";

import type { JobSummary, StepDetail } from "@armada/protocol";

import { Eyebrow } from "./regions";
import { decidedSaidOf, originLineOf } from "./draft/criterion";
import type { CriterionView } from "./draft/criterion";

export type PlanLeadProps = {
  criteria: readonly CriterionView[];
  /**
   * Approve the plan and Propose a change, as `PlanReview` builds them — the
   * same node Overview's gate draws, so the two cannot offer different acts.
   */
  gate: ReactNode;
  /**
   * Whether this Job's own read has yet to answer. The criteria stand in as
   * bars until it has: "Nothing was written down" is the answer for a Job that
   * was read and holds none.
   */
  reading?: boolean;
};

export type PlanGateProps = {
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

/**
 * Where a criterion's words came from, and what will decide it.
 *
 * **Both halves are `draft/criterion.ts`'s, not spelled again here.** This
 * file was written while the same sentence was being fixed on `main`, and
 * carried the old spelling with it when the card moved off the foot of the
 * tab — `answered by the check`, which the owner called useless and
 * confusing on 28 Sep. A second copy is how it came back the first time.
 */
function originSaid(criterion: CriterionView): string {
  return `${originLineOf(criterion)} · ${decidedSaidOf(criterion)}`;
}

/**
 * The two acts at a plan gate. **Approve it, or tell the Drone that wrote it
 * what to change** — the owner's call of 22 Sep 2026. **The request is a field
 * that is always open** (owner, 30 Sep 2026: *why isn't this just a
 * textarea?*), sent as the redirect the button's dialog sent. The narrower asks, which
 * change one group or one task without sending the whole plan back, are on the
 * board's own cards and in the task inspector (`#1552`).
 *
 * Drawn only while the step that recorded the plan is waiting on a person, so
 * a Job already implementing offers nothing here to press.
 */
export function PlanGate({
  job,
  step,
  stale,
  acting,
  deciding,
  onApproveReview,
  onRedirect,
}: PlanGateProps) {
  const [instruction, setInstruction] = useState("");
  if (step === undefined || step.state !== "awaiting_human") return null;
  const empty = instruction.trim() === "";
  return (
    <div className="armada-plan-tab__gate">
      <p className="armada-plan-tab__waiting">This plan is waiting on you.</p>
      <Textarea
        label="Request changes to the entire plan"
        rows={3}
        value={instruction}
        disabled={stale || acting}
        onChange={(event) => setInstruction(event.target.value)}
      />
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
        <Button
          variant="secondary"
          size="sm"
          disabled={stale || acting || empty}
          pending={acting}
          onClick={() => {
            onRedirect(job.id, instruction.trim());
            setInstruction("");
          }}
        >
          Send
        </Button>
      </div>
    </div>
  );
}

const OPEN_KEY = "armada.bridge.plan-lead-open";

/** The chrome disclosure pair, `chevron-right` shut and `chevron-down` open. */
const CHEVRON = 16;
const MARK_STROKE = 2;

function readOpen(): boolean {
  try {
    return window.localStorage.getItem(OPEN_KEY) === "true";
  } catch {
    return false;
  }
}

/**
 * Whether the lead was left open, remembered across a restart. `localStorage`
 * for `left-collapsed.ts`' reason: one window's way of looking, not a fact
 * about the fleet. Shut where nothing is stored.
 */
function useLeadOpen(): [boolean, () => void] {
  const [open, setOpen] = useState(readOpen);

  function toggle(): void {
    const next = !open;
    setOpen(next);
    try {
      window.localStorage.setItem(OPEN_KEY, String(next));
    } catch {
      // A failed write leaves the choice unremembered, the honest answer for a preference.
    }
  }

  return [open, toggle];
}

const NOTHING_HELD = "Nothing was written down for this Job to be held to.";

/**
 * What the Job is held to, with where each line came from, and the gate under
 * it.
 *
 * **Shut, it is the band with the first criterion cut to fit on the line
 * under it** — the owner's call of 29 Sep 2026, after scrolling past it on a
 * failed group every time, and stacked rather than side by side the same day. The criteria after the first are hidden until pressed; that was the
 * cost he took, and no count of them is drawn.
 *
 * **The gate is never folded.** It sits outside the disclosure, so a plan
 * waiting on a person shows its two acts under the shut line as well as under
 * the open list.
 *
 * **Not the planner's expectation**, which is per task and sits in the
 * inspector. `#1274` is why the two are never one list: a Drone never chooses
 * the cases it is held to, and a task's `expects` is the planner's word.
 */
export function PlanLead({ criteria, gate, reading = false }: PlanLeadProps) {
  const [open, toggle] = useLeadOpen();
  const Mark = open ? ChevronDown : ChevronRight;
  const first = criteria[0]?.text ?? NOTHING_HELD;
  return (
    <Card className="armada-plan-tab__lead">
      <CardContent>
        <button
          type="button"
          className="armada-plan-tab__fold"
          aria-expanded={open}
          onClick={toggle}
        >
          <Mark
            className="armada-plan-tab__fold-mark"
            size={CHEVRON}
            strokeWidth={MARK_STROKE}
            aria-hidden
          />
          <Eyebrow>What this Job is held to</Eyebrow>
          {open ? null : reading ? (
            // In the line's own column, which is what gives the bar a width.
            <span className="armada-plan-tab__fold-first">
              <Skeleton width="40%" />
            </span>
          ) : (
            <span
              className="armada-plan-tab__fold-first"
              data-empty={criteria.length === 0 || undefined}
            >
              {first}
            </span>
          )}
        </button>
        {!open ? null : reading ? (
          <SkeletonText />
        ) : criteria.length === 0 ? (
          <p className="armada-plan-tab__criterion-origin">{NOTHING_HELD}</p>
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
        {gate}
      </CardContent>
    </Card>
  );
}
