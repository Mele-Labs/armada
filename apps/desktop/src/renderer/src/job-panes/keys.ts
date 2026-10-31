// The keys that arrange a Job's area. They are heard while the area is on screen, in a text box too, since each is a chord; a sheet or dialog open over it keeps its own keys.

import { useEffect } from "react";
import { isPressed, pressedSlot } from "@armada/components";

import { look, peek, rearrange } from "./store";
import { cycleTab, focusPane, neighbourOf, splitPane, type Toward } from "./tree";

const TOWARD: readonly Toward[] = ["left", "up", "right", "down"];

/** Puts the keyboard in a pane: on its tab in front, or on its + where it has none. */
export function focusIn(paneId: string): void {
  window.requestAnimationFrame(() => {
    const pane = document.querySelector<HTMLElement>(`[data-pane="${paneId}"]`);
    pane?.querySelector<HTMLElement>('[role="tab"][aria-selected="true"], button[aria-label="Add tab"]')?.focus();
  });
}

export function splitFocused(jobId: string, into: "right" | "down"): void {
  rearrange(jobId, (arrangement) => splitPane(arrangement, arrangement.focus, into));
  focusIn(peek(jobId).focus);
}

export function usePaneKeys(jobId: string, openLayouts: () => void): void {
  useEffect(() => {
    const listen = (event: KeyboardEvent) => {
      if (event.defaultPrevented || document.querySelector('[role="dialog"], [role="alertdialog"]') !== null) return;
      const take = (run: () => void) => {
        event.preventDefault();
        run();
      };
      if (isPressed("pane_split_right", event)) return take(() => splitFocused(jobId, "right"));
      if (isPressed("pane_split_down", event)) return take(() => splitFocused(jobId, "down"));
      if (isPressed("layout_menu", event)) return take(openLayouts);
      const by = isPressed("next_tab", event) ? 1 : isPressed("previous_tab", event) ? -1 : 0;
      if (by !== 0) {
        return take(() => {
          look(jobId, (arrangement) => cycleTab(arrangement, arrangement.focus, by));
          focusIn(peek(jobId).focus);
        });
      }
      const slot = pressedSlot("focus_pane", event);
      if (slot === -1) return;
      take(() => {
        const now = peek(jobId);
        const to = neighbourOf(now, now.focus, TOWARD[slot]!);
        if (to === undefined) return;
        look(jobId, (arrangement) => focusPane(arrangement, to));
        focusIn(to);
      });
    };
    window.addEventListener("keydown", listen);
    return () => window.removeEventListener("keydown", listen);
  }, [jobId, openLayouts]);
}
