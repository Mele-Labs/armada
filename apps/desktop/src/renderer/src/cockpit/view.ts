// Grid or map (the timeline), remembered across a restart as the panel's other ways of reading are: in
// the window's own storage, never a Fleet preference. The timeline until a pick is kept, and a failed read is the timeline.

import { useState } from "react";

export type CockpitView = "grid" | "map";

const KEY = "armada.bridge.cockpit-view";

export function useCockpitView(): [CockpitView, (view: CockpitView) => void] {
  const [view, set] = useState<CockpitView>(() => {
    try {
      return window.localStorage.getItem(KEY) === "grid" ? "grid" : "map";
    } catch {
      return "map";
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
