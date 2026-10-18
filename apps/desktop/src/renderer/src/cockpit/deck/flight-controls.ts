// What the owner has done to the flight controls: the order of the tiles, which are hidden, a built-in's name, glyph, key and words, and
// the actions he added. Two sets, the Job's tiles (`job`, drawn by `SessionTiles`) and the tiles drawn with no Job open (`deck`).
// The owner's ask of 18 Oct 2026.
//
// **The file's shape is Fleet's.** `FlightFile` is what `~/.armada/flight-controls.toml` will hold, one `[[job]]` and one `[[deck]]` table
// per tile in order. The mock keeps it as JSON in `localStorage`, where Fleet's route will answer it; swapping the two touches `useFlightFile`
// and `saveFlight` and nothing that reads a `Tile`. A built-in the file does not list is appended in Armada's order.

import { useSyncExternalStore } from "react";
import { ACTIONS, slotsOf } from "@armada/components";

import { ASK_FOR_UPDATE } from "./acts";
import { LAND_OFF_TEXT, LAND_ON_TEXT, PAUSE_KEY, SESSION_TILES, tileKey, type SessionTileId } from "./session-tiles";

export type FlightSet = "job" | "deck";
export type DeckTileId = "checks" | "needs" | "map" | "timeline";
/** Words sent to the Job, a slash command typed to it, a shell line run in its bay, or a page opened. */
export type ActionKind = "words" | "command" | "shell" | "page";

/** One `[[job]]` or `[[deck]]` table. Everything but `id` is optional: absent is Armada's own. */
export type FlightEntry = {
  /** A built-in's id, or an id of the owner's own. */
  id: string;
  name?: string;
  /** A lucide name. */
  glyph?: string;
  /** A digit or a letter. */
  key?: string;
  hidden?: boolean;
  /** The words or command a built-in sends (`ask`, `compact`), or a custom action's words, command, shell line or address. */
  text?: string;
  /** `land` only. */
  on_text?: string;
  off_text?: string;
  /** Custom actions only. */
  kind?: ActionKind;
};

export type FlightFile = { version: 1; job: FlightEntry[]; deck: FlightEntry[] };

export const NOTHING_CHANGED: FlightFile = { version: 1, job: [], deck: [] };

/** What Armada ships for one tile. `says` is the words an `ask` or `compact` sends. */
export type BuiltIn = {
  id: string;
  /** The accessible name. */
  label: string;
  /** The word under the glyph. */
  text: string;
  glyph: string;
  says?: string;
  /** `land` sends one of two. */
  on?: string;
  off?: string;
  /** Its key never moves: Pause answers P. */
  fixedKey?: true;
};

const JOB_GLYPH: Record<SessionTileId, string> = {
  ask: "message-circle-more",
  land: "plane-landing",
  compact: "fold-vertical",
  report: "bug",
  map: "orbit",
  judge: "scale",
  release: "unplug",
  artifacts: "files",
  subagents: "split",
  checks: "shield-check",
  slot: "rows-3",
  pause: "circle-pause",
};

export const COMPACT_COMMAND = "/compact";

export const JOB_BUILT_INS: readonly BuiltIn[] = SESSION_TILES.map((tile) => ({
  id: tile.id,
  label: tile.label,
  text: tile.text,
  glyph: JOB_GLYPH[tile.id],
  ...(tile.id === "ask" ? { says: ASK_FOR_UPDATE } : {}),
  ...(tile.id === "compact" ? { says: COMPACT_COMMAND } : {}),
  ...(tile.id === "land" ? { on: LAND_ON_TEXT, off: LAND_OFF_TEXT } : {}),
  ...(tile.id === "pause" ? { fixedKey: true as const } : {}),
}));

export const DECK_BUILT_INS: readonly BuiltIn[] = [
  { id: "checks", label: "Checks", text: "Checks", glyph: "shield-check" },
  { id: "needs", label: "Needs me", text: "Calls", glyph: "megaphone" },
  { id: "map", label: "Map", text: "Map", glyph: "orbit" },
  { id: "timeline", label: "Timeline", text: "Timeline", glyph: "chart-no-axes-gantt" },
];

const BUILT_INS: Record<FlightSet, readonly BuiltIn[]> = { job: JOB_BUILT_INS, deck: DECK_BUILT_INS };

/** Without a Job there is no bay to run in and nobody to speak to, so the deck's tiles can only open a page. */
export const KINDS: Record<FlightSet, readonly ActionKind[]> = { job: ["words", "command", "shell", "page"], deck: ["page"] };

export const KIND_SAID: Record<ActionKind, string> = { words: "Words", command: "Slash command", shell: "Shell command", page: "Page" };

export const KIND_GLYPH: Record<ActionKind, string> = { words: "message-square", command: "square-terminal", shell: "terminal", page: "external-link" };

/** A tile as it is drawn: the built-in or custom action with the owner's changes laid over it. */
export type Tile = {
  id: string;
  builtIn: BuiltIn | undefined;
  /** The accessible name. */
  label: string;
  /** The word under the glyph. */
  text: string;
  glyph: string;
  /** The key that presses it, explicit or by position; none when hidden or when a taken digit leaves it without. */
  key: string | undefined;
  hidden: boolean;
  /** A built-in with anything of the owner's laid over it. */
  changed: boolean;
  /** Custom only. */
  kind: ActionKind | undefined;
  /** What a built-in sends (`ask`, `compact`) or what a custom action carries. */
  words: string | undefined;
  onWords: string | undefined;
  offWords: string | undefined;
  entry: FlightEntry | undefined;
};

const isKind = (value: unknown): value is ActionKind => value === "words" || value === "command" || value === "shell" || value === "page";
const worded = (value: string | undefined): string | undefined => (value === undefined || value.trim() === "" ? undefined : value);
const normalKey = (key: string | undefined): string | undefined => (key === undefined || !/^[0-9a-z]$/i.test(key) ? undefined : key.toUpperCase());

/** Whether the owner has laid anything over a built-in. */
const overlaid = (entry: FlightEntry): boolean => ["name", "glyph", "key", "hidden", "text", "on_text", "off_text"].some((one) => (entry as Record<string, unknown>)[one] !== undefined);

function tileOf(entry: FlightEntry, built: BuiltIn | undefined): Tile | undefined {
  if (built !== undefined) {
    return {
      id: entry.id,
      builtIn: built,
      label: worded(entry.name) ?? built.label,
      text: worded(entry.name) ?? built.text,
      glyph: entry.glyph ?? built.glyph,
      key: normalKey(entry.key),
      hidden: entry.hidden === true,
      changed: overlaid(entry),
      kind: undefined,
      words: worded(entry.text) ?? built.says,
      onWords: worded(entry.on_text) ?? built.on,
      offWords: worded(entry.off_text) ?? built.off,
      entry,
    };
  }
  if (!isKind(entry.kind)) return undefined;
  const name = worded(entry.name) ?? "Action";
  return {
    id: entry.id,
    builtIn: undefined,
    label: name,
    text: name,
    glyph: entry.glyph ?? KIND_GLYPH[entry.kind],
    key: normalKey(entry.key),
    hidden: entry.hidden === true,
    changed: false,
    kind: entry.kind,
    words: entry.text ?? "",
    onWords: undefined,
    offWords: undefined,
    entry,
  };
}

/**
 * Every tile of a set in the order it is drawn, hidden ones too: the file's entries in their order, then each built-in the file does not
 * name in Armada's own. **Keys follow position by default**, `1` to `9` and `0` over the tiles that are shown, Pause apart; an explicit key
 * wins, and the tile whose position it took has none.
 */
export function tilesOf(set: FlightSet, file: FlightFile): Tile[] {
  const built = BUILT_INS[set];
  const seen = new Set<string>();
  const tiles: Tile[] = [];
  const take = (entry: FlightEntry) => {
    if (seen.has(entry.id)) return;
    const tile = tileOf(entry, built.find((one) => one.id === entry.id));
    if (tile === undefined || (tile.kind !== undefined && !KINDS[set].includes(tile.kind))) return;
    seen.add(entry.id);
    tiles.push(tile);
  };
  file[set].forEach(take);
  built.forEach((one) => take({ id: one.id }));
  const explicit = new Set(tiles.flatMap((one) => (one.key === undefined || one.hidden ? [] : [one.key])));
  let at = 0;
  return tiles.map((tile) => {
    if (tile.hidden) return { ...tile, key: undefined };
    if (tile.builtIn?.fixedKey === true && tile.key === undefined) return { ...tile, key: PAUSE_KEY };
    if (tile.key !== undefined) {
      at += tile.builtIn?.fixedKey === true ? 0 : 1;
      return tile;
    }
    const given = tileKey(at);
    at += 1;
    return given === undefined || explicit.has(given) ? tile : { ...tile, key: given };
  });
}

// ---- Changing the order and the tiles --------------------------------------------------------------------------

/** The file's entries for the tiles as they stand, so a reorder keeps every built-in in its place. */
export const entriesOf = (tiles: readonly Tile[]): FlightEntry[] => tiles.map((one) => one.entry ?? { id: one.id });

export function movedBy(entries: readonly FlightEntry[], id: string, by: number): FlightEntry[] {
  const from = entries.findIndex((one) => one.id === id);
  const to = Math.min(entries.length - 1, Math.max(0, from + by));
  if (from < 0 || to === from) return [...entries];
  const next = [...entries];
  const [one] = next.splice(from, 1);
  next.splice(to, 0, one!);
  return next;
}

/** `id` put where `before` is, as a drop does. */
export function movedTo(entries: readonly FlightEntry[], id: string, before: string): FlightEntry[] {
  const from = entries.findIndex((one) => one.id === id);
  if (from < 0 || id === before) return [...entries];
  const next = [...entries];
  const [one] = next.splice(from, 1);
  const to = next.findIndex((it) => it.id === before);
  next.splice(to < 0 ? next.length : to, 0, one!);
  return next;
}

export const replacedIn = (entries: readonly FlightEntry[], entry: FlightEntry): FlightEntry[] => entries.map((one) => (one.id === entry.id ? entry : one));
export const removedFrom = (entries: readonly FlightEntry[], id: string): FlightEntry[] => entries.filter((one) => one.id !== id);

/** A built-in put back to Armada's own: every change cleared, its place kept. */
export const resetIn = (entries: readonly FlightEntry[], id: string): FlightEntry[] => entries.map((one) => (one.id === id ? { id } : one));

export function hiddenIn(entries: readonly FlightEntry[], id: string, hidden: boolean): FlightEntry[] {
  return entries.map((one) => {
    if (one.id !== id) return one;
    const { hidden: _, ...rest } = one;
    return hidden ? { ...rest, hidden: true } : rest;
  });
}

/** An id no tile of the set has. */
export function freshId(entries: readonly FlightEntry[], name: string): string {
  const base = `my-${name.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "") || "action"}`;
  let id = base;
  for (let n = 2; entries.some((one) => one.id === id); n += 1) id = `${base}-${n}`;
  return id;
}

// ---- Keys ---------------------------------------------------------------------------------------------------------

/** The letters the deck and the window already answer to, so no tile takes one. Read from the registry, with Pause's. */
export function reservedLetters(): ReadonlySet<string> {
  const taken = new Set<string>([PAUSE_KEY]);
  for (const act of ACTIONS) {
    if (act.scope !== "anywhere" && act.scope !== "deck") continue;
    for (const slot of slotsOf(act.id)) {
      if (slot.kind !== "chord" || slot.chord === null) continue;
      const { ctrl, alt, meta, shift, key } = slot.chord;
      if (!ctrl && !alt && !meta && !shift && /^[a-z]$/i.test(key)) taken.add(key.toUpperCase());
    }
  }
  return taken;
}

/** Why `key` cannot be this tile's, or nothing where it can. An empty key is always fine. */
export function keyProblem(key: string, tiles: readonly Tile[], own: string | undefined): string | undefined {
  if (key === "") return undefined;
  if (!/^[0-9a-z]$/i.test(key)) return "A digit or a letter";
  const upper = key.toUpperCase();
  if (/[A-Z]/.test(upper) && reservedLetters().has(upper)) return "Used by the deck";
  const other = tiles.find((one) => one.id !== own && one.entry?.key !== undefined && normalKey(one.entry.key) === upper);
  return other === undefined ? undefined : `Used by ${other.label}`;
}

// ---- A page's address ---------------------------------------------------------------------------------------------

/** What a Job gives an address: `{branch}`, `{pr}` and `{repository}`. */
export type JobFacts = { branch?: string | undefined; pr?: number | undefined; repository?: string | undefined };

/** The address with the Job's facts in, or why it cannot be had. */
export function filled(address: string, job: JobFacts): { url: string } | { missing: string } {
  const need = (name: string, value: string | number | undefined, said: string) => (address.includes(`{${name}}`) && value === undefined ? said : undefined);
  const missing = need("branch", job.branch, "The Job has no branch") ?? need("pr", job.pr, "The Job has no pull request") ?? need("repository", job.repository, "The repository is not known");
  if (missing !== undefined) return { missing };
  const url = address
    .replaceAll("{branch}", encodeURIComponent(job.branch ?? "").replaceAll("%2F", "/"))
    .replaceAll("{pr}", String(job.pr ?? ""))
    .replaceAll("{repository}", encodeURIComponent(job.repository ?? ""));
  return { url };
}

/** Whether the address is one Bridge opens: a web page. */
export const isWebAddress = (address: string): boolean => /^https?:\/\/[^\s{}]+/i.test(address.replace(/\{(branch|pr|repository)\}/g, "x"));

// ---- Where it is kept ----------------------------------------------------------------------------------------------

const STORE_KEY = "armada.bridge.flight-controls";

let raw: string | null | undefined;
let held: FlightFile = NOTHING_CHANGED;
const listeners = new Set<() => void>();

function entriesFrom(value: unknown): FlightEntry[] {
  if (!Array.isArray(value)) return [];
  return value.filter((one): one is FlightEntry => typeof one === "object" && one !== null && typeof (one as FlightEntry).id === "string");
}

function stored(): string | null {
  try {
    return window.localStorage.getItem(STORE_KEY);
  } catch {
    return null;
  }
}

function snapshot(): FlightFile {
  const now = stored();
  if (now !== raw) {
    raw = now;
    try {
      const parsed = JSON.parse(now ?? "null") as Partial<FlightFile> | null;
      held = parsed?.version === 1 ? { version: 1, job: entriesFrom(parsed.job), deck: entriesFrom(parsed.deck) } : NOTHING_CHANGED;
    } catch {
      held = NOTHING_CHANGED;
    }
  }
  return held;
}

/** The file as Fleet holds it. */
export function useFlightFile(): FlightFile {
  return useSyncExternalStore(
    (onChange) => (listeners.add(onChange), () => void listeners.delete(onChange)),
    snapshot,
  );
}

/** Saves one set's tiles in order. A failed write leaves the change for this window only. */
export function saveFlight(set: FlightSet, entries: readonly FlightEntry[]): void {
  const next: FlightFile = { ...snapshot(), [set]: [...entries] };
  try {
    window.localStorage.setItem(STORE_KEY, JSON.stringify(next));
  } catch {
    // Kept for this window only.
  }
  raw = stored();
  held = next;
  listeners.forEach((one) => one());
}
