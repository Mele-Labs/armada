// A tab mod a Job builds opens in that Job's panes as it appears in Fleet's list, by the arrival rule, and PaneView draws it as the mod it is.
// A tab started from the + → Custom composer is bound to its mod already and gets no second copy. A Mod Job that builds a Custom tab has no panes of its own to show it: its tab opens in the
// panes of the Job it was started from.

import { useEffect } from "react";
import type { ModSummary } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import { useModList } from "../mod-list";
import { useSessions } from "../sessions-draft";
import { peekCustom, tabTitle } from "./custom";
import { jobOpens, peek } from "./store";
import { panesOf } from "./tree";

/** The mods a Job has had a tab for, so a tab he closes stays closed. */
const seen = new Set<string>();

/** A tab is bound to the mod once it has the mod's name, or while the Job that is building it is the one that made the mod. */
const bound = (jobId: string, mod: string, maker: string): boolean =>
  panesOf(peek(jobId).root).some((pane) => pane.tabs.some((tab) => tab.kind === "custom" && (tab.mod === mod || peekCustom(tab.id).job === maker)));

/**
 * Opens a Custom tab bound to each `tab` mod this Job made, or that a Mod Job started from this Job made, that no tab of its panes draws, once. A Mod Job started from a tab
 * opens nothing in its own panes.
 */
export function openBuiltTabs(jobId: string, mods: readonly ModSummary[], sessions: readonly Pick<Session, "id" | "modFor">[] = []): void {
  const tabJobs = new Set(sessions.filter((one) => one.modFor !== undefined).map((one) => one.id));
  const makers = new Set([...(tabJobs.has(jobId) ? [] : [jobId]), ...sessions.filter((one) => one.modFor?.job === jobId).map((one) => one.id)]);
  for (const one of mods) {
    if (one.kind !== "tab" || one.made_by === undefined || !makers.has(one.made_by)) continue;
    const key = `${jobId}\n${one.name}`;
    if (seen.has(key)) continue;
    seen.add(key);
    if (!bound(jobId, one.name, one.made_by)) jobOpens(jobId, "custom", tabTitle(one.name), undefined, one.name);
  }
}

export function useBuiltTabs(jobId: string): void {
  const mods = useModList();
  const sessions = useSessions();
  useEffect(() => openBuiltTabs(jobId, mods, sessions), [jobId, mods, sessions]);
}
