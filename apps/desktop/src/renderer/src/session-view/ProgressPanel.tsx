// Progress: where the Session is, in one panel. Status and the Plan showed the same thing from two levels, so they are one. The workflow's
// steps are a line in the band, the step the Session is at named; under it, the plan's groups and their tasks, with the plan's edits and marks
// as they were. A Session with no plan is its band alone.

import { BadgeCheck, PencilRuler } from "lucide-react";
import { Button, Tooltip } from "@armada/components";
import type { SessionWork, SessionWorkActs } from "@armada/screens/src/draft/sessions";

import { AskUpdate } from "../cockpit/deck/AskUpdate";
import { AddGroup, SendChanges, type PlanEditing } from "./PlanEdit";
import { Group } from "./PlanPanel";
import { viewOf } from "./plan-marks";
import { Flow, StepRail } from "./StatusPanel";
import { SvPanel, type PanelState, type Place } from "./SvPanel";

export function ProgressPanel({ work, place, id, manifestId, acts, ed, onOpenSketch }: { work: SessionWork; place: Place; id: string; manifestId?: string | undefined; acts: SessionWorkActs | undefined; ed: PlanEditing | undefined; onOpenSketch: (sketch: string) => void }) {
  const plan = work.plan;
  const flow = work.railOff === true ? undefined : work.workflow;
  const finished = plan !== undefined && plan.approved && plan.groups.every((group) => group.tasks.every((task) => task.state === "done"));
  const state: PanelState | undefined = plan === undefined || !plan.approved ? undefined : finished ? "done" : "live";
  const sketch = plan?.sketch;
  const bar = (
    <>
      {sketch === undefined ? null : (
        <Tooltip label="Sketch">
          <Button variant="ghost" size="sm" iconOnly aria-label="Sketch" onClick={() => onOpenSketch(sketch.id)}>
            <PencilRuler size={16} strokeWidth={2} aria-hidden />
          </Button>
        </Tooltip>
      )}
      <AskUpdate subject={`session:${id}`} />
      {plan?.approved === true ? (
        <Tooltip label="Plan approved">
          <span className="sv-panel__mark" role="img" aria-label="Plan approved">
            <BadgeCheck size={16} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
      ) : null}
    </>
  );
  return (
    <SvPanel label="Progress" area="progress" place={place} state={state} acts={bar} {...(flow === undefined ? {} : { rail: <StepRail steps={flow.steps} at={flow.at} /> })}>
      {work.railOff === true ? null : <Flow work={work} acts={acts} id={id} manifestId={manifestId} />}
      {plan === undefined || ed === undefined ? null : (
        <div className="sv-plan">
          <div className="sv-plan__groups">
            {viewOf(plan, ed.marks).map((group) => (
              <Group key={group.id} group={group} ed={ed} />
            ))}
          </div>
          <div className="sv-plan__foot">
            <AddGroup ed={ed} />
            <SendChanges ed={ed} />
          </div>
        </div>
      )}
    </SvPanel>
  );
}
