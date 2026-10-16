// The Plan: what the agent recorded after planning with the person, in a panel of its own. The scope as paths, the task groups each
// naming the type of subagent that takes it, and the tasks with their state. It arrives with Approve in its band, framed in amber
// until he decides; he can answer in the thread to change it. Edits Fleet saw are marked on the group. Nothing here blocks.

import { BadgeCheck, Bot, PencilLine } from "lucide-react";
import { Button, TaskMark, Tooltip } from "@armada/components";
import type { WorkPlan } from "@armada/screens/src/draft/sessions";

import { SvPanel, type PanelState, type Place } from "./SvPanel";

export function Scope({ plan }: { plan: WorkPlan }) {
  return (
    <ul className="sv-scope" aria-label="Scope">
      {plan.scope.map((path) => (
        <li key={path}>
          <code>{path}</code>
        </li>
      ))}
    </ul>
  );
}

export function Groups({ plan }: { plan: WorkPlan }) {
  return (
    <>
      {plan.groups.map((group) => {
        const live = group.tasks.some((task) => task.state === "working");
        const complete = group.tasks.every((task) => task.state === "done");
        return (
          <section key={group.id} className="sv-group" data-state={complete ? "done" : live ? "live" : undefined} aria-label={group.title}>
            <header className="sv-group__head">
              <Tooltip label="Subagent type">
                <span className="sv-group__agent" role="img" aria-label={`Subagent type ${group.agent}`}>
                  <Bot size={12} strokeWidth={2} aria-hidden />
                  <code>{group.agent}</code>
                </span>
              </Tooltip>
              {group.tasks.some((task) => task.seen === true) ? (
                <Tooltip label="Edits seen">
                  <span className="sv-group__seen" role="img" aria-label="Edits seen">
                    <PencilLine size={12} strokeWidth={2} aria-hidden />
                  </span>
                </Tooltip>
              ) : null}
            </header>
            <h4 className="sv-group__title">{group.title}</h4>
            <ul className="sv-tasks">
              {group.tasks.map((task) => (
                <li key={task.id} className="sv-task" data-state={task.state}>
                  <TaskMark state={task.state} />
                  <span>{task.text}</span>
                </li>
              ))}
            </ul>
          </section>
        );
      })}
    </>
  );
}


export function PlanPanel({ plan, place, onApprove }: { plan: WorkPlan; place: Place; onApprove: () => void }) {
  const finished = plan.approved && plan.groups.every((group) => group.tasks.every((task) => task.state === "done"));
  const state: PanelState = !plan.approved ? "asks" : finished ? "done" : "live";
  const acts = plan.approved ? (
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
    <SvPanel label="Plan" area="plan" place={place} state={state} acts={acts}>
      <div className="sv-plan">
        <Scope plan={plan} />
        <div className="sv-plan__groups">
          <Groups plan={plan} />
        </div>
      </div>
    </SvPanel>
  );
}
