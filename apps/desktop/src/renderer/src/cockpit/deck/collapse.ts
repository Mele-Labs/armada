// A panel that folds to its header band when the band is pressed, and opens on a second press. Which panels are folded is kept per
// panel, by name, in the window's own storage, so a panel comes back as it was left after the view is left and entered again.
// A viewer's convenience, so a blocked store only forgets it.

import { useCallback, useState, type HTMLAttributes, type KeyboardEvent, type MouseEvent } from "react";

const keyOf = (id: string) => `armada.panel-collapsed.${id}`;

/** What was left, or `undefined` where nothing was: the panel is then as its place says. */
function read(id: string): boolean | undefined {
  try {
    const kept = localStorage.getItem(keyOf(id));
    return kept === null ? undefined : kept === "1";
  } catch {
    return undefined;
  }
}

/** What a press on one of these inside the band is for, and not for folding the panel. */
const OWN_PRESS = "button, a, input, select, textarea, label";

/**
 * Whether the panel is folded, and what its band needs to fold it: pressed or with Enter or Space, and read as a button by a screen reader.
 * `foldedAtFirst` is how it stands until it has been folded or opened once, after which what was left is kept.
 */
export function useCollapse(id: string, foldedAtFirst = false): { collapsed: boolean; band: HTMLAttributes<HTMLElement> } {
  const [left, set] = useState(() => read(id));
  const collapsed = left ?? foldedAtFirst;
  const toggle = useCallback(() => {
    set(!collapsed);
    try {
      localStorage.setItem(keyOf(id), collapsed ? "0" : "1");
    } catch {
      // Not kept: the panel is as it was left until the window closes.
    }
  }, [id, collapsed]);
  return {
    collapsed,
    band: {
      role: "button",
      tabIndex: 0,
      "aria-expanded": !collapsed,
      onClick: (event: MouseEvent<HTMLElement>) => {
        const inside = (event.target as Element).closest(OWN_PRESS);
        if (inside === null || inside === event.currentTarget) toggle();
      },
      onKeyDown: (event: KeyboardEvent<HTMLElement>) => {
        if (event.target !== event.currentTarget || (event.key !== "Enter" && event.key !== " ")) return;
        event.preventDefault();
        toggle();
      },
    },
  };
}
