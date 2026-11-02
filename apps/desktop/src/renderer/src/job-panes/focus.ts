// The key of a kind of tab: its tab comes to the front and its pane takes focus, or a tab of the kind opens where the Job has none.

import type { Session } from "@armada/screens/src/draft/sessions";

import { inPanes, type LayerId } from "../session-view/layers";
import { openInPanel, type PanelItem } from "../shared-panel/store";
import { KINDS } from "./kinds";
import { labelFor } from "./PaneView";
import { jobOpens, look, peek, rearrange } from "./store";
import { activate, addTab, arrive, holderOf, panesOf, type TabKind } from "./tree";

export function focusKind(jobId: string, kind: TabKind): void {
  const holder = holderOf(peek(jobId), kind);
  const tab =
    holder === undefined
      ? undefined
      : (holder.tabs.find((one) => one.id === holder.active && one.kind === kind) ??
        holder.tabs.find((one) => one.kind === kind));
  if (tab === undefined) return jobOpens(jobId, kind, labelFor(jobId, kind));
  look(jobId, (arrangement) => activate(arrangement, tab.id));
}

/** The tab that draws each view a Job's layers open over it, for a Job in panes. The Field and the document float over the whole Job and have none. */
const TAB_OF: Partial<Record<LayerId, TabKind>> = {
  artifacts: "ledger",
  checks: "checks",
  subagents: "subagents",
  slot: "bays",
  system: "system",
  lanes: "workflow",
  judge: "judge",
};

/** A view's key for a Job in panes: its tab comes to the front, or opens by the arrival rule. False where the view has no tab and opens over the Job. */
export function focusLayer(jobId: string, layer: LayerId): boolean {
  const kind = TAB_OF[layer];
  if (kind === undefined) return false;
  focusKind(jobId, kind);
  return true;
}

/** A draft's card as a tab named for the draft: its tab comes to the front where it has one, else it joins the thread's pane, or opens by the arrival rule with no thread. */
export function focusDraft(jobId: string, row: string, title: string): void {
  const arrangement = peek(jobId);
  const open = panesOf(arrangement.root)
    .flatMap((pane) => pane.tabs)
    .find((tab) => tab.kind === "draft" && tab.row === row);
  if (open !== undefined) return look(jobId, (one) => activate(one, open.id));
  const holder = holderOf(arrangement, "draft") ?? holderOf(arrangement, "thread");
  if (holder !== undefined)
    return look(jobId, (one) => addTab(one, holder.id, "draft", title, undefined, undefined, row));
  rearrange(
    jobId,
    (one) => arrive(one, "draft", title, undefined, undefined, row),
    undefined,
    true,
  );
}

/** Where a draft's card opens: its own tab on a Job drawn in panes, the side panel on any other and for a rule's draft. False where nothing opened. */
export function openDraftCard(session: Session, card: PanelItem): boolean {
  if (card.kind !== "draft" || !inPanes(session.id)) return openInPanel(card);
  const row = session.rows.find((one) => one.id === card.row);
  focusDraft(session.id, card.row, row?.kind === "draft" ? row.title : KINDS.draft.label);
  return true;
}
