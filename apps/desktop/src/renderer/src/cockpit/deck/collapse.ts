// A panel that folds to its header band when the band is pressed, and opens on a second press. Which panels are folded is kept per
// panel, by name, in the window's own storage, so a panel comes back as it was left after the view is left and entered again.
// A viewer's convenience, so a blocked store only forgets it.

import { useCallback, useState, type HTMLAttributes, type KeyboardEvent, type MouseEvent } from "react";

const keyOf = (id: string) => `armada.panel-collapsed.${id}`;

function read(id: string): boolean {
  try {
    return localStorage.getItem(keyOf(id)) === "1";
  } catch {
    return false;
  }
}

/** What a press on one of these inside the band is for, and not for folding the panel. */
const OWN_PRESS = "button, a, input, select, textarea, label";

/** Whether the panel is folded, and what its band needs to fold it: pressed or with Enter or Space, and read as a button by a screen reader. */
export function useCollapse(id: string): { collapsed: boolean; band: HTMLAttributes<HTMLElement> } {
  const [collapsed, set] = useState(() => read(id));
  const toggle = useCallback(() => {
    set((was) => {
      try {
        localStorage.setItem(keyOf(id), was ? "0" : "1");
      } catch {
        // Not kept: the panel is as it was left until the window closes.
      }
      return !was;
    });
  }, [id]);
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
