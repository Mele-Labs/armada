// What a rule on a Job's lanes is waiting on the person for, in the side panel: the drafted ask as its own card, with its acts, answered here. The card is the thread's (`DraftRow`),
// so Send, Edit and Withdraw are the ones the thread shows and answering here settles the same item.

import { useRef } from "react";
import { DraftRow } from "@armada/components";
import type { DraftAnswer } from "@armada/components";

import { dismissWaiting } from "../cockpit/dismissed";
import { useSessions, useSessionsDraft } from "../sessions-draft";
import { SidePanel } from "./side";
import { useBack } from "./panels";
import { closePanel } from "./store";
import type { PanelItem } from "./store";

export function RulePanel({ item }: { item: Extract<PanelItem, { kind: "rule" }> }) {
  const session = useSessions().find((one) => one.id === item.sessionId);
  const draft = useSessionsDraft();
  const back = useBack();
  const rule = session?.lanes?.job.copy.rules.find((one) => one.id === item.rule);
  const found = session?.rows.find((one) => one.id === item.row);
  // **Sending leaves the panel as it is**: the card settles to what was sent, and the way back stays. The last card read is held, so a read that comes a beat late never takes the panel away.
  const held = useRef<{ row: Extract<NonNullable<typeof found>, { kind: "draft" }>; title: string } | undefined>(undefined);
  if (found !== undefined && found.kind === "draft") held.current = { row: found, title: rule?.label ?? found.title };
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
