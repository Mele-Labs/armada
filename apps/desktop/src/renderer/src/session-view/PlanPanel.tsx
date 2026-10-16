// The Plan: what the agent recorded after planning with the person, in a panel of its own. The task groups each
// naming the type of subagent that takes it, each opening to its own paths and its tasks. The plan lists no paths of its own. It arrives with Approve in its band, framed in amber
// until he decides; he can answer in the thread to change it. Edits Fleet saw are marked on the group. Nothing here blocks.

import { useState } from "react";
import { BadgeCheck, Bot, ChevronDown, PencilLine } from "lucide-react";
import { Button, TaskMark, Tooltip } from "@armada/components";
import type { WorkGroup, WorkPlan } from "@armada/screens/src/draft/sessions";

import { SvPanel, type PanelState, type Place } from "./SvPanel";

/** A group is a row that opens: closed it is its title, the type of subagent that takes it and a glyph for each task; open it shows its own paths and its tasks in full. */
function Group({ group }: { group: WorkGroup }) {
  const [open, setOpen] = useState(false);
  const live = group.tasks.some((task) => task.state === "working");
  const complete = group.tasks.every((task) => task.state === "done");
  return (
    <section className="sv-group" data-state={complete ? "done" : live ? "live" : undefined} data-open={open || undefined} aria-label={group.title}>
      <button type="button" className="sv-group__row" aria-expanded={open} aria-label={group.title} onClick={() => setOpen((was) => !was)}>
        <span className="sv-group__head">
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
          <ChevronDown className="sv-group__chevron" size={12} strokeWidth={2} aria-hidden />
        </span>
        <span className="sv-group__title">{group.title}</span>
        {open ? null : (
          <span className="sv-group__marks">
            {group.tasks.map((task) => (
              <TaskMark key={task.id} state={task.state} />
            ))}
          </span>
        )}
      </button>
      {open ? (
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
      ) : null}
    </section>
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
        <div className="sv-plan__groups">
          {plan.groups.map((group) => (
            <Group key={group.id} group={group} />
          ))}
        </div>
      </div>
    </SvPanel>
  );
}
