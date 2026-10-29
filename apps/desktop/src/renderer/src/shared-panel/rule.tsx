// What a rule on a Job's lanes is waiting on the person for, in the side panel: the drafted ask as its own card, with its acts, answered here. The card is the thread's (`DraftRow`),
// so Send, Edit and Withdraw are the ones the thread shows and answering here settles the same item.

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
  const lanes = session?.lanes;
  const fired = lanes?.job.fired.find((one) => one.rule === item.rule && one.needs !== undefined);
  const rule = lanes?.job.copy.rules.find((one) => one.id === item.rule);
  const row = session?.rows.find((one) => one.id === fired?.needs);
  if (session === undefined || row === undefined || row.kind !== "draft") return null;
  return (
    <SidePanel title={`Rule: ${rule?.label ?? row.title}`} heading={rule?.label ?? row.title} crumb="Rule" back={back} onClose={closePanel}>
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
