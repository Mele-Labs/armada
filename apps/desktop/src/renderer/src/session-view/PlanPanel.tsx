// The Plan: what the agent recorded after planning with the person, in a panel of its own. The task groups each
// naming the type of subagent that takes it, each opening to its own paths and its tasks. The plan lists no paths of its own. Its approval is a card in the thread, so while it waits the
// panel shows the plan with no band of its own; once approved the panel carries the plan for the rest of the Session. Edits Fleet saw are marked on the group. Nothing here blocks.

import { useState } from "react";
import { BadgeCheck, Bot, ChevronDown, Link, PencilLine, PencilRuler } from "lucide-react";
import { Button, LinkedSession, TaskMark, Tooltip } from "@armada/components";
import type { Session, WorkPlan } from "@armada/screens/src/draft/sessions";

import { AskUpdate } from "../cockpit/deck/AskUpdate";
import { useSessions, useSessionsDraft } from "../sessions-draft";
import { AddGroup, GroupEditor, SendChanges, type PlanEditing } from "./PlanEdit";
import { childrenOf, linkedPropsOf } from "./linked";
import { viewOf, type ViewGroup } from "./plan-marks";
import { wasOpen } from "./place";
import { SvPanel, type PanelState, type Place } from "./SvPanel";

const NOWHERE = () => undefined;

/**
 * A group is a row that opens: closed it is its title, the type of subagent that takes it and a glyph for each task; open it shows its own paths and its tasks in full.
 * **A group handed to a Session carries that Session's row** under its own; one not handed
 * offers the hand off in its open body.
 */
function Group({ group, ed, parent, onOpenSession }: { group: ViewGroup; ed: PlanEditing; parent: Session | undefined; onOpenSession: (id: string) => void }) {
  const [open, setOpen] = useState(() => wasOpen(ed.id, group.id));
  const sessions = useSessions();
  const draft = useSessionsDraft();
  const child = parent === undefined ? undefined : childrenOf(sessions, parent).find((one) => one.group === group.id)?.session;
  const handOff = draft?.handOff;
  const live = group.tasks.some((task) => task.state === "working");
  const complete = group.tasks.every((task) => task.state === "done");
  const failed = group.tasks.some((task) => task.state === "failed");
  return (
    <section className="sv-group" data-group={group.id} data-state={failed ? "failed" : complete ? "done" : live ? "live" : undefined} data-open={open || undefined} aria-label={group.title}>
      <button type="button" className="sv-group__row" aria-expanded={open} aria-label={group.title} onClick={() => setOpen((was) => !was)}>
        <span className="sv-group__title">{group.title}</span>
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
        {open ? null : (
          <span className="sv-group__marks">
            {group.tasks.map((task) => (
              <TaskMark key={task.id} state={task.state} />
            ))}
          </span>
        )}
      </button>
      {child === undefined ? null : (
        <div className="sv-group__linked">
          <LinkedSession {...linkedPropsOf(child, onOpenSession)} />
        </div>
      )}
      {open ? (
        <div className="sv-group__body">
          <GroupEditor group={group} ed={ed} />
          {child !== undefined || handOff === undefined ? null : (
            <Button variant="ghost" size="sm" onClick={() => handOff(ed.id, group.id)}>
              <Link size={12} strokeWidth={2} aria-hidden />
              Hand to a Session
            </Button>
          )}
        </div>
      ) : null}
    </section>
  );
}

export function PlanPanel({ id, plan, place, ed, onOpenSketch, onOpenSession }: { id: string; plan: WorkPlan; place: Place; ed: PlanEditing; onOpenSketch: (sketch: string) => void; onOpenSession?: (id: string) => void }) {
  const parent = useSessions().find((one) => one.id === id);
  const finished = plan.approved && plan.groups.every((group) => group.tasks.every((task) => task.state === "done"));
  const state: PanelState | undefined = !plan.approved ? undefined : finished ? "done" : "live";
  const sketch = plan.sketch;
  const acts = (
    <>
      {sketch === undefined ? null : (
        <Tooltip label="Sketch">
          <Button variant="ghost" size="sm" iconOnly aria-label="Sketch" onClick={() => onOpenSketch(sketch.id)}>
            <PencilRuler size={16} strokeWidth={2} aria-hidden />
          </Button>
        </Tooltip>
      )}
      <AskUpdate subject={`session:${id}`} />
      {plan.approved ? (
        <Tooltip label="Plan approved">
          <span className="sv-panel__mark" role="img" aria-label="Plan approved">
            <BadgeCheck size={16} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
      ) : null}
    </>
  );
  return (
    <SvPanel label="Plan" area="plan" place={place} state={state} acts={acts}>
      <div className="sv-plan">
        <div className="sv-plan__groups">
          {viewOf(plan, ed.marks).map((group) => (
            <Group key={group.id} group={group} ed={ed} parent={parent} onOpenSession={onOpenSession ?? NOWHERE} />
          ))}
        </div>
        <div className="sv-plan__foot">
          <AddGroup ed={ed} />
          <SendChanges ed={ed} />
        </div>
      </div>
    </SvPanel>
  );
}
