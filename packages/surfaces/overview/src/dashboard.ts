// The Dashboard's three filters, and which of Overview's sections each one reads. The ids are the
// tabs' they replaced (`command-central` is Your move, `running` is Active), so a remembered choice holds.
//
// **A section belongs to one tab.** Needs you is Command Central, where a person is sent first;
// Running and Queued are Running; Recently ended and Done are Done. Other, a Job no section
// claims, rides with Running so that no row is left off every tab.

import type { BoardSection } from "@armada/screens/src/board";

/** `bays` reads no section: it draws the worktree bays, what waits for one, and the merge line. */
export type DashboardTab = "command-central" | "running" | "done" | "bays";

/** The filters in the order they are drawn. Labels carry no count. */
export const DASHBOARD_TABS: readonly { id: DashboardTab; label: string }[] = [
  { id: "command-central", label: "Your move" },
  { id: "running", label: "Active" },
  { id: "done", label: "Done" },
  { id: "bays", label: "Bays" },
];

/** The tab a section is read on. */
export function dashboardTabOf(section: BoardSection): DashboardTab {
  if (section === "needs-you") return "command-central";
  return section === "recently-ended" || section === "done" ? "done" : "running";
}

/** The sections a tab reads. */
export function sectionsOfTab(tab: DashboardTab): readonly BoardSection[] {
  const all: readonly BoardSection[] = ["needs-you", "running", "queued", "recently-ended", "done", "other"];
  return all.filter((section) => dashboardTabOf(section) === tab);
}

/** A stored tab, or Command Central where nothing usable was stored. */
export function dashboardTabNamed(value: string | null): DashboardTab {
  return DASHBOARD_TABS.find((tab) => tab.id === value)?.id ?? "command-central";
}
