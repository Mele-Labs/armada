// The plan opened in full over the Cockpit, for review: every group open with its own paths and its tasks, the sketch drawn with it large
// and playing, and Approve at the foot as on the card in the thread. **Every group and task takes quick edits and notes in place** (`PlanEdit`),
// which collect as marks and go to the agent together as Send changes, beside Approve. Esc or Close shuts it, and approving shuts it too.
// **A plan recorded again opens as the diff the thread's card draws**, changed groups open and the rest one line, with Edit plan to reach the rows.

import { useState } from "react";
import { BadgeCheck } from "lucide-react";
import { Button, FindingNotes, LandsInChip, PlanRevisionView, Sheet, SketchReading, Tooltip, anchorsOf, landingsOf, useFindingKeys, useUnchanged, type AdversarialReviewState, type PlanRevision } from "@armada/components";
import { useAtFloor } from "@armada/shell";
import type { WorkPlan } from "@armada/screens/src/draft/sessions";

import { AddGroup, GroupEditor, SendChanges, type FindingPins, type PlanEditing } from "./PlanEdit";
import { OverTheApp } from "./OverTheApp";
import { viewOf } from "./plan-marks";

export function PlanReview({ open, plan, revision, ed, findings, pullRequests, onApprove, onClose }: { open: boolean; plan: WorkPlan; revision?: PlanRevision; ed: PlanEditing; /** The Job opens pull requests: the groups stand under the chip of the one each lands in, and a chip's press opens its pull request. */ pullRequests?: { onOpen: (number: number) => void }; /** An adversarial review of the plan, its findings pinned under the group or task each names, with the acts the card offers. */ findings?: { review: AdversarialReviewState; onSend: (id: string) => void; onDismiss: (id: string) => void }; onApprove: () => void; onClose: () => void }) {
  const floor = useAtFloor();
  const [editing, setEditing] = useState(false);
  const unchanged = useUnchanged(revision);
  // Marks already made are the rows' to show, so a plan holding any opens on them.
  const diff = revision !== undefined && !editing && ed.marks.length === 0;
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
      {revision === undefined || plan.approved || ed.marks.length > 0 ? null : (
        <Button variant="ghost" size="sm" onClick={() => setEditing(!editing)}>
          {editing ? "Show changes" : "Edit plan"}
        </Button>
      )}
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
  const keys = useFindingKeys(findings?.review, { onSend: (id) => findings?.onSend(id), onDismiss: (id) => findings?.onDismiss(id) });
  const pins: FindingPins | undefined = findings === undefined ? undefined : { anchors: anchorsOf(findings.review, groups.flatMap((group) => [group.id, ...group.tasks.map((task) => task.id)])), onSend: keys.send, onDismiss: keys.dismiss };
  return (
    <OverTheApp open={open}>
    <Sheet kind="plan-review" open={open} floating floor={floor} size="widest" title="Plan" controls={settled} footer={marked || !plan.approved ? foot : undefined} closeLabel="Close" closeBinding="Esc" onClose={onClose}>
      <div className="sv-review">
        <div className="sv-review__groups" {...(pins === undefined ? {} : keys.bind)}>
          {diff ? (
            <PlanRevisionView revision={revision} unchanged={unchanged} />
          ) : (
            <>
              {pins === undefined ? null : <FindingNotes findings={pins.anchors.whole} label="Findings on the plan" onSend={pins.onSend} onDismiss={pins.onDismiss} />}
              {landingsOf(groups, pullRequests !== undefined).map((run) => {
                const editors = run.groups.map((group) => (
                  <section key={group.id} className="sv-group sv-review__group" data-open aria-label={group.title}>
                    <GroupEditor group={group} ed={ed} {...(pins === undefined ? {} : { pins })} />
                  </section>
                ));
                return run.landsIn === undefined ? (
                  editors
                ) : (
                  <div key={run.key} className="sv-review__landing" role="group" aria-label={`Lands in ${run.landsIn.pullRequest === undefined ? run.key : `#${run.landsIn.pullRequest.number}`}`}>
                    <LandsInChip landsIn={run.landsIn} onOpen={pullRequests?.onOpen} />
                    {editors}
                  </div>
                );
              })}
              <AddGroup ed={ed} />
            </>
          )}
        </div>
        {plan.sketch === undefined ? null : (
          <div className="sv-review__sketch">
            <SketchReading scene={plan.sketch.scene} title={plan.sketch.title} />
          </div>
        )}
      </div>
    </Sheet>
    </OverTheApp>
  );
}
