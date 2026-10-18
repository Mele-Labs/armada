// The three starts of an empty Cockpit as the start menu of a game: Armada's name large, the starts as orders, the one the focus is on opened
// with the line that says what it does. Two takes to compare, by `?menu=`: `map` has the Cockpit's own battlefield behind it, which previews the
// focused start; `hangar` is three bays side by side, the focused one open. Keys, and which start was last, are `FirstRun.tsx`'s.

import { Tooltip, KbdChord } from "@armada/components";
import { ArmadaLockupHorizontal } from "@armada/brand";
import type { ReactNode } from "react";

import "./start-menu.css";

export type MenuTake = "map" | "hangar";

/** Which take the page is on. The take that is not chosen is deleted when one is. */
export const menuTake = (): MenuTake => (new URLSearchParams(window.location.search).get("menu") === "hangar" ? "hangar" : "map");

export type Order = { to: "folder" | "new" | "found"; title: string; line: string; off?: string };

/** The three starts, worded as orders. Each line says exactly what happens. */
export const ORDERS: readonly Order[] = [
  { to: "folder", title: "Dispatch to a repository", line: "Choose a folder on this Mac, with a Manifest or without." },
  { to: "new", title: "Launch a new project", line: "Name a new folder. The Job starts with git init." },
  { to: "found", title: "Bring in your Claude Code sessions", line: "Adds the folders listed in ~/.claude/projects." },
];

/** A start drawn flat for the hangar take: the shape of what it makes. */
function Schematic({ to }: { to: Order["to"] }) {
  const edge = { fill: "none", stroke: "currentColor", strokeWidth: 1.5 } as const;
  return (
    <svg className="armada-start__schematic" viewBox="0 0 160 96" aria-hidden="true">
      {to === "folder" ? (
        <>
          <rect x="4" y="4" width="84" height="88" rx="4" {...edge} />
          <rect x="94" y="4" width="62" height="52" rx="4" {...edge} />
          <rect x="94" y="62" width="62" height="30" rx="4" {...edge} />
          <path d="M14 20h40M14 32h56M14 44h28" {...edge} />
        </>
      ) : to === "new" ? (
        <rect x="30" y="14" width="100" height="68" rx="4" {...edge} strokeDasharray="6 6" />
      ) : (
        <>
          <rect x="4" y="4" width="68" height="40" rx="4" {...edge} />
          <rect x="88" y="4" width="68" height="40" rx="4" {...edge} />
          <rect x="4" y="52" width="68" height="40" rx="4" {...edge} />
          <rect x="88" y="52" width="68" height="40" rx="4" {...edge} strokeDasharray="6 6" />
        </>
      )}
    </svg>
  );
}

/** The keys that move through the menu, and the ones that pick. */
function Hints(): ReactNode {
  return (
    <div className="armada-start__hints" aria-hidden="true">
      <span><KbdChord keys={["↑", "↓"]} /> Move</span>
      <span><KbdChord keys={["1", "2", "3"]} /> Pick</span>
      <span><KbdChord keys={["↵"]} /> Select</span>
    </div>
  );
}

export function StartMenu({ take, at, last, orders, onPick, onFocus }: {
  take: MenuTake;
  /** The start the focus is on. */
  at: number;
  /** The start to focus when the menu opens: the one last picked. */
  last: number;
  orders: readonly Order[];
  onPick: (to: Order["to"], at: number) => void;
  onFocus: (at: number) => void;
}) {
  return (
    <div className="armada-start" data-take={take}>
      <ArmadaLockupHorizontal className="armada-start__title" height={take === "map" ? 88 : 64} title="Armada" />
      <div role="group" aria-label="Start from" className="armada-start__orders">
        {orders.map(({ to, title, line, off }, i) => {
          const open = i === at && off === undefined;
          const order = (
            <button
              key={to}
              type="button"
              className="armada-start__order"
              data-open={open ? "" : undefined}
              disabled={off !== undefined}
              autoFocus={i === last && off === undefined}
              aria-label={title}
              aria-describedby={`armada-start-line-${i}`}
              onClick={() => onPick(to, i)}
              onFocus={() => onFocus(i)}
              onMouseEnter={(event) => off === undefined && event.currentTarget.focus()}
            >
              <KbdChord keys={[String(i + 1)]} aria-hidden />
              {take === "hangar" ? <Schematic to={to} /> : null}
              <span className="armada-start__text">
                <span className="armada-start__name">{title}</span>
                <span className="armada-start__line" id={`armada-start-line-${i}`}>
                  <span>{line}</span>
                </span>
              </span>
            </button>
          );
          return off === undefined ? order : (
            <Tooltip key={to} label={off}>
              <span className="armada-start__off">{order}</span>
            </Tooltip>
          );
        })}
      </div>
      <Hints />
    </div>
  );
}
