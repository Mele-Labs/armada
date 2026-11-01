// A tab mod a Job builds opens in that Job's panes as it appears in Fleet's list, by the arrival rule, and PaneView draws it as the mod it is.
// A tab started from the + → Custom composer is bound to its mod already and gets no second copy.

import { useEffect } from "react";
import type { ModSummary } from "@armada/protocol";

import { useModList } from "../mod-list";
import { tabTitle } from "./custom";
import { jobOpens, peek } from "./store";
import { panesOf } from "./tree";

/** The mods a Job has had a tab for, so a tab he closes stays closed. */
const seen = new Set<string>();

const bound = (jobId: string, mod: string): boolean => panesOf(peek(jobId).root).some((pane) => pane.tabs.some((tab) => tab.kind === "custom" && tab.mod === mod));

/** Opens a Custom tab bound to each `tab` mod this Job made that no tab of its panes draws, once. */
export function openBuiltTabs(jobId: string, mods: readonly ModSummary[]): void {
  for (const one of mods) {
    if (one.kind !== "tab" || one.made_by !== jobId) continue;
    const key = `${jobId}\n${one.name}`;
    if (seen.has(key)) continue;
    seen.add(key);
    if (!bound(jobId, one.name)) jobOpens(jobId, "custom", tabTitle(one.name), undefined, one.name);
  }
}

export function useBuiltTabs(jobId: string): void {
  const mods = useModList();
  useEffect(() => openBuiltTabs(jobId, mods), [jobId, mods]);
}
