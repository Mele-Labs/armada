// Progress: where the Session is, in one panel. Status and the Plan showed the same thing from two levels, so they are one. The workflow, its picker and
// its steps stand in the Workflow panel above it (`WorkflowPanel.tsx`), and the band keeps the name and its few acts. The body reads in the order of what matters: what is happening now,
// where that sits in the plan, then the issues and pull requests it works, which are reference, and under those the watches it holds. The plan is a stepper (`PlanGroup.tsx`, `PlanSteps.tsx`):
// done groups and groups to come are one muted line, the group being worked is open and raised, with the plan's edits and marks as they were.
// A Session with no plan lists the agent's own tasks and its subagents at work, or is its band alone.
// Now is the turn's account in his words and the page it is building, while a turn runs and until his next message (`TurnAccountBlock.tsx`).

import { BadgeCheck, PencilRuler } from "lucide-react";
import type { ReactNode } from "react";
import { Button, Tooltip } from "@armada/components";
import type { Session, SessionWork, SessionWorkActs } from "@armada/screens/src/draft/sessions";

import { AskUpdate } from "../cockpit/deck/AskUpdate";
import { remindLater } from "../cockpit/deck/acts";
import { accountGroupOf, accountOf } from "../session-activity";
import { useSessionsDraft } from "../sessions-draft";
import { AgentTasks } from "./AgentTasks";
import { AddGroup, SendChanges, type PlanEditing } from "./PlanEdit";
import { IssuesBlock, issuesOf, issuesOutsidePlan } from "./IssuesBlock";
import { LaterBlock } from "./LaterBlock";
import { Group } from "./PlanGroup";
import { stepsOf } from "./PlanSteps";
import { viewOf } from "./plan-marks";
import { SubagentsAtWork } from "./SubagentsAtWork";
import { subagentsAtWork } from "./subagents-at-work";
import { SvPanel, type PanelState, type Place } from "./SvPanel";
import { WatchingBlock } from "./WatchingBlock";
import { TurnAccountBlock, type LiveOpen } from "./TurnAccountBlock";
import "./progress-takes.css";

/** One named part of the panel: a small quiet label over what it holds. */
function Zone({ label, children }: { label: string; children: ReactNode }) {
  return (
    <section className="sv-zone" aria-label={label}>
      <h4 className="sv-zone__label">{label}</h4>
      {children}
    </section>
  );
}

export function ProgressPanel({ work, place, id, session, manifestId, acts, ed, onOpenSketch, onOpenSession, live }: { work: SessionWork; place: Place; id: string; session: Session; manifestId?: string | undefined; acts: SessionWorkActs | undefined; ed: PlanEditing | undefined; onOpenSketch: (sketch: string) => void; onOpenSession: (id: string) => void; live?: LiveOpen | undefined }) {
  const plan = work.plan;
  const flow = work.railOff === true ? undefined : work.workflow;
  const finished = plan !== undefined && plan.approved && plan.groups.every((group) => group.tasks.every((task) => task.state === "done" || task.state === "handed_in"));
  const state: PanelState | undefined = plan === undefined || !plan.approved ? undefined : finished ? "done" : "live";
  const sketch = plan?.sketch;
  // With no plan to show them, the agent's tasks and the subagents at work are listed. With none, a bare None select is all there would be: the panel stands folded to its band, and a press opens it.
  const planned = plan !== undefined && ed !== undefined;
  const tasks = planned ? [] : (work.tasks ?? []);
  const atWork = planned ? [] : subagentsAtWork(session);
  // Issues and pull requests are drawn with a plan or without one, and a Session with only those is not folded.
  const { issues, loose } = issuesOf(session);
  const account = accountOf(session);
  const workingGroup = accountGroupOf(session)?.id;
  const draft = useSessionsDraft();
  const later = session.later ?? [];
  const watching = (session.watches ?? []).length > 0;
  const bare = account === undefined && issues.length === 0 && later.length === 0 && !watching && loose.length === 0 && !planned && tasks.length === 0 && atWork.length === 0 && flow === undefined && work.railOff !== true && work.proposal?.workflow === null && acts !== undefined;
  const groups = plan === undefined || ed === undefined ? [] : viewOf(plan, ed.marks);
  const steps = stepsOf(groups, plan?.approved === true, workingGroup);
  const now = account === undefined ? null : <TurnAccountBlock session={session} lines={account} live={live} />;
  const stepper = plan === undefined || ed === undefined ? null : (
    <div className="sv-plan">
      <div className="sv-plan__groups">
        {groups.map((group) => (
          <Group key={group.id} group={group} step={steps.get(group.id)} ed={ed} parent={session} manifestId={manifestId} onOpenSession={onOpenSession} />
        ))}
      </div>
      <div className="sv-plan__foot">
        <AddGroup ed={ed} />
        <SendChanges ed={ed} />
      </div>
    </div>
  );
  const outside = issuesOutsidePlan(session);
  const hasIssues = outside.issues.length > 0 || outside.loose.length > 0;
  const agentWork = (
    <>
      {tasks.length === 0 ? null : <AgentTasks tasks={tasks} />}
      {atWork.length === 0 ? null : <SubagentsAtWork rows={atWork} />}
    </>
  );
  // Now, then the plan, then the issues, each a named zone.
  const zoned = (
    <>
      {now === null ? null : <Zone label="Now">{now}</Zone>}
      {agentWork}
      {stepper === null ? null : <Zone label="Plan">{stepper}</Zone>}
      {hasIssues ? (
        <Zone label="Issues">
          <IssuesBlock session={session} manifestId={manifestId} />
        </Zone>
      ) : null}
    </>
  );
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
    <SvPanel label="Progress" area="progress" place={place} state={state} acts={bar} empty={bare}>
      {zoned}
      <WatchingBlock session={session} />
      {later.length === 0 ? null : <LaterBlock later={later} onRemind={(sentence) => void remindLater(draft, id, sentence)} onDrop={(one) => draft?.dropLater?.(id, one.id)} />}
    </SvPanel>
  );
}
