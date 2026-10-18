import { useEffect } from "react";
import type { RefObject } from "react";

/**
 * While the band is open, Esc anywhere folds it, so an open panel that took no focus (a question opens it without) never swallows the key. Focus goes
 * back to the band only if it was inside the panel. A key something else already handled, or one pressed with a dialog up, is that other thing's.
 */
export function useEscapeFold(band: RefObject<HTMLElement | null>, open: boolean, fold: () => void): void {
  useEffect(() => {
    if (!open) return;
    const press = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || event.defaultPrevented) return;
      if (document.querySelector('[role="dialog"], [role="alertdialog"], dialog[open]') !== null) return;
      const inside = band.current?.contains(document.activeElement) === true;
      fold();
      if (inside) band.current?.querySelector<HTMLElement>(".armada-needs-panel__trigger")?.focus();
    };
    window.addEventListener("keydown", press);
    return () => window.removeEventListener("keydown", press);
  }, [band, open, fold]);
}
