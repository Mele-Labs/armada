import { useEffect } from "react";
import type { RefObject } from "react";

/** The band dims the thread's stage while it is open, and a press on that dimmed stage folds it. The stage is the Session's own, found from the band. */
export function useScrimFold(band: RefObject<HTMLElement | null>, open: boolean, fold: () => void): void {
  useEffect(() => {
    if (!open) return;
    const press = (event: MouseEvent) => {
      const stage = event.target;
      if (stage instanceof Element && stage.classList.contains("armada-session-thread__stage") && band.current?.closest(".armada-session-frame__centre")?.contains(stage) === true) fold();
    };
    document.addEventListener("click", press);
    return () => document.removeEventListener("click", press);
  }, [band, open, fold]);
}
