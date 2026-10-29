// A panel that folds to its header band when the band is pressed, and opens on a second press. Which panels are folded is kept per
// panel, by name, in the window's own storage, so a panel comes back as it was left after the view is left and entered again.
// A viewer's convenience, so a blocked store only forgets it. Whether a Job's flight control is expanded is kept the same way.

import { useCallback, useState, type HTMLAttributes, type KeyboardEvent, type MouseEvent } from "react";

const keyOf = (id: string) => `armada.panel-collapsed.${id}`;
const FLIGHT_EXPANDED = "armada.flight-expanded";

function read(key: string): boolean {
  try {
    return localStorage.getItem(key) === "1";
  } catch {
    return false;
  }
}

function keep(key: string, on: boolean) {
  try {
    localStorage.setItem(key, on ? "1" : "0");
  } catch {
    // Not kept: the panel is as it was left until the window closes.
  }
}

/** What a press on one of these inside the band is for, and not for folding the panel. */
const OWN_PRESS = "button, a, input, select, textarea, label";

/** Whether the panel is folded, and what its band needs to fold it: pressed or with Enter or Space, and read as a button by a screen reader. */
export function useCollapse(id: string): { collapsed: boolean; band: HTMLAttributes<HTMLElement> } {
  const [collapsed, set] = useState(() => read(keyOf(id)));
  const toggle = useCallback(() => {
    set((was) => {
      keep(keyOf(id), !was);
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

/** Whether a Job's flight control lays its tiles six to a row rather than in one row that scrolls. One choice for every Job, kept as a fold is. */
export function useFlightExpanded(): { expanded: boolean; toggle: () => void } {
  const [expanded, set] = useState(() => read(FLIGHT_EXPANDED));
  const toggle = useCallback(() => {
    set((was) => {
      keep(FLIGHT_EXPANDED, !was);
      return !was;
    });
  }, []);
  return { expanded, toggle };
}
