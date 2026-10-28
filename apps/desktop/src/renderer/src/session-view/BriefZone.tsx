// The Job's brief as the first beat of its spine, above the zones: one line while shut, and pressed it opens in place to the whole text and how the Job started (workflow,
// delivery, model, effort, mode, and the Job it was started from). A row the Job holds no value for is left out. `session.started` is what Fleet sends; without it the grid
// reads what the Job holds now.

import { useState } from "react";
import { ChevronDown } from "lucide-react";
import type { DeliveryChoice } from "@armada/protocol";
import type { Session, SessionMode, SessionWork } from "@armada/screens/src/draft/sessions";

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

export function BriefZone({ session, work, text, onOpenSession }: { session: Session; work: SessionWork; text: string; onOpenSession: (id: string) => void }) {
  const [open, setOpen] = useState(false);
  const rows = startedRows(session, work);
  const by = session.started?.by;
  const body = `brief-${session.id}`;
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
    </section>
  );
}
