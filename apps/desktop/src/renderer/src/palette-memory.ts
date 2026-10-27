// What the palette remembers between openings: the rows pinned, the rows opened or run last, and what the empty
// query shows. **Held in memory for now** — the wiring step gives them a place on disk. Row ids are the palette's own
// (`session:…`, `pull:…`), so a pin survives the list a row came from.

import { useSyncExternalStore } from "react";
import { PALETTE_OPENS, type PaletteChoice, type PaletteOpens } from "@armada/shell";

type Memory = { pinned: readonly string[]; recent: readonly string[]; opens: PaletteOpens };

const EMPTY: Memory = { pinned: [], recent: [], opens: "pinned" };
const RECENT_KEPT = 6;

let held: Memory = EMPTY;
const listeners = new Set<() => void>();
const set = (next: Memory): void => {
  held = next;
  listeners.forEach((one) => one());
};

export function usePaletteMemory(): Memory {
  return useSyncExternalStore(
    (on) => (listeners.add(on), () => void listeners.delete(on)),
    () => held,
  );
}

/** A window starts with what its scenario says was pinned and opened, or with nothing. */
export const seedPaletteMemory = (seed?: Partial<Memory>): void => set({ ...EMPTY, ...seed });

export const togglePin = (id: string): void => set({ ...held, pinned: held.pinned.includes(id) ? held.pinned.filter((one) => one !== id) : [...held.pinned, id] });

/** The row that was opened or run goes to the head of Recent. */
export const remember = (id: string): void => set({ ...held, recent: [id, ...held.recent.filter((one) => one !== id)].slice(0, RECENT_KEPT) });

/** The next of the three things the empty query can show. */
export const nextOpens = (): void => set({ ...held, opens: PALETTE_OPENS[(PALETTE_OPENS.findIndex((one) => one.id === held.opens) + 1) % PALETTE_OPENS.length]!.id });

export const opensLabel = (opens: PaletteOpens): string => PALETTE_OPENS.find((one) => one.id === opens)?.label ?? "";

const PREFIX: Partial<Record<PaletteChoice["of"], string>> = { session: "session", pull: "pull", issue: "issue", task: "task", command: "cmd", surface: "nav", setting: "set" };

/** The id of the row a choice was made on, for the kinds Recent keeps. */
export function rowOf(choice: PaletteChoice): string | undefined {
  if (choice.of === "on") return choice.action === "pin" || choice.action === "unpin" ? undefined : choice.target;
  const prefix = PREFIX[choice.of];
  return prefix === undefined || !("id" in choice) ? undefined : `${prefix}:${choice.id}`;
}
