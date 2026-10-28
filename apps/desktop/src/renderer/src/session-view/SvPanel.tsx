// One panel of a Session's view: a header band over a mono body, the state carried in the frame as the pool's bays are.
// `asks` is what the person is owed, `draft` is a plan waiting on its approval, `live` is work going on, `done` is work finished, `failed` is work that broke, and none is a panel at rest.
// `place` is where it stands (the Cockpit's left column, its right column, its foot), which decides how its body is laid out.

import { useState, type KeyboardEvent, type MouseEvent, type ReactNode } from "react";
import { GraduationCap } from "lucide-react";

import { DropdownMenu, Tooltip, useTeachMenu, type DropdownMenuEntry } from "@armada/components";
import { useLayout } from "@armada/shell";

import { useCollapse } from "../cockpit/deck/collapse";
import type { Fold } from "./story";
import { useTeachBridge } from "../teach";

export type PanelState = "asks" | "draft" | "live" | "done" | "failed";
export type Place = "left" | "side" | "foot" | "calls";

// The lamp at the strip's left, in the state's hue, and the word its tooltip says. A panel's own stage can name a state in its own words. A plan drafted and waiting on his
// approval is a call on him, so it is the needs lamp.
const MARK: Record<PanelState, { lamp: "needs" | "running" | "done" | "failed"; said: string }> = {
  asks: { lamp: "needs", said: "Waiting on you" },
  draft: { lamp: "needs", said: "Drafting" },
  live: { lamp: "running", said: "Live" },
  done: { lamp: "done", said: "Done" },
  failed: { lamp: "failed", said: "Failed" },
};
const SAID: Partial<Record<string, Partial<Record<PanelState, string>>>> = {
  blueprint: { live: "Approved" },
  landing: { live: "Waiting", done: "Landed" },
};

function StateMark({ state, said }: { state: PanelState; said: string }) {
  return (
    <Tooltip asChild label={said}>
      <span className="sv-panel__lamp" data-lamp={MARK[state].lamp} role="img" aria-label={said} />
    </Tooltip>
  );
}

/** A panel with nothing to show stays open: it is never folded for being empty, only by a press (`story.ts`). */
/** The layout entry each kind of step zone answers to: finished steps to Completed, the current one to Now, the ones to come to Next. */
const STEP_ZONE: Readonly<Record<string, string>> = { "step-done": "done", "step-now": "now", "step-next": "next" };

/** What the band's own menu offers, on a right press or the ContextMenu key or Shift+F10 on the band. Teach joins it where the window offers Teach. */
export type BandMenu = { entries: readonly DropdownMenuEntry[]; onSelect: (id: string) => void };

const opensMenu = (event: KeyboardEvent<HTMLElement>) => event.key === "ContextMenu" || (event.key === "F10" && event.shiftKey);

export function SvPanel({ label, state, acts, place, area, fold, keycap, marker, skipped, menu, children }: { label: string; state?: PanelState | undefined; acts?: ReactNode; place: Place; area: string; /** The story's say on whether the panel is open and whether the stage is at it: open at every stage unless pressed shut, and the current one takes the larger share (`story.ts`). */ fold?: Fold | undefined; /** The key that opens and folds the panel, named on the label's tooltip. */ keycap?: string | undefined; /** A mark of its own on the spine line, in place of the dot. */ marker?: ReactNode; /** A step the person skipped: drawn muted, with the word. */ skipped?: boolean; menu?: BandMenu | undefined; children?: ReactNode }) {
  // Pressing the band folds the panel to it.
  const { collapsed: kept, band: own } = useCollapse(area);
  const collapsed = fold !== undefined ? !fold.open : kept;
  // A panel the layout lists can be hidden, and one it does not list is always drawn. A workflow's step zones answer to the zone they stand for. Teach is offered on the frame,
  // wherever the window offers it.
  const listed = STEP_ZONE[area] ?? area;
  const placed = useLayout("job.panels").all.find((one) => one.id === listed);
  const teachBridge = useTeachBridge();
  const teach = useTeachMenu(`Panel ${label}`, teachBridge === undefined ? undefined : () => teachBridge("panel", label, [area]));
  const [menuAt, setMenuAt] = useState<{ x: number; y: number; from: HTMLElement } | undefined>(undefined);
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
  const folding = fold !== undefined ? pressed(fold.open, fold.toggle) : own;
  // The band's menu is opened before the frame's Teach menu sees the press, which a press already handled leaves shut.
  const band =
    menu === undefined
      ? folding
      : {
          ...folding,
          onContextMenu: (event: MouseEvent<HTMLElement>) => {
            event.preventDefault();
            setMenuAt({ x: event.clientX, y: event.clientY, from: event.currentTarget });
          },
          onKeyDown: (event: KeyboardEvent<HTMLElement>) => {
            if (!opensMenu(event)) return folding.onKeyDown?.(event);
            event.preventDefault();
            const box = event.currentTarget.getBoundingClientRect();
            setMenuAt({ x: box.left + box.width / 2, y: box.bottom, from: event.currentTarget });
          },
        };
  const bandMenu =
    menu === undefined || menuAt === undefined ? null : (
      <DropdownMenu
        triggerLabel={label}
        at={{ x: menuAt.x, y: menuAt.y }}
        open
        entries={[...menu.entries, ...(teachBridge === undefined ? [] : [{ kind: "separator" as const, id: "teach-rule" }, { kind: "item" as const, id: "teach", label: "Teach", icon: GraduationCap }])]}
        onOpenChange={(open) => {
          if (open) return;
          setMenuAt(undefined);
          menuAt.from.focus();
        }}
        onSelect={(chosen) => (chosen === "teach" ? teachBridge?.("panel", label, [area]) : menu.onSelect(chosen))}
      />
    );
  return (
    <section className="sv-panel" data-area={area} data-place={place} data-state={state} data-collapsed={collapsed || undefined} data-current={fold?.current || undefined} data-marked={marker !== undefined || undefined} data-band-only={children === undefined || undefined} data-skipped={skipped || undefined} aria-label={label} {...teach.on}>
      {teach.menu}
      {bandMenu}
      <header className="sv-panel__band" {...band}>
        {marker}
        {state === undefined ? null : <StateMark state={state} said={SAID[area]?.[state] ?? MARK[state].said} />}
        {keycap === undefined ? (
          <h3 className="sv-panel__label">{label}</h3>
        ) : (
          <Tooltip asChild label={collapsed ? `Open ${label}` : `Fold ${label}`} shortcut={keycap}>
            <h3 className="sv-panel__label">{label}</h3>
          </Tooltip>
        )}
        {skipped === true ? <span className="sv-panel__skipped">Skipped</span> : null}
        {acts === undefined ? null : <span className="sv-panel__acts">{acts}</span>}
      </header>
      <div className="sv-panel__body">{children}</div>
    </section>
  );
}
