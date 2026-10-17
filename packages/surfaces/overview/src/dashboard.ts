// The Dashboard's filters. The ids are the tabs' they replaced (`command-central` is Your move,
// `running` is Active), so a remembered choice holds.

/** `bays` draws the worktree bays, what waits for one, and the merge line. */
export type DashboardTab = "command-central" | "running" | "done" | "bays";

/** The filters in the order they are drawn. Labels carry no count. */
export const DASHBOARD_TABS: readonly { id: DashboardTab; label: string }[] = [
  { id: "command-central", label: "Your move" },
  { id: "running", label: "Active" },
  { id: "done", label: "Done" },
  { id: "bays", label: "Bays" },
];

/** A stored tab, or Command Central where nothing usable was stored. */
export function dashboardTabNamed(value: string | null): DashboardTab {
  return DASHBOARD_TABS.find((tab) => tab.id === value)?.id ?? "command-central";
}
