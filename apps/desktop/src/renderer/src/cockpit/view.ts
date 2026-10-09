// Grid or map, remembered across a restart as the panel's other ways of reading are: in the window's
// own storage, never a Fleet preference. A failed read is the grid.

import { useState } from "react";

export type CockpitView = "grid" | "map";

const KEY = "armada.bridge.cockpit-view";

export function useCockpitView(): [CockpitView, (view: CockpitView) => void] {
  const [view, set] = useState<CockpitView>(() => {
    try {
      return window.localStorage.getItem(KEY) === "map" ? "map" : "grid";
    } catch {
      return "grid";
    }
  });
  return [
    view,
    (next) => {
      set(next);
      try {
        window.localStorage.setItem(KEY, next);
      } catch {
        // Unremembered, which is the honest answer for a preference.
      }
    },
  ];
}
