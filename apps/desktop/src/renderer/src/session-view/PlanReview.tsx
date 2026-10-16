// The plan opened in full over the Cockpit, for review: every group open with its own paths and its tasks, the sketch drawn with it large
// and playing, and Approve here as on the card in the thread. Esc or Close shuts it, and approving shuts it too.

import { BadgeCheck, Bot } from "lucide-react";
import { Button, Sheet, SketchReading, TaskMark, Tooltip } from "@armada/components";
import { useAtFloor } from "@armada/shell";
import type { WorkPlan } from "@armada/screens/src/draft/sessions";

export function PlanReview({ open, plan, onApprove, onClose }: { open: boolean; plan: WorkPlan; onApprove: () => void; onClose: () => void }) {
  const floor = useAtFloor();
  const approve = plan.approved ? (
    <Tooltip label="Plan approved">
      <span className="sv-panel__mark" role="img" aria-label="Plan approved">
        <BadgeCheck size={16} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  ) : (
    <Button variant="primary" size="sm" onClick={onApprove}>
      Approve
    </Button>
  );
  return (
    <Sheet kind="plan-review" open={open} floating floor={floor} size="widest" title="Plan" controls={approve} closeLabel="Close" closeBinding="Esc" onClose={onClose}>
      <div className="sv-review">
        <div className="sv-review__groups">
          {plan.groups.map((group) => (
            <section key={group.id} className="sv-group sv-review__group" data-open aria-label={group.title}>
              <div className="sv-group__row">
                <span className="sv-group__head">
                  <span className="sv-group__agent" role="img" aria-label={`Subagent type ${group.agent}`}>
                    <Bot size={12} strokeWidth={2} aria-hidden />
                    <code>{group.agent}</code>
                  </span>
                </span>
                <span className="sv-group__title">{group.title}</span>
              </div>
              <div className="sv-group__body">
                <ul className="sv-scope" aria-label="Paths">
                  {group.paths.map((path) => (
                    <li key={path}>
                      <code>{path}</code>
                    </li>
                  ))}
                </ul>
                <ul className="sv-tasks">
                  {group.tasks.map((task) => (
                    <li key={task.id} className="sv-task" data-state={task.state}>
                      <TaskMark state={task.state} />
                      <span>{task.text}</span>
                    </li>
                  ))}
                </ul>
              </div>
            </section>
          ))}
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
