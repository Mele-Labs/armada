// `⇧⌘R`, Refresh, on a key and nowhere on screen — the owner's note of
// 1 Oct 2026. The binding is the registry's `refresh` row, read, never spelled.

import { useEffect, useRef } from "react";
import { isPressed } from "@armada/components";


/**
 * Re-read on `⇧⌘R`, from every surface and from inside a field.
 *
 * **The press is always taken**, connected or not. Electron's default menu
 * binds the same key to Force Reload, and a press this lets through reloads
 * the whole window instead.
 */
export function useRefreshKey(onRefresh: () => void): void {
  const latest = useRef(onRefresh);
  latest.current = onRefresh;

  useEffect(() => {
    function pressed(event: KeyboardEvent): void {
      // With ⌘ down macOS reports the letter lowercase, shift or not; the keymap reads shift from the flag.
      if (event.repeat || !isPressed("refresh", event)) return;
      event.preventDefault();
      latest.current();
    }
    window.addEventListener("keydown", pressed);
    return () => window.removeEventListener("keydown", pressed);
  }, []);
}
