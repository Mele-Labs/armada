// The Dashboard's tab strip: Command Central, Running, Done. It replaced the summary tiles. A tab
// carries a live glyph and no count: a beacon that pulses while something needs the owner, a
// spinner while anything runs, a tick for what is over. A glowing underline slides to the active
// tab, and a hover on a glyph names it.

import type { KeyboardEvent, ReactNode } from "react";
import { CircleCheck, LoaderCircle, type LucideIcon } from "lucide-react";
import { Tooltip } from "@armada/components";
import { useLayout } from "@armada/shell";
import { DASHBOARD_TABS, type DashboardTab } from "./dashboard";

const HUE: Record<DashboardTab, string> = { "command-central": "ask", running: "running", done: "ok" };

export function DashboardTabs({
  tab,
  onTab,
  asking = false,
  running = false,
}: {
  tab: DashboardTab;
  onTab: (tab: DashboardTab) => void;
  /** Whether anything needs the owner: the beacon pulses. */
  asking?: boolean;
  /** Whether any Job runs: the spinner turns. */
  running?: boolean;
}) {
  // The layout's order and what it hides; a hidden tab still shows while it is the one open.
  const layout = useLayout("dashboard.tabs");
  const tabs = layout.all.filter((one) => one.visible || one.id === tab).map((one) => DASHBOARD_TABS.find((known) => known.id === one.id)).filter((one) => one !== undefined);
  const at = Math.max(0, tabs.findIndex((one) => one.id === tab));
  const glyph = (id: DashboardTab): { label: string; node: ReactNode } => {
    if (id === "command-central") return { label: asking ? "Something needs you" : "Nothing needs you", node: <span className="armada-dtabs__beacon" data-on={asking || undefined} /> };
    const Icon: LucideIcon = id === "running" ? LoaderCircle : CircleCheck;
    return { label: id === "running" ? (running ? "Running" : "Nothing running") : "Done", node: <Icon size={20} aria-hidden="true" data-live={id === "running" && running ? "" : undefined} /> };
  };
  const onKey = (event: KeyboardEvent) => {
    const step = event.key === "ArrowRight" ? 1 : event.key === "ArrowLeft" ? -1 : 0;
    if (step === 0) return;
    event.preventDefault();
    onTab(tabs[(at + step + tabs.length) % tabs.length]!.id);
  };
  return (
    <div className="armada-dtabs" role="tablist" data-hue={HUE[tab]} style={{ ["--at" as string]: at, ["--tabs" as string]: tabs.length }} onKeyDown={onKey}>
      {tabs.map(({ id, label }) => {
        const { label: named, node } = glyph(id);
        return (
          <button key={id} type="button" role="tab" aria-selected={id === tab} tabIndex={id === tab ? 0 : -1} className="armada-dtabs__tab" data-hue={HUE[id]} onClick={() => onTab(id)}>
            <Tooltip label={named}>
              <span className="armada-dtabs__glyph">{node}</span>
            </Tooltip>
            {label}
          </button>
        );
      })}
      <span className="armada-dtabs__bar" aria-hidden="true" />
    </div>
  );
}
