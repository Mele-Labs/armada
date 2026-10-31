// Zoom: one pane of the open Job covers the Job's own area, to the right of the Jobs list; one Cockpit panel covers the Cockpit's body below the title bar. The merge strip shows either way. It is where the person is looking and not how the area is laid
// out, so it is kept apart from the arrangement: not edited, not saved, not an undo step. A change of Job or the Layout menu ends it.

import { useEffect, useState, useSyncExternalStore, type RefObject } from "react";

type Zoomed = { jobId: string; paneId: string } | undefined;

let pane: Zoomed;
let panel: HTMLElement | undefined;
const listeners = new Set<() => void>();
const tell = () => listeners.forEach((one) => one());

/** The Cockpit's body: the grid the Jobs list, the stage and the right column stand in, which ends above the merge strip. */
const bodyOf = (): HTMLElement | null => document.querySelector<HTMLElement>(".deck-grid");

export function usePaneZoomed(jobId: string): string | undefined {
  const now = useSyncExternalStore(
    (hear) => (listeners.add(hear), () => void listeners.delete(hear)),
    () => pane,
  );
  return now?.jobId === jobId ? now.paneId : undefined;
}

export function toggleZoom(jobId: string, paneId: string): void {
  unfit();
  pane = pane?.jobId === jobId && pane.paneId === paneId ? undefined : { jobId, paneId };
  tell();
}

export function endZoom(): void {
  if (pane === undefined && panel === undefined) return;
  unfit();
  pane = undefined;
  tell();
}

function unfit(): void {
  if (panel === undefined) return;
  const was = panel;
  panel = undefined;
  was.removeAttribute("data-zoomed");
  for (const name of ["position", "left", "top", "width", "height", "z-index"]) was.style.removeProperty(name);
}

/** The box the zoomed thing fills, in the window's pixels, kept as the Cockpit is resized: the element `within` holds, else the Cockpit's body. `pad` is what to grow it by on every side. */
export function useZoomBox(on: boolean, pad: number, within?: RefObject<HTMLElement | null>): { left: number; top: number; width: number; height: number } | undefined {
  const [box, setBox] = useState<{ left: number; top: number; width: number; height: number } | undefined>(undefined);
  useEffect(() => {
    const body = on ? (within?.current ?? bodyOf()) : null;
    if (body === null) return setBox(undefined);
    const measure = () => {
      const at = body.getBoundingClientRect();
      setBox({ left: at.left - pad, top: at.top - pad, width: at.width + pad * 2, height: at.height + pad * 2 });
    };
    measure();
    const watch = typeof ResizeObserver === "undefined" ? undefined : new ResizeObserver(measure);
    watch?.observe(body);
    window.addEventListener("resize", measure);
    return () => {
      watch?.disconnect();
      window.removeEventListener("resize", measure);
    };
  }, [on, pad]);
  return box;
}

/** A Cockpit panel zoomed or put back by double-clicking its header, set on the element itself since the panels are drawn by the deck. */
export function toggleZoomPanel(element: HTMLElement): void {
  const body = bodyOf();
  if (body === null) return;
  const was = panel === element;
  pane = undefined;
  unfit();
  if (was) return tell();
  panel = element;
  element.setAttribute("data-zoomed", "");
  const fit = () => {
    const at = body.getBoundingClientRect();
    element.style.position = "fixed";
    element.style.left = `${at.left}px`;
    element.style.top = `${at.top}px`;
    element.style.width = `${at.width}px`;
    element.style.height = `${at.height}px`;
    element.style.zIndex = "20";
  };
  fit();
  const watch = typeof ResizeObserver === "undefined" ? undefined : new ResizeObserver(() => (panel === element ? fit() : watch?.disconnect()));
  watch?.observe(body);
  tell();
}

/** Double-clicking a Cockpit panel's header zooms the panel, and again puts it back. A press on something in the header, or a header inside a pane, keeps its own meaning. */
export function usePanelZoom(): void {
  useEffect(() => {
    const on = (event: MouseEvent) => {
      const target = event.target;
      if (!(target instanceof Element) || target.closest("button, a, input, select, textarea, [role=menu]") !== null) return;
      const band = target.closest(".deck-band, .sv-panel__band");
      if (band === null || band.closest(".deck") === null || band.closest(".job-pane") !== null) return;
      const panel = band.closest<HTMLElement>(".deck-panel, .sv-panel");
      if (panel !== null) toggleZoomPanel(panel);
    };
    document.addEventListener("dblclick", on);
    return () => document.removeEventListener("dblclick", on);
  }, []);
}
