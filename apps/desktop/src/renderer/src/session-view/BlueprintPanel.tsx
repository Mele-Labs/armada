// Blueprint: the plan the agent recorded, in the Job's right column under the workflow. While it is being drafted it stands open and editable: the
// summary in his words, every group with its tasks, the sketch from the band, and Approve at the foot beside the marks he put on it. Once approved
// it stays up and compact: a line to each group with a mark for each task, the group being worked lit. A Job with no plan lists the agent's own
// tasks and the subagents at work in its place, and with neither is its band alone. The stage opens it (`story.ts`); the band or ⌥B does too.

import { BadgeCheck, PencilRuler } from "lucide-react";
import { Button, Tooltip } from "@armada/components";
import type { Session, SessionWork, SessionWorkActs } from "@armada/screens/src/draft/sessions";

import { AskUpdate } from "../cockpit/deck/AskUpdate";
import { accountGroupOf } from "../session-activity";
import { AgentTasks } from "./AgentTasks";
import { AddGroup, SendChanges, type PlanEditing } from "./PlanEdit";
import { Group } from "./PlanGroup";
import { stepsOf } from "./PlanSteps";
import { viewOf } from "./plan-marks";
import { SubagentsAtWork } from "./SubagentsAtWork";
import { subagentsAtWork } from "./subagents-at-work";
import { SvPanel, type PanelState, type Place } from "./SvPanel";
import { bandKeyFor, useFold } from "./story";
import "./progress-takes.css";
import "./story.css";

export function BlueprintPanel({ work, place, id, session, manifestId, acts, ed, onOpenSketch, onOpenSession }: { work: SessionWork; place: Place; id: string; session: Session; manifestId?: string | undefined; acts: SessionWorkActs | undefined; ed: PlanEditing | undefined; onOpenSketch: (sketch: string) => void; onOpenSession: (id: string) => void }) {
  const plan = work.plan;
  const fold = useFold(session, "blueprint");
  const planned = plan !== undefined && ed !== undefined;
  const drafting = planned && !plan.approved;
  const finished = plan !== undefined && plan.approved && plan.groups.every((group) => group.tasks.every((task) => task.state === "done" || task.state === "handed_in"));
  const state: PanelState | undefined = plan === undefined || !plan.approved ? undefined : finished ? "done" : "live";
  const tasks = planned ? [] : (work.tasks ?? []);
  const atWork = planned ? [] : subagentsAtWork(session);
  const workingGroup = accountGroupOf(session)?.id;
  const groups = planned ? viewOf(plan, ed.marks) : [];
  const steps = stepsOf(groups, plan?.approved === true, workingGroup);
  const sketch = plan?.sketch;
  const empty = !planned && tasks.length === 0 && atWork.length === 0 && acts !== undefined;
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
    <SvPanel label="Blueprint" area="blueprint" place={place} state={state} acts={bar} empty={empty} fold={fold} keycap={bandKeyFor("blueprint")}>
      {drafting && plan.summary !== undefined ? <p className="sv-blueprint__summary">{plan.summary}</p> : null}
      {tasks.length === 0 ? null : <AgentTasks tasks={tasks} />}
      {atWork.length === 0 ? null : <SubagentsAtWork rows={atWork} />}
      {!planned ? null : (
        <div className="sv-plan" data-drafting={drafting || undefined}>
          <div className="sv-plan__groups">
            {groups.map((group) => {
              const step = steps.get(group.id);
              return <Group key={group.id} group={group} {...(drafting && step !== undefined ? { step, opened: true } : { working: group.id === workingGroup })} ed={ed} parent={session} manifestId={manifestId} onOpenSession={onOpenSession} />;
            })}
          </div>
          <div className="sv-plan__foot">
            <AddGroup ed={ed} />
            <span className="sv-plan__acts">
              <SendChanges ed={ed} />
              {!drafting || ed.approve === undefined ? null : (
                <Tooltip label={ed.marks.length > 0 ? "Approve with these changes" : "Approve"}>
                  <Button variant="primary" size="sm" onClick={ed.approve}>
                    Approve
                  </Button>
                </Tooltip>
              )}
            </span>
          </div>
        </div>
      )}
    </SvPanel>
  );
}
