// The Dashboard's filters, and which of Overview's sections each one reads. The ids are the tabs'
// they replaced (`running` is Active). Your move was folded into Active on 13 Oct 2026, so a call
// that needs the owner is read there and a remembered `command-central` reads as Active.
//
// **A section belongs to one tab.** Needs you, Running and Queued are Active; Recently ended and
// Done are Done. Other, a Job no section claims, rides with Active so that no row is left off
// every tab.

import type { BoardSection } from "@armada/screens/src/board";

/** `bays` reads no section: it draws the worktree bays, what waits for one, and the merge line. */
export type DashboardTab = "running" | "done" | "bays";

/** The filters in the order they are drawn. Labels carry no count. */
export const DASHBOARD_TABS: readonly { id: DashboardTab; label: string }[] = [
  { id: "running", label: "Active" },
  { id: "done", label: "Done" },
  { id: "bays", label: "Bays" },
];

/** The tab a section is read on. */
export function dashboardTabOf(section: BoardSection): DashboardTab {
  return section === "recently-ended" || section === "done" ? "done" : "running";
}

/** The sections a tab reads. */
export function sectionsOfTab(tab: DashboardTab): readonly BoardSection[] {
  const all: readonly BoardSection[] = ["needs-you", "running", "queued", "recently-ended", "done", "other"];
  return all.filter((section) => dashboardTabOf(section) === tab);
}

/** A stored tab, or Active where nothing usable was stored. `command-central`, Your move's id, is Active now. */
export function dashboardTabNamed(value: string | null): DashboardTab {
  return DASHBOARD_TABS.find((tab) => tab.id === value)?.id ?? "running";
}
