// The plan opened in full over the Cockpit, for review: every group open with its own paths and its tasks, the sketch drawn with it large
// and playing, and Approve at the foot as on the card in the thread. **Every group and task takes quick edits and notes in place** (`PlanEdit`),
// which collect as marks and go to the agent together as Send changes, beside Approve. Esc or Close shuts it, and approving shuts it too.

import { BadgeCheck } from "lucide-react";
import { Button, Sheet, SketchReading, Tooltip } from "@armada/components";
import { useAtFloor } from "@armada/shell";
import type { WorkPlan } from "@armada/screens/src/draft/sessions";

import { AddGroup, GroupEditor, SendChanges, type PlanEditing } from "./PlanEdit";
import { viewOf } from "./plan-marks";

export function PlanReview({ open, plan, ed, onApprove, onClose }: { open: boolean; plan: WorkPlan; ed: PlanEditing; onApprove: () => void; onClose: () => void }) {
  const floor = useAtFloor();
  const marked = ed.marks.length > 0;
  const settled = plan.approved ? (
    <Tooltip label="Plan approved">
      <span className="sv-panel__mark" role="img" aria-label="Plan approved">
        <BadgeCheck size={16} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  ) : undefined;
  const foot = (
    <div className="sv-review__foot">
      <SendChanges ed={ed} />
      {plan.approved ? null : (
        <Tooltip label={marked ? "Approve with these changes" : "Approve"}>
          <Button variant="primary" size="sm" onClick={onApprove}>
            Approve
          </Button>
        </Tooltip>
      )}
    </div>
  );
  const groups = viewOf(plan, ed.marks);
  return (
    <Sheet kind="plan-review" open={open} floating floor={floor} size="widest" title="Plan" controls={settled} footer={marked || !plan.approved ? foot : undefined} closeLabel="Close" closeBinding="Esc" onClose={onClose}>
      <div className="sv-review">
        <div className="sv-review__groups">
          {groups.map((group) => (
            <section key={group.id} className="sv-group sv-review__group" data-open aria-label={group.title}>
              <GroupEditor group={group} ed={ed} />
            </section>
          ))}
          <AddGroup ed={ed} />
        </div>
        {plan.sketch === undefined ? null : (
          <div className="sv-review__sketch">
            <SketchReading scene={plan.sketch.scene} title={plan.sketch.title} />
          </div>
        )}
      </div>
    </Sheet>
  );
}
