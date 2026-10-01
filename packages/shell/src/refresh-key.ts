// `⇧⌘R`, Refresh, on a key and nowhere on screen — the owner's note of
// 1 Oct 2026. The binding is the registry's `refresh` row, read, never spelled.

import { useEffect, useRef } from "react";
import { keyFor } from "@armada/components";

const REFRESH_KEY = keyFor("refresh");

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
      if (!(event.metaKey || event.ctrlKey) || !event.shiftKey || event.altKey || event.repeat) return;
      // With ⌘ down macOS reports the letter lowercase, shift or not.
      if (`⇧⌘${event.key.toUpperCase()}` !== REFRESH_KEY) return;
      event.preventDefault();
      latest.current();
    }
    window.addEventListener("keydown", pressed);
    return () => window.removeEventListener("keydown", pressed);
  }, []);
}
