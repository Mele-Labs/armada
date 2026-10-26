// The layer open over a Session's stage: Artifacts, Subagents, Checks, Slot, the Field or the Job's document (which covers the whole Job view). One at a time, for one Session. Flight control's keys open
// them and the layer's Back closes it, so the thread under it is never unmounted and is where he left it. The element that had focus
// when a layer opened gets it back on Back.

import { useSyncExternalStore } from "react";

export type LayerId = "artifacts" | "subagents" | "checks" | "slot" | "field" | "document";

// `page` is which of the Job's documents the document layer reads: the `id` of its artifact. Absent, the layer takes the first.
type Open = { session: string; layer: LayerId; from: HTMLElement | null; page?: string };

let open: Open | undefined;
const hear = new Set<() => void>();
const set = (next: Open | undefined) => {
  open = next;
  for (const one of hear) one();
};

const focused = () => (document.activeElement instanceof HTMLElement && document.activeElement !== document.body ? document.activeElement : null);

/** Opens the layer over the Session; the same key again, or Back, closes it. A different key swaps it, keeping where focus first was. */
export function toggleLayer(session: string, layer: LayerId): void {
  if (open?.session === session && open.layer === layer) return closeLayer();
  set({ session, layer, from: open?.session === session ? open.from : focused() });
}

/** Opens the layer over the Session and leaves it open if it already is. The document layer is told which page it reads. */
export function showLayer(session: string, layer: LayerId, page?: string): void {
  if (open?.session === session && open.layer === layer && open.page === page) return;
  set({ session, layer, from: open?.session === session ? open.from : focused(), ...(page === undefined ? {} : { page }) });
}

/** Closes the layer and gives focus back to what had it before the first one opened. */
export function closeLayer(): void {
  const from = open?.from;
  set(undefined);
  if (from?.isConnected === true) window.setTimeout(() => from.focus(), 0);
}

/** Drops the layer when the Session it is over goes. */
export function dropLayer(session: string): void {
  if (open?.session === session) set(undefined);
}

/** The layer over this Session, if one is. */
export function useLayer(session: string): LayerId | undefined {
  const now = useSyncExternalStore(
    (listener) => {
      hear.add(listener);
      return () => void hear.delete(listener);
    },
    () => open,
  );
  return now?.session === session ? now.layer : undefined;
}

/** The page the document layer over this Session was opened on, if it was told one. */
export function useLayerPage(session: string): string | undefined {
  const now = useSyncExternalStore(
    (listener) => {
      hear.add(listener);
      return () => void hear.delete(listener);
    },
    () => open,
  );
  return now?.session === session ? now.page : undefined;
}
