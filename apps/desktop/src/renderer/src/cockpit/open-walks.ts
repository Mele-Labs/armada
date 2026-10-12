// The addresses of the walk windows open in Bridge now. A walk the owner is looking at is not a call
// yet, so the Cockpit holds back its call until the window closes. Main alone knows which are open.

import { useEffect, useState } from "react";

export function useOpenWalks(): ReadonlySet<string> {
  const [open, setOpen] = useState<ReadonlySet<string>>(new Set());
  useEffect(() => {
    const bridge = window.armada;
    if (typeof bridge?.getSessionWindows !== "function" || typeof bridge.onSessionWindows !== "function") return;
    let live = true;
    void bridge.getSessionWindows().then((addresses) => live && setOpen(new Set(addresses)));
    const off = bridge.onSessionWindows((addresses) => setOpen(new Set(addresses)));
    return () => {
      live = false;
      off();
    };
  }, []);
  return open;
}
