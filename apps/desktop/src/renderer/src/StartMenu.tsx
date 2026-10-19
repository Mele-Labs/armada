// The three starts of an empty Cockpit as the start menu of a game: a clear column on the left (the name spaced wide, the starts as orders, the focused
// one on a solid slanted bar, its line under the menu) and the Cockpit's own battlefield on the right as the hero, which previews the focused start.
// Keys, and which start was last, are `FirstRun.tsx`'s.

import { Tooltip, KbdChord } from "@armada/components";
import type { ReactNode } from "react";

import "./start-menu.css";

export type Order = { to: "folder" | "new" | "found"; title: string; line: string; off?: string };

/** The three starts, in a new user's words. Each line says exactly what happens. */
export const ORDERS: readonly Order[] = [
  { to: "folder", title: "Open a project", line: "Work in a folder already on this Mac." },
  { to: "new", title: "New project", line: "Start from an empty folder." },
  { to: "found", title: "Import from Claude Code", line: "Bring in the projects you have used Claude Code in." },
];

/** The keys that move through the menu, and the ones that pick. */
function Hints(): ReactNode {
  return (
    <div className="armada-start__hints" aria-hidden="true">
      <span><KbdChord keys={["j", "k"]} /> Move</span>
      <span><KbdChord keys={["1", "2", "3"]} /> Pick</span>
      <span><KbdChord keys={["↵"]} /> Select</span>
    </div>
  );
}

export function StartMenu({ at, last, orders, onPick, onFocus }: {
  /** The start the focus is on. */
  at: number;
  /** The start to focus when the menu opens: the one last picked. */
  last: number;
  orders: readonly Order[];
  onPick: (to: Order["to"], at: number) => void;
  onFocus: (at: number) => void;
}) {
  const open = orders[at]?.off === undefined ? orders[at] : undefined;
  return (
    <div className="armada-start">
      <h1 className="armada-start__word">Armada</h1>
      <div role="group" aria-label="Start from" className="armada-start__orders">
        {orders.map(({ to, title, off }, i) => {
          const here = i === at && off === undefined;
          const order = (
            <button
              key={to}
              type="button"
              className="armada-start__order"
              data-open={here ? "" : undefined}
              disabled={off !== undefined}
              autoFocus={i === last && off === undefined}
              aria-label={title}
              aria-describedby={here ? "armada-start-line" : undefined}
              onClick={() => onPick(to, i)}
              onFocus={() => onFocus(i)}
              onMouseEnter={(event) => off === undefined && event.currentTarget.focus()}
            >
              <span className="armada-start__key" aria-hidden="true">{i + 1}</span>
              <span className="armada-start__name">{title}</span>
            </button>
          );
          return off === undefined ? order : (
            <Tooltip key={to} label={off}>
              <span className="armada-start__off">{order}</span>
            </Tooltip>
          );
        })}
      </div>
      <p className="armada-start__brief" id="armada-start-line">{open?.line}</p>
      <Hints />
    </div>
  );
}
