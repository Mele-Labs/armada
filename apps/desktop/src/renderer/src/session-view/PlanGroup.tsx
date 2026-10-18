// A plan group, as Progress lists it: the type of subagent that takes it, and each task. Its approval is a card in the thread. Edits Fleet saw
// are marked on the group. Nothing here blocks.

import { useState } from "react";
import { Bot, ChevronDown, Link, PencilLine } from "lucide-react";
import { Button, LinkedSession, TaskMark, Tooltip } from "@armada/components";
import type { Session } from "@armada/screens/src/draft/sessions";

import { useSessions, useSessionsDraft } from "../sessions-draft";
import { GroupEditor, type PlanEditing } from "./PlanEdit";
import { childrenOf, linkedPropsOf } from "./linked";
import type { ViewGroup } from "./plan-marks";
import { wasOpen } from "./place";

/**
 * A group is a row that opens: closed it is its title, the type of subagent that takes it and a glyph for each task; open it shows its own paths and its tasks in full.
 * **A group handed to a Session carries that Session's row** under its own; one not handed
 * offers the hand off in its open body.
 */
export function Group({ group, ed, parent, onOpenSession }: { group: ViewGroup; ed: PlanEditing; parent: Session | undefined; onOpenSession: (id: string) => void }) {
  const [open, setOpen] = useState(() => wasOpen(ed.id, group.id));
  const sessions = useSessions();
  const draft = useSessionsDraft();
  const child = parent === undefined ? undefined : childrenOf(sessions, parent).find((one) => one.group === group.id)?.session;
  const handOff = draft?.handOff;
  const live = group.tasks.some((task) => task.state === "working");
  const complete = group.tasks.every((task) => task.state === "done" || task.state === "handed_in");
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
            <Button variant="ghost" size="sm" onClick={() => void handOff(ed.id, group.id)}>
              <Link size={12} strokeWidth={2} aria-hidden />
              Hand to a Session
            </Button>
          )}
        </div>
      ) : null}
    </section>
  );
}
