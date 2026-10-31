// Where a Job's acts stand when its panes do: as icons in the Thread tab's head, drawn by the Cockpit's `SessionTiles` into the slot the head holds. The acts live with the Job, not with the Thread
// tab, so a Thread tab that is closed or hidden leaves them pressable by their keys; and where the head is narrow they are the ellipsis menu's instead.

import { useLayoutEffect, useRef, useSyncExternalStore } from "react";

/** The head's width under which its acts fold into the ellipsis menu. */
const NARROW = 360;

export type JobAct = { id: string; label: string; /** Why it cannot be pressed, where it cannot. */ off: string | undefined; run: () => void };

type Place = { slot: HTMLElement | null; narrow: boolean };
let place: Place = { slot: null, narrow: false };
const placeListeners = new Set<() => void>();
const setPlace = (next: Place) => {
  if (next.slot === place.slot && next.narrow === place.narrow) return;
  place = next;
  placeListeners.forEach((one) => one());
};

let acts: readonly JobAct[] = [];
let actsKey = "";
const actsListeners = new Set<() => void>();

/** Every render offers the acts again with fresh closures; only a change of what they say is told. */
export function publishActs(next: readonly JobAct[]): void {
  const key = next.map((one) => `${one.id}|${one.label}|${one.off ?? ""}`).join("\n");
  const changed = key !== actsKey;
  acts = next;
  actsKey = key;
  if (changed) actsListeners.forEach((one) => one());
}

export const clearActs = () => publishActs([]);

export function useActsPlace(): Place {
  return useSyncExternalStore(
    (hear) => (placeListeners.add(hear), () => void placeListeners.delete(hear)),
    () => place,
  );
}

export function useActs(): readonly JobAct[] {
  return useSyncExternalStore(
    (hear) => (actsListeners.add(hear), () => void actsListeners.delete(hear)),
    () => acts,
  );
}

/** The place in a Thread head the acts are drawn into. */
export function JobActsSlot() {
  const element = useRef<HTMLSpanElement>(null);
  useLayoutEffect(() => {
    const slot = element.current;
    if (slot === null) return;
    const head = slot.closest(".armada-session-frame__head");
    const measure = () => setPlace({ slot, narrow: head !== null && head.clientWidth < NARROW });
    measure();
    const watch = typeof ResizeObserver === "undefined" || head === null ? undefined : new ResizeObserver(measure);
    if (head !== null) watch?.observe(head);
    return () => {
      watch?.disconnect();
      setPlace({ slot: null, narrow: false });
    };
  }, []);
  return <span ref={element} className="job-acts" role="toolbar" aria-label="Job acts" />;
}
