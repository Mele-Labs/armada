// A rule on a Job's lanes, in the side panel. One that is waiting on the person for something shows the drafted ask as its own card, with its acts, answered here; the card is the
// thread's (`DraftRow`), so Send, Edit and Withdraw are the ones the thread shows and answering here settles the same item. One that has acts Fleet kept says what each is and where it
// stands, for each instance it was done to.

import { useRef } from "react";
import { Ban, Check, Clock, MessageCircleQuestion } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { DraftRow, Tooltip } from "@armada/components";
import type { DraftAnswer } from "@armada/components";

import { dismissWaiting } from "../cockpit/dismissed";
import { useSessions, useSessionsDraft } from "../sessions-draft";
import { SidePanel } from "./side";
import { useBack } from "./panels";
import { closePanel } from "./store";
import type { PanelItem } from "./store";

/** How an act stands, as Fleet keeps it: the glyph the rule's chip uses for the same, and the word. */
const STANDING: Readonly<Record<string, { Glyph: LucideIcon; said: string }>> = {
  planned: { Glyph: Clock, said: "Planned" },
  asked: { Glyph: MessageCircleQuestion, said: "Asked" },
  stuck: { Glyph: MessageCircleQuestion, said: "Stuck" },
  done: { Glyph: Check, said: "Done" },
  skipped: { Glyph: Ban, said: "Skipped" },
  told: { Glyph: Check, said: "Told the agent" },
  not_carried: { Glyph: Clock, said: "Not carried" },
};

/** What a rule did on each instance, from what the Job's rules kept. */
function ActsPanel({ item }: { item: Extract<PanelItem, { kind: "rule" }> }) {
  const session = useSessions().find((one) => one.id === item.sessionId);
  const back = useBack();
  const job = session?.lanes?.job;
  const rule = job?.copy.rules.find((one) => one.id === item.rule);
  const done = (job?.fired ?? []).filter((one) => one.rule === item.rule && (one.acts?.length ?? 0) > 0);
  if (session === undefined || job === undefined || rule === undefined || done.length === 0) return null;
  return (
    <SidePanel title={`Rule: ${rule.label}`} heading={rule.label} crumb="Rule" back={back} onClose={closePanel}>
      <div className="sv-lanes__acts">
        {done.map((one) => (
          <section key={one.instance} className="sv-lanes__triage" aria-label={job.instances.find((each) => each.id === one.instance)?.label ?? rule.label}>
            <h4 className="sv-lanes__triage-head">{job.instances.find((each) => each.id === one.instance)?.label ?? rule.label}</h4>
            <ul className="sv-lanes__comments">
              {(one.acts ?? []).map((act, at) => {
                const { Glyph, said } = STANDING[act.standing] ?? { Glyph: Clock, said: act.standing };
                return (
                  <li key={at} className="sv-lanes__item">
                    <div className="sv-lanes__act" data-standing={act.standing}>
                      <span className="sv-lanes__read">
                        <Glyph size={16} strokeWidth={2} aria-hidden />
                        <span>{said}</span>
                      </span>
                      <span className="sv-lanes__words">
                        <span className="sv-lanes__text">{act.text}</span>
                        {act.said === undefined ? null : <span className="sv-lanes__why">{act.said}</span>}
                      </span>
                      <span className="sv-lanes__course">{act.course === "asks" ? "Asks" : "Acts"}</span>
                      <Tooltip asChild label="Time">
                        <span className="sv-lanes__at">{act.at}</span>
                      </Tooltip>
                    </div>
                  </li>
                );
              })}
            </ul>
          </section>
        ))}
      </div>
    </SidePanel>
  );
}

export function RulePanel({ item }: { item: Extract<PanelItem, { kind: "rule" }> }) {
  const session = useSessions().find((one) => one.id === item.sessionId);
  const draft = useSessionsDraft();
  const back = useBack();
  const rule = session?.lanes?.job.copy.rules.find((one) => one.id === item.rule);
  const found = item.row === undefined ? undefined : session?.rows.find((one) => one.id === item.row);
  // **Sending leaves the panel as it is**: the card settles to what was sent, and the way back stays. The last card read is held, so a read that comes a beat late never takes the panel away.
  const held = useRef<{ row: Extract<NonNullable<typeof found>, { kind: "draft" }>; title: string } | undefined>(undefined);
  if (found !== undefined && found.kind === "draft") held.current = { row: found, title: rule?.label ?? found.title };
  if (item.row === undefined) return <ActsPanel item={item} />;
  if (session === undefined || held.current === undefined) return null;
  const { row, title } = held.current;
  return (
    <SidePanel title={`Rule: ${title}`} heading={title} crumb="Rule" back={back} onClose={closePanel}>
      <ul className="sv-lanes__ask" aria-label="Waiting on you">
        <DraftRow
          row={row}
          keep={`draft:${session.id}:${row.id}`}
          {...(draft?.answerWaiting === undefined ? {} : { onAnswer: (rowId: string, answer: DraftAnswer) => void draft.answerWaiting?.(session.id, `draft:${rowId}`, answer) })}
          {...(draft?.answerWaiting === undefined ? {} : { onWithdraw: (rowId: string, reason?: string) => void dismissWaiting(session.id, `draft:${rowId}`, reason) })}
        />
      </ul>
    </SidePanel>
  );
}
