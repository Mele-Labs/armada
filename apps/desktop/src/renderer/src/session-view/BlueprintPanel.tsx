// Blueprint: the Job's flight plan, at the top of its right column. It opens on the brief the Job was dispatched with and the workflow's steps (`FlightHead`).
// While the plan is being drafted it stands open and editable under them: the summary in his words, every group with its title and tasks, the sketch from the
// band, and Approve at the foot beside the marks he put on it. Once approved it is a row for each group with a tick for each task, and only the group being
// worked opens to its tasks and the live line (`FlightRows`). A button in the band, or ⌥N, shows the whole plan instead: each group over a line for each task
// with its state glyph, and the subagent type and paths open with the group. A Job with no plan lists the agent's own
// tasks and the subagents at work under the head. It is never folded by the stage (`story.ts`); the band or ⌥B folds it by hand.

import { BadgeCheck, ListTree, PencilRuler } from "lucide-react";
import { Button, Tooltip } from "@armada/components";
import type { Session, SessionWork, SessionWorkActs } from "@armada/screens/src/draft/sessions";

import { AskUpdate } from "../cockpit/deck/AskUpdate";
import { accountGroupOf, accountNowOf } from "../session-activity";
import { useSessions } from "../sessions-draft";
import { AgentTasks } from "./AgentTasks";
import { FlightHead, FlightRows } from "./FlightPlan";
import { inHandOf } from "./linked";
import { AddGroup, SendChanges, type PlanEditing } from "./PlanEdit";
import { Group } from "./PlanGroup";
import { stepsOf } from "./PlanSteps";
import { viewOf } from "./plan-marks";
import { SubagentsAtWork } from "./SubagentsAtWork";
import { subagentsAtWork } from "./subagents-at-work";
import { SvPanel, type PanelState, type Place } from "./SvPanel";
import { bandKeyFor, useFold, useWhole, WHOLE_KEY } from "./story";
import "./progress-takes.css";
import "./story.css";

export function BlueprintPanel({ work, place, id, session, manifestId, acts, ed, onOpenSketch, onOpenSession }: { work: SessionWork; place: Place; id: string; session: Session; manifestId?: string | undefined; acts: SessionWorkActs | undefined; ed: PlanEditing | undefined; onOpenSketch: (sketch: string) => void; onOpenSession: (id: string) => void }) {
  const plan = work.plan;
  const fold = useFold(session, "blueprint");
  const everything = useWhole(session.id);
  const planned = plan !== undefined && ed !== undefined;
  const drafting = planned && !plan.approved;
  const finished = plan !== undefined && plan.approved && plan.groups.every((group) => group.tasks.every((task) => task.state === "done" || task.state === "handed_in"));
  const broke = plan !== undefined && plan.approved && plan.groups.some((group) => group.tasks.some((task) => task.state === "failed"));
  const state: PanelState | undefined = plan === undefined ? undefined : !plan.approved ? "draft" : broke ? "failed" : finished ? "done" : "live";
  const todos = (session.activity?.todos ?? []).filter((one) => one.task === undefined);
  const atWork = planned ? [] : subagentsAtWork(session);
  const workingGroup = accountGroupOf(session)?.id;
  const sessions = useSessions();
  const now = accountNowOf(session);
  const groups = planned ? viewOf(plan, ed.marks) : [];
  const steps = stepsOf(groups, plan?.approved === true, workingGroup, inHandOf(sessions, session));
  const sketch = plan?.sketch;
  // An approved plan is read as the flight plan; the whole plan is a press away, and a plan with changes marked on it stands whole.
  const nowView = planned && plan.approved && !everything.whole && ed.marks.length === 0;
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
      {!planned || !plan.approved ? null : (
        <Tooltip label="Whole blueprint" shortcut={WHOLE_KEY.cap}>
          <Button variant="ghost" size="sm" iconOnly aria-label="Whole blueprint" aria-pressed={!nowView} onClick={everything.toggle}>
            <ListTree size={16} strokeWidth={2} aria-hidden />
          </Button>
        </Tooltip>
      )}
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
    <SvPanel label="Blueprint" area="blueprint" place={place} state={state} acts={bar} fold={fold} keycap={bandKeyFor("blueprint")}>
      <FlightHead session={session} work={work} id={id} manifestId={manifestId} acts={acts} />
      {drafting && plan.summary !== undefined ? <p className="sv-blueprint__summary">{plan.summary}</p> : null}
      {todos.length === 0 ? null : <AgentTasks todos={todos} working={session.turn.state === "working"} />}
      {atWork.length === 0 ? null : <SubagentsAtWork rows={atWork} />}
      {!planned || !nowView ? null : <FlightRows groups={groups} workingGroup={workingGroup} now={now} session={session} manifestId={manifestId} onOpenSession={onOpenSession} />}
      {!planned || nowView ? null : (
        <div className="sv-plan" data-drafting={drafting || undefined}>
          <div className="sv-plan__groups">
            {groups.map((group) => {
              const step = steps.get(group.id);
              return <Group key={group.id} group={group} {...(drafting && step !== undefined ? { step, opened: true } : group.id === workingGroup && now !== undefined ? { now } : {})} ed={ed} parent={session} manifestId={manifestId} onOpenSession={onOpenSession} />;
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
