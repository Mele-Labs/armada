import { useState } from "react";

/** Whether the ledger is hidden beside the thread, kept for the window. A viewer's convenience, so a blocked store only forgets it. */
export function useMinimized(): [boolean, (minimized: boolean) => void] {
  const [minimized, set] = useState(() => {
    try {
      return localStorage.getItem("armada.session-ledger.minimized") === "1";
    } catch {
      return false;
    }
  });
  return [
    minimized,
    (next) => {
      set(next);
      try {
        localStorage.setItem("armada.session-ledger.minimized", next ? "1" : "0");
      } catch {
        // Not kept: the ledger is as it was left until the window closes.
      }
    },
  ];
}
