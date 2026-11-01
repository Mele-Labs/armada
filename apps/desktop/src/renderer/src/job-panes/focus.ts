// The key of a kind of tab: its tab comes to the front and its pane takes focus, or a tab of the kind opens where the Job has none.

import { labelFor } from "./PaneView";
import { jobOpens, look, peek } from "./store";
import { activate, holderOf, type TabKind } from "./tree";

export function focusKind(jobId: string, kind: TabKind): void {
  const holder = holderOf(peek(jobId), kind);
  const tab = holder === undefined ? undefined : (holder.tabs.find((one) => one.id === holder.active && one.kind === kind) ?? holder.tabs.find((one) => one.kind === kind));
  if (tab === undefined) return jobOpens(jobId, kind, labelFor(jobId, kind));
  look(jobId, (arrangement) => activate(arrangement, tab.id));
}
