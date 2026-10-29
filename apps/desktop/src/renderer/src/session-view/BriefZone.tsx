// The Job's brief as the first beat of its spine, above the zones: one line while shut, and pressed it opens in place to the whole text and how the Job started (workflow,
// delivery, model, effort, mode, and the Job it was started from). A row the Job holds no value for is left out. `session.started` is what Fleet sends; without it the grid
// reads what the Job holds now. Under it, open or shut, the tasks of his projects the Job holds, each with its project; a press opens one on Projects.

import { useState } from "react";
import { ChevronDown } from "lucide-react";
import { Tooltip } from "@armada/components";
import type { DeliveryChoice } from "@armada/protocol";
import type { Session, SessionMode, SessionWork } from "@armada/screens/src/draft/sessions";

import { askToShowTask, useTasksHeld, type HeldTask } from "../projects-tab/held";

const DELIVERY: Record<DeliveryChoice, string> = { auto_land: "Auto land", open_pr: "Open PR only", keep_local: "Keep local" };
const MODE: Record<SessionMode, string> = { ask: "Ask", auto: "Auto", accept_edits: "Accept edits", plan: "Plan" };

type Row = { label: string; value: string };

/** How the Job started, one row for each thing it holds a value for. */
export function startedRows(session: Session, work: SessionWork): Row[] {
  const from = session.started;
  const delivery = from?.delivery ?? session.start?.picked?.choice;
  const rows: (Row | undefined)[] = [
    { label: "Workflow", value: from?.workflow ?? work.workflow?.name ?? "" },
    { label: "Delivery", value: delivery === undefined ? "" : DELIVERY[delivery] },
    { label: "Model", value: from?.model ?? session.model ?? "" },
    { label: "Effort", value: from?.effort ?? session.effort ?? "" },
    { label: "Mode", value: (from?.mode ?? session.mode) === undefined ? "" : MODE[(from?.mode ?? session.mode)!] },
  ];
  return rows.filter((one): one is Row => one !== undefined && one.value !== "");
}

/** The tasks the Job holds, as a row of How it started is drawn: the label, then a line each with its project. */
function HeldTasks({ tasks }: { tasks: readonly HeldTask[] }) {
  const label = tasks.length === 1 ? "Task" : "Tasks";
  return (
    <dl className="sv-brief__held">
      <div className="sv-brief__row">
        <dt>{label}</dt>
        <dd>
          <ul className="sv-brief__tasks" aria-label={label}>
            {tasks.map((task) => (
              <li key={task.id}>
                <Tooltip asChild label="Open on Projects">
                  <button type="button" className="sv-brief__task" aria-label={`Open ${task.title} on Projects`} onClick={() => askToShowTask(task.id)}>
                    <span className="sv-brief__task-title">{task.title}</span>
                    <span className="sv-brief__task-project">{task.project}</span>
                  </button>
                </Tooltip>
              </li>
            ))}
          </ul>
        </dd>
      </div>
    </dl>
  );
}

export function BriefZone({ session, work, text, onOpenSession }: { session: Session; work: SessionWork; text: string; onOpenSession: (id: string) => void }) {
  const [open, setOpen] = useState(false);
  const rows = startedRows(session, work);
  const by = session.started?.by;
  const body = `brief-${session.id}`;
  const held = useTasksHeld(session.id);
  return (
    <section className="sv-panel" data-area="brief" data-place="side" aria-label="Brief">
      <header className="sv-panel__band">
        <button type="button" className="sv-brief__press" aria-label="Brief" aria-expanded={open} aria-controls={body} onClick={() => setOpen(!open)}>
          <span className="sv-panel__label">Brief</span>
          {open ? null : <span className="sv-brief__excerpt">{text}</span>}
          <ChevronDown className="sv-brief__caret" size={14} strokeWidth={2} aria-hidden />
        </button>
      </header>
      <div id={body} className="sv-panel__body" hidden={!open}>
        {!open ? null : (
          <>
            <p className="sv-brief__text">{text}</p>
            {rows.length === 0 && by === undefined ? null : (
              <dl className="sv-brief__started" aria-label="How it started">
                {rows.map((row) => (
                  <div key={row.label} className="sv-brief__row">
                    <dt>{row.label}</dt>
                    <dd>{row.value}</dd>
                  </div>
                ))}
                {by === undefined ? null : (
                  <div className="sv-brief__row">
                    <dt>Started by</dt>
                    <dd>
                      <button type="button" className="sv-brief__by" onClick={() => onOpenSession(by.job)}>
                        {by.title}
                      </button>
                    </dd>
                  </div>
                )}
              </dl>
            )}
          </>
        )}
      </div>
      {held.length === 0 ? null : <HeldTasks tasks={held} />}
    </section>
  );
}
