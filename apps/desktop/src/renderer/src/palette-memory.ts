// What the palette remembers between openings: the rows pinned, the rows opened or run last, what the empty query shows,
// and the alias and key he gave a row. Row ids are the palette's own (`session:…`, `pull:…`), so a pin survives the list
// a row came from.
//
// **`localStorage`, not a Fleet preference**, for `remembered-views.ts`' reason: Fleet's preference set is closed, and a
// pin or a key is how this person works on this Mac, not a fact Fleet holds. Every window of Bridge shares the one store
// and the browser tells the others when one writes (`storage`).

import { useSyncExternalStore } from "react";
import { PALETTE_OPENS, type PaletteChoice, type PaletteOpens } from "@armada/shell";

export type PaletteMemory = {
  pinned: readonly string[];
  recent: readonly string[];
  opens: PaletteOpens;
  /** A word that finds the row, by row id. */
  aliases: Readonly<Record<string, string>>;
  /** A key that opens the row, by row id, spelled as the palette draws keys: `⇧⌘S`. */
  hotkeys: Readonly<Record<string, string>>;
};

export const PALETTE_KEY = "armada.bridge.palette";

const EMPTY: PaletteMemory = { pinned: [], recent: [], opens: "pinned", aliases: {}, hotkeys: {} };
const RECENT_KEPT = 6;

const words = (value: unknown): value is string => typeof value === "string" && value !== "";
const listOf = (value: unknown): string[] => (Array.isArray(value) ? value.filter(words) : []);
const mapOf = (value: unknown): Record<string, string> =>
  value !== null && typeof value === "object" && !Array.isArray(value) ? Object.fromEntries(Object.entries(value).filter((one): one is [string, string] => words(one[1]))) : {};

/** What was stored, a part at a time: a part that is not whole is the default and leaves the rest alone. */
function readStored(): PaletteMemory {
  try {
    const raw: unknown = JSON.parse(window.localStorage.getItem(PALETTE_KEY) ?? "{}");
    if (raw === null || typeof raw !== "object") return EMPTY;
    const stored = raw as Record<string, unknown>;
    const opens = PALETTE_OPENS.find((one) => one.id === stored.opens)?.id ?? EMPTY.opens;
    return { pinned: listOf(stored.pinned), recent: listOf(stored.recent).slice(0, RECENT_KEPT), opens, aliases: mapOf(stored.aliases), hotkeys: mapOf(stored.hotkeys) };
  } catch {
    return EMPTY;
  }
}

let held: PaletteMemory | undefined;
const listeners = new Set<() => void>();

/** What is kept, read from storage the first time it is asked for. */
export function paletteMemory(): PaletteMemory {
  held ??= readStored();
  return held;
}

const notify = (): void => listeners.forEach((one) => one());

/** Keeps a change. A blocked storage leaves it to this window until it closes. */
function set(next: PaletteMemory): void {
  held = next;
  try {
    window.localStorage.setItem(PALETTE_KEY, JSON.stringify(next));
  } catch {
    // Blocked storage: unremembered between launches.
  }
  notify();
}

/** Lets go of what this window holds, so the next read is the storage's. Another window's write, and a test's relaunch. */
export function forgetPaletteMemory(): void {
  held = undefined;
  notify();
}

export function usePaletteMemory(): PaletteMemory {
  return useSyncExternalStore(
    (on) => {
      listeners.add(on);
      const heard = (event: StorageEvent) => event.key === PALETTE_KEY && forgetPaletteMemory();
      window.addEventListener("storage", heard);
      return () => {
        listeners.delete(on);
        window.removeEventListener("storage", heard);
      };
    },
    paletteMemory,
  );
}

/** A mock window starts with what its scenario says was pinned and opened, or with nothing. Never written: the storage is the Mac's. */
export function seedPaletteMemory(seed?: Partial<PaletteMemory>): void {
  held = { ...EMPTY, ...seed };
  notify();
}

export const togglePin = (id: string): void => {
  const was = paletteMemory();
  set({ ...was, pinned: was.pinned.includes(id) ? was.pinned.filter((one) => one !== id) : [...was.pinned, id] });
};

/** The row that was opened or run goes to the head of Recent. */
export const remember = (id: string): void => {
  const was = paletteMemory();
  set({ ...was, recent: [id, ...was.recent.filter((one) => one !== id)].slice(0, RECENT_KEPT) });
};

/** What the empty query shows, chosen in Settings. */
export const setOpens = (opens: PaletteOpens): void => set({ ...paletteMemory(), opens });

/** The next of the three things the empty query can show. */
export const nextOpens = (): void => {
  const was = paletteMemory();
  set({ ...was, opens: PALETTE_OPENS[(PALETTE_OPENS.findIndex((one) => one.id === was.opens) + 1) % PALETTE_OPENS.length]!.id });
};

const without = (from: Readonly<Record<string, string>>, id: string): Record<string, string> => Object.fromEntries(Object.entries(from).filter(([key]) => key !== id));

/** The word that finds a row. Nothing, or only spaces, lets it go. */
export const setAlias = (id: string, alias: string): void => {
  const was = paletteMemory();
  const word = alias.trim();
  set({ ...was, aliases: word === "" ? without(was.aliases, id) : { ...was.aliases, [id]: word } });
};

/** The key that opens a row. Nothing lets it go. */
export const setHotkey = (id: string, chord: string): void => {
  const was = paletteMemory();
  set({ ...was, hotkeys: chord === "" ? without(was.hotkeys, id) : { ...was.hotkeys, [id]: chord } });
};

const PREFIX: Partial<Record<PaletteChoice["of"], string>> = { session: "session", pull: "pull", issue: "issue", task: "task", command: "cmd", surface: "nav", setting: "set" };

/** The id of the row a choice was made on, for the kinds Recent keeps. */
export function rowOf(choice: PaletteChoice): string | undefined {
  if (choice.of === "on") return ["pin", "unpin", "alias", "remove-alias", "hotkey", "remove-hotkey"].includes(choice.action) ? undefined : choice.target;
  const prefix = PREFIX[choice.of];
  return prefix === undefined || !("id" in choice) ? undefined : `${prefix}:${choice.id}`;
}
