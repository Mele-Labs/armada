// A rule on a Job's lanes, in the side panel. One that is waiting on the person for something shows the drafted ask as its own card, with its acts, answered here; the card is the
// thread's (`DraftRow`), so Send, Edit and Withdraw are the ones the thread shows and answering here settles the same item. One that has acts Fleet kept says what each is and where it
// stands, for each instance it was done to. A draft no rule left is answered the same way from its own card: here for a Job not in panes, in a Draft tab for one in panes.

import { useRef } from "react";
import { Ban, Check, Clock, MessageCircleQuestion } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { DraftRow, Tooltip } from "@armada/components";
import type { DraftAnswer } from "@armada/components";
import type { Session } from "@armada/screens/src/draft/sessions";

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

type DraftCardRow = Extract<Session["rows"][number], { kind: "draft" }>;

/** The draft card as last read. Held, so a read that comes a beat late never takes the card away. */
function useHeldDraft(session: Session | undefined, rowId: string): DraftCardRow | undefined {
  const found = session?.rows.find((one) => one.id === rowId);
  const held = useRef<DraftCardRow | undefined>(undefined);
  if (found !== undefined && found.kind === "draft") held.current = found;
  return held.current;
}

/** A draft card from the Job's thread with the thread's answers, so answering it settles the same item. **Sending leaves the card where it is**, settled to what was sent. */
export function DraftCard({ sessionId, rowId }: { sessionId: string; rowId: string }) {
  const session = useSessions().find((one) => one.id === sessionId);
  const draft = useSessionsDraft();
  const row = useHeldDraft(session, rowId);
  if (session === undefined || row === undefined) return null;
  return (
    <ul className="sv-lanes__ask" aria-label="Waiting on you">
      <DraftRow
        row={row}
        keep={`draft:${session.id}:${row.id}`}
        {...(draft?.answerWaiting === undefined ? {} : { onAnswer: (rowId: string, answer: DraftAnswer) => void draft.answerWaiting?.(session.id, `draft:${rowId}`, answer) })}
        {...(draft?.answerWaiting === undefined ? {} : { onWithdraw: (rowId: string, reason?: string) => void dismissWaiting(session.id, `draft:${rowId}`, reason) })}
      />
    </ul>
  );
}

/** A draft card answered beside the thread: `heading` where the panel is named for something other than the card, as a rule is. The way back stays once it is sent. */
function DraftSide({ sessionId, rowId, crumb, heading }: { sessionId: string; rowId: string; crumb: string; heading?: string }) {
  const session = useSessions().find((one) => one.id === sessionId);
  const back = useBack();
  const row = useHeldDraft(session, rowId);
  if (session === undefined || row === undefined) return null;
  const title = heading ?? row.title;
  return (
    <SidePanel title={`${crumb}: ${title}`} heading={title} crumb={crumb} back={back} onClose={closePanel}>
      <DraftCard sessionId={sessionId} rowId={rowId} />
    </SidePanel>
  );
}

export function RulePanel({ item }: { item: Extract<PanelItem, { kind: "rule" }> }) {
  const rule = useSessions()
    .find((one) => one.id === item.sessionId)
    ?.lanes?.job.copy.rules.find((one) => one.id === item.rule);
  if (item.row === undefined) return <ActsPanel item={item} />;
  return <DraftSide sessionId={item.sessionId} rowId={item.row} crumb="Rule" {...(rule === undefined ? {} : { heading: rule.label })} />;
}

/** A draft a Job put to the person, answered beside the thread it may sit far back in, on a Job not drawn in panes. One in panes opens as a tab (`job-panes/focus.ts`). */
export function DraftPanel({ item }: { item: Extract<PanelItem, { kind: "draft" }> }) {
  return <DraftSide sessionId={item.sessionId} rowId={item.row} crumb="Draft" />;
}

/**
 * Where a draft waiting on the person (`draft:<card>`) is answered beside the Job: its rule's panel where a rule on the Job's lanes left it, its own card otherwise.
 * Absent for a proposed workflow, which Needs you answers from its facts, and for an item that is no draft card.
 */
export function draftPanelOf(session: Session, itemId: string): PanelItem | undefined {
  const rowId = itemId.startsWith("draft:") ? itemId.slice("draft:".length) : undefined;
  const row = rowId === undefined ? undefined : session.rows.find((one) => one.id === rowId);
  if (rowId === undefined || row === undefined || row.kind !== "draft") return undefined;
  const fired = session.lanes?.job.fired.find((one) => one.needs === rowId);
  if (fired !== undefined) return { kind: "rule", sessionId: session.id, rule: fired.rule, row: rowId };
  return row.workflow !== undefined || row.change !== undefined ? undefined : { kind: "draft", sessionId: session.id, row: rowId };
}
