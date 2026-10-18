// Progress: where the Session is, in one panel. Status and the Plan showed the same thing from two levels, so they are one. The workflow's
// steps are a line of glyphs in the band, each named by its hover card; under it, the plan's groups and their tasks, with the plan's edits and marks
// as they were. A Session with no plan lists the agent's own tasks and its subagents at work, or is its band alone.

import { BadgeCheck, PencilRuler } from "lucide-react";
import { Button, Tooltip } from "@armada/components";
import type { Session, SessionWork, SessionWorkActs } from "@armada/screens/src/draft/sessions";

import { AskUpdate } from "../cockpit/deck/AskUpdate";
import { AgentTasks } from "./AgentTasks";
import { AddGroup, SendChanges, type PlanEditing } from "./PlanEdit";
import { Group } from "./PlanGroup";
import { viewOf } from "./plan-marks";
import { Flow, StepRail } from "./StatusPanel";
import { SubagentsAtWork } from "./SubagentsAtWork";
import { subagentsAtWork } from "./subagents-at-work";
import { SvPanel, type PanelState, type Place } from "./SvPanel";

export function ProgressPanel({ work, place, id, session, manifestId, acts, ed, onOpenSketch, onOpenSession }: { work: SessionWork; place: Place; id: string; session: Session; manifestId?: string | undefined; acts: SessionWorkActs | undefined; ed: PlanEditing | undefined; onOpenSketch: (sketch: string) => void; onOpenSession: (id: string) => void }) {
  const plan = work.plan;
  const flow = work.railOff === true ? undefined : work.workflow;
  const finished = plan !== undefined && plan.approved && plan.groups.every((group) => group.tasks.every((task) => task.state === "done" || task.state === "handed_in"));
  const state: PanelState | undefined = plan === undefined || !plan.approved ? undefined : finished ? "done" : "live";
  const sketch = plan?.sketch;
  // With no plan to show them, the agent's tasks and the subagents at work are listed. With none, a bare None select is all there would be: the panel stands folded to its band, and a press opens it.
  const planned = plan !== undefined && ed !== undefined;
  const tasks = planned ? [] : (work.tasks ?? []);
  const atWork = planned ? [] : subagentsAtWork(session);
  const bare = !planned && tasks.length === 0 && atWork.length === 0 && flow === undefined && work.railOff !== true && work.proposal?.workflow === null && acts !== undefined;
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
    <SvPanel label="Progress" area="progress" place={place} state={state} acts={bar} empty={bare} {...(flow === undefined ? {} : { rail: <StepRail steps={flow.steps} at={flow.at} /> })}>
      {work.railOff === true ? null : <Flow work={work} acts={acts} id={id} manifestId={manifestId} />}
      {tasks.length === 0 ? null : <AgentTasks tasks={tasks} />}
      {atWork.length === 0 ? null : <SubagentsAtWork rows={atWork} />}
      {plan === undefined || ed === undefined ? null : (
        <div className="sv-plan">
          <div className="sv-plan__groups">
            {viewOf(plan, ed.marks).map((group) => (
              <Group key={group.id} group={group} ed={ed} parent={session} onOpenSession={onOpenSession} />
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
