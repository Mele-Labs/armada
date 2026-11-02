// The key of a kind of tab: its tab comes to the front and its pane takes focus, or a tab of the kind opens where the Job has none.

import type { LayerId } from "../session-view/layers";
import { labelFor } from "./PaneView";
import { jobOpens, look, peek } from "./store";
import { activate, holderOf, type TabKind } from "./tree";

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
};

/** A view's key for a Job in panes: its tab comes to the front, or opens by the arrival rule. False where the view has no tab and opens over the Job. */
export function focusLayer(jobId: string, layer: LayerId): boolean {
  const kind = TAB_OF[layer];
  if (kind === undefined) return false;
  focusKind(jobId, kind);
  return true;
}
