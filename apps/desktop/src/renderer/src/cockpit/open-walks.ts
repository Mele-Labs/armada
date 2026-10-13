// The addresses of the walk windows open in Bridge now. A walk the owner is looking at is not a call
// yet, so the Cockpit holds back its call until the window closes. Main alone knows which are open.

import { useEffect, useState } from "react";

export function useOpenWalks(): ReadonlySet<string> {
  const [open, setOpen] = useState<ReadonlySet<string>>(new Set());
  useEffect(() => {
    const bridge = window.armada;
    if (typeof bridge?.getSessionWindows !== "function" || typeof bridge.onSessionWindows !== "function") return;
    let live = true;
    // A surface's fake Bridge answers every call, with whatever it has: only a list is the open windows.
    const take = (addresses: unknown) => Array.isArray(addresses) && setOpen(new Set(addresses.filter((one): one is string => typeof one === "string")));
    void bridge.getSessionWindows().then((addresses) => live && take(addresses));
    const off = bridge.onSessionWindows(take);
    return () => {
      live = false;
      off();
    };
  }, []);
  return open;
}
