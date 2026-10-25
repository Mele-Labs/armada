// One panel of a Session's view: a header band over a mono body, the state carried in the frame as the pool's bays are.
// `asks` is what the person is owed, `live` is work going on, `done` is work finished, and none is a panel at rest.
// `place` is where it stands (the Cockpit's left column, its right column, its foot), which decides how its body is laid out.

import { useState, type KeyboardEvent, type MouseEvent, type ReactNode } from "react";

import { Tooltip, useTeachMenu } from "@armada/components";
import { useLayout } from "@armada/shell";

import { useCollapse } from "../cockpit/deck/collapse";
import type { Fold } from "./story";
import { useTeachBridge } from "../teach";

export type PanelState = "asks" | "live" | "done";
export type Place = "left" | "side" | "foot" | "calls";

/**
 * `empty` is a panel with nothing to show: it stands folded to its band and gives its height to the panels round it, and opens on
 * its own once there is something. A press on the band still opens it and folds it again, for as long as it stays empty.
 */
export function SvPanel({ label, state, acts, rail, place, area, empty = false, fold, keycap, children }: { label: string; state?: PanelState | undefined; acts?: ReactNode; /** A line drawn in the band after the label: the Workflow panel's steps. */ rail?: ReactNode; place: Place; area: string; empty?: boolean; /** The story's say on whether the panel is open: the stage opens one and folds the rest, and a press or a key overrides it (`story.ts`). */ fold?: Fold | undefined; /** The key that opens and folds the panel, named on the label's tooltip. */ keycap?: string | undefined; children?: ReactNode }) {
  // Pressing the band folds the panel to it.
  const { collapsed: kept, band: own } = useCollapse(area);
  const [peek, setPeek] = useState(false);
  const collapsed = empty ? !peek : fold !== undefined ? !fold.open : kept;
  // A panel the layout lists can be hidden, and one it does not list is always drawn. Teach is offered on the frame, wherever the window offers it.
  const placed = useLayout("job.panels").all.find((one) => one.id === area);
  const teachBridge = useTeachBridge();
  const teach = useTeachMenu(`Panel ${label}`, teachBridge === undefined ? undefined : () => teachBridge("panel", label, [area]));
  if (placed !== undefined && !placed.visible) return null;
  const pressed = (expanded: boolean, flip: () => void) => ({
    role: "button",
    tabIndex: 0,
    "aria-expanded": expanded,
    onClick: (event: MouseEvent<HTMLElement>) => {
      const inside = (event.target as Element).closest("button, a, input, select, textarea, label");
      if (inside === null || inside === event.currentTarget) flip();
    },
    onKeyDown: (event: KeyboardEvent<HTMLElement>) => {
      if (event.target !== event.currentTarget || (event.key !== "Enter" && event.key !== " ")) return;
      event.preventDefault();
      flip();
    },
  });
  const band = empty ? pressed(peek, () => setPeek((was) => !was)) : fold !== undefined ? pressed(fold.open, fold.toggle) : own;
  return (
    <section className="sv-panel" data-area={area} data-place={place} data-state={state} data-collapsed={collapsed || undefined} data-empty={empty || undefined} data-band-only={children === undefined || undefined} aria-label={label} {...teach.on}>
      {teach.menu}
      <header className="sv-panel__band" {...band}>
        {keycap === undefined ? (
          <h3 className="sv-panel__label">{label}</h3>
        ) : (
          <Tooltip asChild label={collapsed ? `Open ${label}` : `Fold ${label}`} shortcut={keycap}>
            <h3 className="sv-panel__label">{label}</h3>
          </Tooltip>
        )}
        {rail}
        {acts === undefined ? null : <span className="sv-panel__acts">{acts}</span>}
      </header>
      <div className="sv-panel__body">{children}</div>
    </section>
  );
}
