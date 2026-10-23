// What the owner has done to the flight controls: the order of the tiles, which are hidden, a built-in's name, glyph, key and words, and
// the actions he added. Two sets, the Job's tiles (`job`, drawn by `SessionTiles`) and the tiles drawn with no Job open (`deck`).
// The owner's ask of 18 Oct 2026.
//
// **The file's shape is Fleet's.** `FlightFile` is what `~/.armada/flight-controls.toml` holds, one `[[job]]` and one `[[deck]]` table
// per tile in order, read and saved through Kit (`get_flight_controls`, `save_flight_controls`); `useFlightFile` and `saveFlight` are the
// only parts that reach Fleet and nothing that reads a `Tile` does. A built-in the file does not list is appended in Armada's order.
//
// **A repository keeps actions of the owner's own beside it**, in the same shape, where its remembered runs are kept
// (`get_repository_flight_controls`): drawn while a Job in it is open or it is picked, and kept only for him. The owner's decision of 19 Oct 2026.
//
// **The repository's own committed tiles are drawn after his**, read from the open Job's tree (`in_repository`). Where one of his names
// the same id and says something else, his runs and is marked as hiding it; a committed shell tile asks before its first run (`asking`).
// `docs/design/who-sees-each-piece-of-configuration.md`.

import { useEffect, useSyncExternalStore } from "react";
import { conflictsOf, formatChord, parseChord, pressesChord, type Chord, type Press } from "@armada/components";
import type { FlightControl, FlightControlKind, FlightControls, RunPlace } from "@armada/protocol";
import type { TileAsk } from "@armada/protocol";
import { refusalWords } from "@armada/screens/src/refusal-words";

import type { FlightControlsAnswer, RepositoryFlightControlsAnswer } from "../../../../shared/api/flight-controls";
import type { BridgeState } from "../../../../shared/bridge";
import { withState } from "../../state-feed";

import { ASK_FOR_UPDATE } from "./acts";
import { LAND_OFF_TEXT, LAND_ON_TEXT, PAUSE_KEY, SESSION_TILES, tileKey, type SessionTileId } from "./session-tiles";

export type FlightSet = "job" | "deck";
export type DeckTileId = "checks" | "needs" | "map" | "timeline";
/** Words sent to the Job, a slash command typed to it, a shell line run in its bay, or a page opened. */
export type ActionKind = FlightControlKind;

/** One `[[job]]` or `[[deck]]` table, as the wire has it. Everything but `id` is optional: absent is Armada's own. */
export type FlightEntry = FlightControl;

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
  /** Pause answers P when it is given no other key. */
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
  requests: "arrow-right-left",
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
  /** Kept with the open repository rather than in the person's own file. */
  fromRepository: boolean;
  /** Committed to the repository by whoever wrote it, rather than kept by the person. */
  committed: boolean;
  /** The person's, hiding a committed tile of the same id that says something else: that tile's name. */
  hides: string | undefined;
  /** What a built-in sends (`ask`, `compact`) or what a custom action carries. */
  words: string | undefined;
  onWords: string | undefined;
  offWords: string | undefined;
  entry: FlightEntry | undefined;
};

const isKind = (value: unknown): value is ActionKind => value === "words" || value === "command" || value === "shell" || value === "page";
const worded = (value: string | undefined): string | undefined => (value === undefined || value.trim() === "" ? undefined : value);
const bareKey = (key: string): boolean => /^[0-9a-z]$/i.test(key);

/**
 * The chord a tile's key spells. A lone digit or letter is that key unshifted, so `R` in an older file is `r`; anything else is the registry's
 * spelling, `⌥1` or `⇧⌘J`.
 */
export function keyChord(key: string): Chord | null {
  if (bareKey(key)) return { ctrl: false, alt: false, shift: false, meta: false, key: key.toLowerCase() };
  return parseChord(key);
}

/** A key as it is kept and drawn, as the registry spells one: a lone digit or letter in lower case, a chord as it draws one. Nothing where it cannot be a tile's. */
const normalKey = (key: string | undefined): string | undefined => {
  if (key === undefined) return undefined;
  if (bareKey(key)) return key.toLowerCase();
  const chord = parseChord(key);
  return chord !== null && (chord.ctrl || chord.alt || chord.meta) ? formatChord(chord) : undefined;
};

/** Whether two spellings are one key to a tile: ⌘ and ⌃ are, as the registry reads them, unless both are held. */
const sameKey = (one: string, other: string): boolean => {
  const as = (key: string): string | undefined => {
    const chord = keyChord(key);
    return chord !== null && chord.ctrl && !chord.meta ? normalKey(formatChord({ ...chord, ctrl: false, meta: true })) : normalKey(key);
  };
  return as(one) !== undefined && as(one) === as(other);
};

/** Whether a press is the key a tile wears. */
export const keyPressed = (key: string, press: Press): boolean => {
  const chord = keyChord(key);
  return chord !== null && pressesChord(chord, press);
};

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
      fromRepository: false,
      committed: false,
      hides: undefined,
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
    fromRepository: false,
    committed: false,
    hides: undefined,
    words: entry.text ?? "",
    onWords: undefined,
    offWords: undefined,
    entry,
  };
}

/**
 * Every tile of a set in the order it is drawn, hidden ones too: the file's entries in their order, then each built-in the file does not
 * name in Armada's own, then the open repository's actions, before Pause where Pause ends the person's. **Keys follow position by
 * default**, `1` to `9` and `0` over the tiles that are shown, Pause apart; an explicit key wins, and the tile whose position it took has
 * none. A repository's key that one of the person's already has is dropped. The repository's committed tiles follow the person's for
 * it; one whose id the person's already have is not drawn, and theirs is marked as hiding it where the two differ.
 */
export function tilesOf(set: FlightSet, file: FlightFile, repository?: FlightFile, committed?: FlightFile): Tile[] {
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
  const yours = new Set(tiles.flatMap((one) => (one.key === undefined || one.hidden ? [] : [one.key])));
  const theirs = (repository?.[set] ?? []).flatMap((entry) => {
    if (seen.has(entry.id) || built.some((one) => one.id === entry.id)) return [];
    const tile = tileOf(entry, undefined);
    if (tile === undefined || !KINDS[set].includes(tile.kind!)) return [];
    seen.add(entry.id);
    return [{ ...tile, fromRepository: true, key: tile.key !== undefined && yours.has(tile.key) ? undefined : tile.key }];
  });
  const taken = new Set([...yours, ...theirs.flatMap((one) => (one.key === undefined ? [] : [one.key]))]);
  const committedTiles = (committed?.[set] ?? []).flatMap((entry) => {
    if (seen.has(entry.id) || built.some((one) => one.id === entry.id)) return [];
    const tile = tileOf(entry, undefined);
    if (tile === undefined || !KINDS[set].includes(tile.kind!)) return [];
    seen.add(entry.id);
    return [{ ...tile, committed: true, key: tile.key !== undefined && taken.has(tile.key) ? undefined : tile.key }];
  });
  const mark = (tile: Tile): Tile => {
    const under = committed?.[set].find((one) => one.id === tile.id);
    if (under === undefined || tile.builtIn !== undefined || tile.entry === undefined || sameEntry(under, tile.entry)) return tile;
    return { ...tile, hides: worded(under.name) ?? under.id };
  };
  tiles.splice(0, tiles.length, ...tiles.map(mark));
  tiles.splice(tiles.at(-1)?.builtIn?.fixedKey === true ? tiles.length - 1 : tiles.length, 0, ...theirs.map(mark), ...committedTiles);
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

/** Two tiles that say the same thing, whatever order their fields were written in. */
function sameEntry(one: FlightEntry, other: FlightEntry): boolean {
  const keys = new Set([...Object.keys(one), ...Object.keys(other)]);
  return [...keys].every((key) => (one as Record<string, unknown>)[key] === (other as Record<string, unknown>)[key]);
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

/** The editing keys of the text boxes the Cockpit holds, which are no act of the registry, so a tile may not take them. */
const EDITING: readonly { name: string; chord: Chord }[] = [
  { name: "Copy", chord: { ctrl: false, alt: false, shift: false, meta: true, key: "c" } },
  { name: "Paste", chord: { ctrl: false, alt: false, shift: false, meta: true, key: "v" } },
  { name: "Cut", chord: { ctrl: false, alt: false, shift: false, meta: true, key: "x" } },
  { name: "Select all", chord: { ctrl: false, alt: false, shift: false, meta: true, key: "a" } },
  { name: "Undo", chord: { ctrl: false, alt: false, shift: false, meta: true, key: "z" } },
  { name: "Redo", chord: { ctrl: false, alt: false, shift: true, meta: true, key: "z" } },
];

const sameChord = (a: Chord, b: Chord): boolean => a.key === b.key && a.ctrl === b.ctrl && a.alt === b.alt && a.shift === b.shift && a.meta === b.meta;

/**
 * Why `key` cannot be this tile's, or nothing where it can. An empty key is always fine. The key is a digit, a letter or a chord in the
 * registry's spelling; Shift alone makes no chord, for a capital `R` in a file has always meant `r`.
 */
export function keyProblem(key: string, tiles: readonly Tile[], own: string | undefined): string | undefined {
  if (key === "") return undefined;
  const chord = keyChord(key);
  if (chord === null) return "A digit, a letter or a chord";
  if (!chord.ctrl && !chord.alt && !chord.meta) {
    if (chord.shift) return "⇧ needs ⌘, ⌃ or ⌥ with it";
    if (!bareKey(chord.key)) return "A digit, a letter or a chord";
  }
  const held = normalKey(key);
  const edit = EDITING.find((one) => sameChord(one.chord, chord));
  if (edit !== undefined) return `Used by ${edit.name}`;
  const pause = tiles.find((one) => one.builtIn?.fixedKey === true);
  const pauses = pause !== undefined && pause.id === own;
  // P is the list row's Pause, and the open Job's while its Pause wears it; once Pause has another key P is free there.
  const pauseKey = normalKey(pause?.entry?.key);
  if (held === PAUSE_KEY && !pauses && (pauseKey === undefined || pauseKey === PAUSE_KEY)) return "Used by Pause";
  const act = pauses && held === PAUSE_KEY ? undefined : conflictsOf("deck_tile", { kind: "chord", chord })[0];
  if (act !== undefined) return `Used by ${act.verb}`;
  const other = tiles.find((one) => one.id !== own && one.entry?.key !== undefined && sameKey(one.entry.key, key));
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

// ---- What Fleet will not be asked ----------------------------------------------------------------------------------

/**
 * Why `entries` cannot be saved, naming the field as Fleet does (`job[2].kind`), or nothing where they can. **Fleet does not know the
 * built-in ids**, so these two are Bridge's to refuse: a `kind` on a built-in, and `on_text` or `off_text` on anything but Auto land.
 */
export function refusalOf(set: FlightSet, entries: readonly FlightEntry[]): string | undefined {
  const ids = BUILT_INS[set].map((one) => one.id);
  for (const [at, entry] of entries.entries()) {
    if (entry.kind !== undefined && ids.includes(entry.id)) return `${set}[${at}].kind`;
    if (entry.on_text !== undefined && !(set === "job" && entry.id === "land")) return `${set}[${at}].on_text`;
    if (entry.off_text !== undefined && !(set === "job" && entry.id === "land")) return `${set}[${at}].off_text`;
  }
  return undefined;
}

// ---- Where it is kept ----------------------------------------------------------------------------------------------

/** What Bridge reaches Fleet through: `window.armada` in the app, a fake in a test. */
export type FlightFleet = {
  state: () => Promise<Pick<BridgeState, "connection">>;
  subscribe: (on: (state: Pick<BridgeState, "connection">) => void) => () => void;
  getFlightControls: () => Promise<FlightControlsAnswer>;
  saveFlightControls: (controls: FlightControls) => Promise<FlightControlsAnswer>;
  getRepositoryFlightControls: (repository: string, sessionId?: string) => Promise<RepositoryFlightControlsAnswer>;
  saveRepositoryFlightControls: (repository: string, controls: FlightControls) => Promise<RepositoryFlightControlsAnswer>;
};

/**
 * One repository's actions as this window holds them: not answered yet, read from its place, or why they could not be. A read carries
 * the tiles the repository holds as committed, where they read, and the ask of each committed shell tile.
 */
export type RepositoryFlight =
  | { what: "unread" }
  | { what: "read"; file: FlightFile; place: RunPlace; committed?: FlightFile; asking?: readonly TileAsk[] }
  | { what: "not_read"; why: string };

const UNREAD: RepositoryFlight = { what: "unread" };
const NOT_REACHED = { ok: false, why: "not_connected" } as const;

/** What a save came to: kept for the window, and sent to Fleet unless Bridge refused it first. */
export type FlightSaved = { sent: true } | { refused: string };

/** What a save to a repository came to: kept, or why not in words. Nothing is drawn until Fleet has read it again. */
export type RepositorySaved = { saved: true } | { said: string };

export type FlightStore = {
  get: () => FlightFile;
  subscribe: (on: () => void) => () => void;
  save: (set: FlightSet, entries: readonly FlightEntry[]) => Promise<FlightSaved>;
  /** The repository whose actions are drawn, or none, and the Job open in it. A change reads its file again; the same again reads nothing. */
  open: (root: string | undefined, sessionId?: string) => void;
  /** Takes what Fleet answered for a repository after an approval, as a read of it. */
  took: (root: string, answer: RepositoryFlightControlsAnswer) => void;
  repository: (root: string) => RepositoryFlight;
  /** Saves one set of the repository's actions, beside its other set as last read. */
  saveRepository: (root: string, set: FlightSet, entries: readonly FlightEntry[]) => Promise<RepositorySaved>;
};

function entriesFrom(value: unknown): FlightEntry[] {
  if (!Array.isArray(value)) return [];
  return value.filter((one): one is FlightEntry => typeof one === "object" && one !== null && typeof (one as FlightEntry).id === "string");
}

/**
 * The file held for this window, read from Fleet once it is connected. **An unread file is the built-ins** and says nothing: a file that
 * will not read, or a Fleet not reached, leaves the tiles as Armada ships them and editing still works. A save is drawn at once and sent
 * after; a save Fleet refuses or cannot write stays for this window, and the one Fleet answers is the file from then on.
 */
export function createFlightStore(fleet: FlightFleet | undefined): FlightStore {
  let held: FlightFile = NOTHING_CHANGED;
  let latest = 0;
  let wasConnected = false;
  let opened: string | undefined;
  let openedJob: string | undefined;
  const repositories = new Map<string, RepositoryFlight>();
  /** The last read asked of each root, so an answer overtaken by a later one is dropped. */
  const asked = new Map<string, number>();
  let stop: (() => void) | null = null;
  const listeners = new Set<() => void>();
  const set = (to: FlightFile): void => {
    held = to;
    listeners.forEach((on) => on());
  };
  const fileOf = (controls: FlightControls): FlightFile => ({ version: 1, job: entriesFrom(controls.job), deck: entriesFrom(controls.deck) });
  const read = (): void =>
    void fleet?.getFlightControls().then((answer) => {
      if (!answer.ok) return;
      set(answer.value.what === "read" ? fileOf(answer.value.controls) : NOTHING_CHANGED);
    });
  const hold = (root: string, to: RepositoryFlight): void => {
    repositories.set(root, to);
    listeners.forEach((on) => on());
  };
  const flightOf = (answer: RepositoryFlightControlsAnswer): RepositoryFlight => {
    if (!answer.ok) return { what: "not_read", why: refusalWords(answer.outcome) };
    const { read: was, place, in_repository: theirs, asking } = answer.value;
    if (was.what !== "read") return { what: "not_read", why: was.why };
    // A repository that commits no tiles reads as one with nothing to draw from it.
    const committed = theirs?.what === "read" ? fileOf(theirs.controls) : undefined;
    const holds = committed !== undefined && committed.job.length + committed.deck.length > 0;
    return { what: "read", file: fileOf(was.controls), place, ...(holds ? { committed } : {}), ...(asking === undefined ? {} : { asking }) };
  };
  // Never polled: a repository kept in a bucket costs a network read each time.
  const readRepository = (root: string): void => {
    if (fleet === undefined) return;
    const mine = (asked.get(root) ?? 0) + 1;
    asked.set(root, mine);
    void (openedJob === undefined ? fleet.getRepositoryFlightControls(root) : fleet.getRepositoryFlightControls(root, openedJob)).then((answer) => {
      if (asked.get(root) === mine) hold(root, flightOf(answer));
    });
  };
  const connectedIn = (facts: Pick<BridgeState, "connection">): void => {
    const connected = facts.connection.state === "connected";
    if (connected && !wasConnected) {
      read();
      if (opened !== undefined) readRepository(opened);
    }
    wasConnected = connected;
  };
  const start = (): void => {
    if (fleet === undefined) return;
    const off = fleet.subscribe(connectedIn);
    void fleet.state().then(connectedIn);
    stop = () => {
      off();
      wasConnected = false;
    };
  };
  return {
    get: () => held,
    subscribe: (on) => {
      listeners.add(on);
      if (stop === null) start();
      return () => {
        listeners.delete(on);
        if (listeners.size === 0 && stop !== null) {
          stop();
          stop = null;
        }
      };
    },
    save: async (which, entries) => {
      const refused = refusalOf(which, entries);
      if (refused !== undefined) return { refused };
      const next: FlightFile = { ...held, [which]: [...entries] };
      const mine = (latest += 1);
      set(next);
      const answer = await fleet?.saveFlightControls({ version: 1, job: next.job, deck: next.deck });
      if (answer?.ok === true && answer.value.what === "read" && mine === latest) set(fileOf(answer.value.controls));
      return { sent: true };
    },
    open: (root, sessionId) => {
      if (root === opened && sessionId === openedJob) return;
      opened = root;
      openedJob = sessionId;
      if (root !== undefined && wasConnected) readRepository(root);
    },
    took: (root, answer) => {
      if (!answer.ok) return;
      asked.set(root, (asked.get(root) ?? 0) + 1);
      hold(root, flightOf(answer));
    },
    repository: (root) => repositories.get(root) ?? UNREAD,
    saveRepository: async (root, which, entries) => {
      const was = repositories.get(root) ?? UNREAD;
      // Saving over a file that was never read would lose what it holds.
      if (was.what !== "read") return { said: was.what === "not_read" ? was.why : refusalWords(NOT_REACHED) };
      const refused = refusalOf(which, entries) ?? entries.flatMap((one, at) => (one.kind === undefined ? [`${which}[${at}].kind`] : []))[0];
      if (refused !== undefined) return { said: `${refused} is refused` };
      if (fleet === undefined) return { said: refusalWords(NOT_REACHED) };
      const next: FlightControls = { version: 1, job: was.file.job, deck: was.file.deck, [which]: [...entries] };
      const answer = await fleet.saveRepositoryFlightControls(root, next);
      if (!answer.ok) return { said: refusalWords(answer.outcome) };
      // A read still out was asked before this save, so what it answers is older.
      asked.set(root, (asked.get(root) ?? 0) + 1);
      hold(root, flightOf(answer));
      return answer.value.read.what === "read" ? { saved: true } : { said: answer.value.read.why };
    },
  };
}

/** The window's one store, made again where `window.armada` is another, as the mock's scenario switch makes it. */
let own: { over: unknown; store: FlightStore } | undefined;
const store = (): FlightStore => {
  if (own === undefined || own.over !== window.armada) own = { over: window.armada, store: createFlightStore(window.armada === undefined ? undefined : withState(window.armada)) };
  return own.store;
};

/** The file as Fleet holds it. */
export function useFlightFile(): FlightFile {
  const kept = store();
  return useSyncExternalStore(kept.subscribe, kept.get);
}

/** The actions kept for the repository open in the Cockpit, read again whenever it or the Job open in it changes; nothing with none open. */
export function useRepositoryFlight(root: string | undefined, sessionId?: string): RepositoryFlight | undefined {
  const kept = store();
  useEffect(() => kept.open(root, sessionId), [kept, root, sessionId]);
  return useSyncExternalStore(kept.subscribe, () => (root === undefined ? undefined : kept.repository(root)));
}

/** The repository's file where it was read, and nothing to draw while it is unread or would not read. */
export const repositoryFileOf = (flight: RepositoryFlight | undefined): FlightFile | undefined => (flight?.what === "read" ? flight.file : undefined);

/** The tiles the repository holds as committed, where they were read. */
export const committedFileOf = (flight: RepositoryFlight | undefined): FlightFile | undefined => (flight?.what === "read" ? flight.committed : undefined);

/** Approves the command shown for a committed tile, and takes the read Fleet answers with. A refusal is answered for the caller to word. */
export async function approveTile(root: string, sessionId: string | undefined, ask: TileAsk): Promise<RepositoryFlightControlsAnswer> {
  const answer = await window.armada.approveRepositoryTile({ repository: root, tile_id: ask.id, command_hash: ask.command_hash, ...(sessionId === undefined ? {} : { session_id: sessionId }) });
  store().took(root, answer);
  return answer;
}

/** Saves one set's tiles in order. A save that fails leaves the change for this window only. */
export const saveFlight = (set: FlightSet, entries: readonly FlightEntry[]): Promise<FlightSaved> => store().save(set, entries);

/** Saves one set of a repository's actions in order. A save that fails changes nothing and says why. */
export const saveRepositoryFlight = (root: string, set: FlightSet, entries: readonly FlightEntry[]): Promise<RepositorySaved> => store().saveRepository(root, set, entries);
