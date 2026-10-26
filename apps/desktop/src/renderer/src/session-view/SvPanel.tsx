// One panel of a Session's view: a header band over a mono body, the state carried in the frame as the pool's bays are.
// `asks` is what the person is owed, `draft` is a plan waiting on its approval, `live` is work going on, `done` is work finished, `failed` is work that broke, and none is a panel at rest.
// `place` is where it stands (the Cockpit's left column, its right column, its foot), which decides how its body is laid out.

import { useState, type KeyboardEvent, type MouseEvent, type ReactNode } from "react";
import { CircleCheck, CircleDashed, CircleX, Megaphone, type LucideIcon } from "lucide-react";

import { TASK_GLYPH, Tooltip, useTeachMenu } from "@armada/components";
import { useLayout } from "@armada/shell";

import { useCollapse } from "../cockpit/deck/collapse";
import type { Fold } from "./story";
import { useTeachBridge } from "../teach";

export type PanelState = "asks" | "draft" | "live" | "done" | "failed";
export type Place = "left" | "side" | "foot" | "calls";

// The glyph that opens a band, in the band's state hue, and the word its tooltip says. A panel's own stage can name a state in its own words.
const MARK: Record<PanelState, { Glyph: LucideIcon; said: string }> = {
  asks: { Glyph: Megaphone, said: "Waiting on you" },
  draft: { Glyph: CircleDashed, said: "Drafting" },
  live: { Glyph: TASK_GLYPH.working, said: "Live" },
  done: { Glyph: CircleCheck, said: "Done" },
  failed: { Glyph: CircleX, said: "Failed" },
};
const SAID: Partial<Record<string, Partial<Record<PanelState, string>>>> = {
  blueprint: { live: "Approved" },
  landing: { live: "Waiting", done: "Landed" },
};

function StateMark({ state, said }: { state: PanelState; said: string }) {
  const { Glyph } = MARK[state];
  return (
    <Tooltip asChild label={said}>
      <span className="sv-panel__state armada-session-mark" role="img" aria-label={said} {...(state === "live" ? { "data-pulsing": "" } : {})}>
        <Glyph size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

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
        {state === undefined || rail !== undefined ? null : <StateMark state={state} said={SAID[area]?.[state] ?? MARK[state].said} />}
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
