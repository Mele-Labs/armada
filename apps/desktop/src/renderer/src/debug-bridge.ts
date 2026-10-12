// Debug Bridge, from the palette, to the annotation layer that carries it out. A window event,
// because the layer is a root of its own beside the app's and the app must not import it.

import { useEffect, useRef } from "react";
import { isPressed } from "@armada/components";

const DEBUG_BRIDGE = "armada:debug-bridge";

export function askToDebugBridge(): void {
  window.dispatchEvent(new Event(DEBUG_BRIDGE));
}

/** `debug` on the palette's row and on ⇧⌘D, wherever focus is. */
export function useDebugBridgeAsked(debug: () => void): void {
  const latest = useRef(debug);
  latest.current = debug;
  useEffect(() => {
    const asked = (): void => latest.current();
    function pressed(event: KeyboardEvent): void {
      if (event.repeat || !isPressed("debug_bridge", event)) return;
      event.preventDefault();
      latest.current();
    }
    window.addEventListener(DEBUG_BRIDGE, asked);
    window.addEventListener("keydown", pressed);
    return () => {
      window.removeEventListener(DEBUG_BRIDGE, asked);
      window.removeEventListener("keydown", pressed);
    };
  }, []);
}
