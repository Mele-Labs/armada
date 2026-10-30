// The keys each act answers to, as this person has them: the registry's, with whatever they rebound
// in Settings → Keyboard laid over it. `docs/concepts/key-bindings.md`.
//
// **Every handler asks here, and none spells a key.** `actions.toml` stays the authority on what an
// act *ships* bound to; this is the one place that knows what it is bound to *now*. A handler that
// compares `event.key` to a letter is a binding Settings cannot move, which is the defect this file
// exists to end.
//
// **A binding is the registry's own grammar.** `⇧⌘R`, `B`, `Esc`, `⌘[ ⌘]`, `⌘1–⌘9` — so what a person
// saves reads exactly as what the registry ships, every `Kbd` already draws it, and nothing on the
// wire needs a second spelling.

import { useSyncExternalStore } from "react";

import { ACTION, ACTIONS } from "./generated/actions";
import type { Action, ActionScope } from "./generated/actions";

/** One key with its modifiers. `key` is a letter in lower case, a digit, a symbol, or a named key. */
export type Chord = { readonly ctrl: boolean; readonly alt: boolean; readonly shift: boolean; readonly meta: boolean; readonly key: string };

/** One thing a person can bind: a key, or the modifiers that go with a row of digits. */
export type Slot =
  | { readonly kind: "chord"; readonly chord: Chord | null }
  | { readonly kind: "digits"; readonly mods: Omit<Chord, "key"> | null; readonly from: number; readonly to: number };

/** A press, as a handler receives it. `code` reads a key by place when Option has changed what it types. */
export type Press = Pick<KeyboardEvent, "key" | "metaKey" | "ctrlKey" | "altKey"> & { readonly shiftKey?: boolean; readonly code?: string };

const NAMED: Readonly<Record<string, string>> = {
  Escape: "Esc",
  Esc: "Esc",
  Enter: "Enter",
  Return: "Enter",
  "↵": "Enter",
  Tab: "Tab",
  " ": "Space",
  Space: "Space",
  Backspace: "Backspace",
  Delete: "Delete",
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
  "↑": "↑",
  "↓": "↓",
  "←": "←",
  "→": "→",
  Home: "Home",
  End: "End",
  PageUp: "PageUp",
  PageDown: "PageDown",
};

/** Keys that are only ever half a chord. Pressing one alone binds nothing. */
const MODIFIER_KEYS = new Set(["Meta", "Control", "Alt", "Shift", "CapsLock", "Fn", "OS", "Hyper", "Super"]);

const isLetter = (key: string): boolean => /^[a-z]$/i.test(key);
const isNamed = (key: string): boolean => Object.values(NAMED).includes(key) || /^F\d{1,2}$/.test(key);

/**
 * Shift is part of a letter's chord and of a named key's, and never of a symbol's: `?` is shift and
 * `/` on one keyboard and a key of its own on another, so the character typed is the whole answer.
 */
const shiftCounts = (key: string): boolean => isLetter(key) || isNamed(key) || /^\d$/.test(key);

/** One chord from its spelling — `⇧⌘R`, `B`, `Esc`, `⌥↑`. `null` for one this cannot read. */
export function parseChord(text: string): Chord | null {
  let rest = text.trim();
  let ctrl = false;
  let alt = false;
  let shift = false;
  let meta = false;
  for (;;) {
    const head = rest[0];
    if (rest.length <= 1 || head === undefined) break;
    if (head === "⌃") ctrl = true;
    else if (head === "⌥") alt = true;
    else if (head === "⇧") shift = true;
    else if (head === "⌘") meta = true;
    else break;
    rest = rest.slice(1);
  }
  if (rest === "") return null;
  const named = NAMED[rest];
  if (named !== undefined) return { ctrl, alt, shift, meta, key: named };
  if (/^F\d{1,2}$/.test(rest)) return { ctrl, alt, shift, meta, key: rest };
  if ([...rest].length !== 1) return null;
  if (isLetter(rest)) {
    // Bare, an upper-case letter is the shifted one (`B`); beside ⌘, ⌃ or ⌥ it is only how it is drawn.
    const bare = !ctrl && !alt && !meta;
    return { ctrl, alt, meta, shift: shift || (bare && rest !== rest.toLowerCase()), key: rest.toLowerCase() };
  }
  return { ctrl, alt, meta, shift: shiftCounts(rest) ? shift : false, key: rest };
}

/** The modifiers, in the order the registry draws them. */
const modsText = (mods: Omit<Chord, "key">): string =>
  `${mods.ctrl ? "⌃" : ""}${mods.alt ? "⌥" : ""}${mods.shift ? "⇧" : ""}${mods.meta ? "⌘" : ""}`;

/** A chord in the registry's spelling: `B` for a shifted letter alone, `⇧⌘R` beside a modifier. */
export function formatChord(chord: Chord): string {
  if (isLetter(chord.key)) {
    const bare = !chord.ctrl && !chord.alt && !chord.meta;
    if (bare) return chord.shift ? chord.key.toUpperCase() : chord.key;
    return `${modsText(chord)}${chord.key.toUpperCase()}`;
  }
  return `${modsText(chord)}${chord.key}`;
}

/**
 * The slots an act's spelling holds. `⌘[ ⌘]` is two, `j / k / ↓ / ↑` four, `⌘1–⌘9` one row of digits.
 * A blank spelling, which is how a person unbinds a key, is a slot with nothing in it.
 */
export function parseSlots(text: string): Slot[] {
  const range = /^(\D*?)(\d)–(?:\D*?)(\d)$/.exec(text.trim());
  if (range !== null) {
    const probe = parseChord(`${range[1] ?? ""}${range[2]}`);
    return [{ kind: "digits", mods: probe === null ? null : withoutKey(probe), from: Number(range[2]), to: Number(range[3]) }];
  }
  const parts = text.includes(" / ") ? text.split(" / ") : text.split(" ");
  return parts.map((part) => ({ kind: "chord", chord: part.trim() === "" ? null : parseChord(part) }));
}

const withoutKey = ({ ctrl, alt, shift, meta }: Chord): Omit<Chord, "key"> => ({ ctrl, alt, shift, meta });

/** One slot in the registry's spelling, or `""` where nothing is bound. */
export function formatSlot(slot: Slot): string {
  if (slot.kind === "chord") return slot.chord === null ? "" : formatChord(slot.chord);
  if (slot.mods === null) return "";
  const mods = modsText(slot.mods);
  return `${mods}${slot.from}–${mods}${slot.to}`;
}

/** Slots back into one spelling, the way the registry separates them. Unbound slots draw nothing. */
function formatSlots(id: string, slots: readonly Slot[]): string {
  const shown = slots.map(formatSlot).filter((one) => one !== "");
  return shown.join(SLASHED.has(id) ? " / " : " ");
}

/** Acts whose registry spelling separates its keys with ` / ` rather than a space. */
const SLASHED = new Set(ACTIONS.filter((one) => one.shortcut.includes(" / ")).map((one) => one.id));

/** What each slot of a many-key act does, where one row of keys is more than one motion. */
export const SLOT_NAMES: Readonly<Record<string, readonly string[] | undefined>> = {
  history: ["Back", "Forward"],
  move_focus: ["Down", "Up", "Down", "Up"],
  deck_reorder: ["Up", "Down"],
  projects_reorder: ["Up", "Down"],
  finding_move: ["Down", "Up", "Down", "Up"],
};

// ---- What this person has changed ------------------------------------------------------------

/** The acts a person has rebound, by id, each to the spelling of every slot it holds. */
export type KeyBindings = Readonly<Record<string, readonly string[]>>;

let overrides: KeyBindings = {};
let version = 0;
let saver: ((text: string) => void) | null = null;
const listeners = new Set<() => void>();

const announce = (): void => {
  version += 1;
  listeners.forEach((on) => on());
};

/** What a person has changed, as Fleet keeps it. */
export function keyBindings(): KeyBindings {
  return overrides;
}

/**
 * Lay a person's bindings over the registry — what Fleet answered, on a read or after a save. An id
 * the registry does not carry is kept, so a binding from a newer Armada survives an older one.
 */
export function setKeyBindings(next: KeyBindings): void {
  if (JSON.stringify(next) === JSON.stringify(overrides)) return;
  overrides = next;
  announce();
}

/** Where a change is sent to be kept. Bridge hands Fleet's save in; a test or the mock keeps none. */
export function setKeyBindingSaver(save: ((text: string) => void) | null): void {
  saver = save;
}

function keep(next: KeyBindings): void {
  overrides = next;
  announce();
  saver?.(serializeKeyBindings(next));
}

/** Bind one act's slots. A spelling identical to the registry's takes the act's own change back. */
export function rebind(id: string, slots: readonly Slot[]): void {
  const spelled = slots.map(formatSlot);
  const shipped = defaultSlotsOf(id).map(formatSlot);
  const { [id]: _was, ...rest } = overrides;
  keep(JSON.stringify(spelled) === JSON.stringify(shipped) ? rest : { ...rest, [id]: spelled });
}

/** Put one act's keys back to what the registry ships. */
export function resetBinding(id: string): void {
  if (overrides[id] === undefined) return;
  const { [id]: _was, ...rest } = overrides;
  keep(rest);
}

/** Put every key back to what the registry ships. */
export function resetAllBindings(): void {
  if (Object.keys(overrides).length === 0) return;
  keep({});
}

/** The text Fleet keeps. Empty where nothing has changed, which takes the preference back. */
export function serializeKeyBindings(bindings: KeyBindings): string {
  return Object.keys(bindings).length === 0 ? "" : JSON.stringify({ version: 1, bindings });
}

/**
 * Fleet's text back into bindings. **One wrong shape and none of it is applied**, `ipc::key_bindings`'
 * rule, so a half-read file never moves half the keys.
 */
export function parseKeyBindings(text: string): KeyBindings {
  if (text.trim() === "") return {};
  try {
    const raw: unknown = JSON.parse(text);
    if (typeof raw !== "object" || raw === null || (raw as { version?: unknown }).version !== 1) return {};
    const bindings = (raw as { bindings?: unknown }).bindings ?? {};
    if (typeof bindings !== "object" || bindings === null || Array.isArray(bindings)) return {};
    const out: Record<string, readonly string[]> = {};
    for (const [id, keys] of Object.entries(bindings)) {
      if (!Array.isArray(keys) || keys.length > 8 || !keys.every((one) => typeof one === "string")) return {};
      out[id] = keys as string[];
    }
    return out;
  } catch {
    return {};
  }
}

/** Re-renders a component when a binding moves, so every caption it draws is the key now in force. */
export function useKeyBindings(): KeyBindings {
  useSyncExternalStore(
    (on) => {
      listeners.add(on);
      return () => void listeners.delete(on);
    },
    () => version,
    () => version,
  );
  return overrides;
}

// ---- What an act answers now ----------------------------------------------------------------

/** The slots an act ships with. Empty for an id the registry does not carry. */
export function defaultSlotsOf(id: string): Slot[] {
  const act = ACTION[id];
  return act === undefined ? [] : parseSlots(act.shortcut);
}

/**
 * The slots an act answers now. A saved list shorter than the registry's leaves the rest as shipped,
 * and one whose slots do not have the registry's shape is ignored.
 */
export function slotsOf(id: string): Slot[] {
  const shipped = defaultSlotsOf(id);
  const saved = overrides[id];
  if (saved === undefined) return shipped;
  return shipped.map((slot, at) => {
    const text = saved[at];
    if (text === undefined) return slot;
    if (slot.kind === "digits") {
      if (text === "") return { ...slot, mods: null };
      const read = parseSlots(text)[0];
      return read?.kind === "digits" ? { ...slot, mods: read.mods } : slot;
    }
    return { kind: "chord", chord: text === "" ? null : parseChord(text) };
  });
}

/** The key an act answers now, spelled for drawing. `""` where a person has unbound every slot. */
export function boundKeyFor(id: string): string {
  const act = ACTION[id];
  if (act !== undefined && overrides[id] === undefined) return act.shortcut;
  return formatSlots(id, slotsOf(id));
}

/** Whether a person has moved this act off the registry's keys. */
export function isRebound(id: string): boolean {
  return overrides[id] !== undefined;
}

// ---- A press ----------------------------------------------------------------------------------

/**
 * The key a press is, read the way a chord is spelled. **Option changes the character a key types**,
 * so with it held a letter or digit is read by its place on the keyboard instead.
 */
function keyOf(press: Press): string {
  const code = press.code ?? "";
  if (press.altKey) {
    const letter = /^Key([A-Z])$/.exec(code)?.[1];
    if (letter !== undefined) return letter.toLowerCase();
    const digit = /^Digit(\d)$/.exec(code)?.[1];
    if (digit !== undefined) return digit;
  }
  const named = NAMED[press.key];
  if (named !== undefined) return named;
  if (/^F\d{1,2}$/.test(press.key)) return press.key;
  return isLetter(press.key) ? press.key.toLowerCase() : press.key;
}

/** A press as a chord, or `null` for a modifier pressed alone — which is half a chord, not one. */
export function chordOf(press: Press): Chord | null {
  if (MODIFIER_KEYS.has(press.key) || press.key === "Unidentified") return null;
  // Option makes some letters a dead key (⌥E, ⌥U, ⌥N on a US Mac); with it held, the key's place still names the chord.
  if (press.key === "Dead" && !(press.altKey && /^(Key[A-Z]|Digit\d)$/.test(press.code ?? ""))) return null;
  const key = keyOf(press);
  // An upper-case letter is a shifted one whatever the flag says — Caps Lock types it too, and
  // `B` has always answered to the letter typed rather than to the key held.
  const upper = isLetter(press.key) && press.key !== press.key.toLowerCase();
  const shift = shiftCounts(key) && (press.shiftKey === true || upper);
  return { ctrl: press.ctrlKey, alt: press.altKey, meta: press.metaKey, shift, key };
}

/**
 * Whether a press is a chord. **⌘ answers to Control as well**, unless the chord asks for ⌃ itself:
 * every modified binding in Bridge has always taken either, so one learned on a Mac still works on a
 * keyboard without the key.
 */
function same(chord: Chord, press: Chord): boolean {
  if (chord.key !== press.key || chord.alt !== press.alt || chord.shift !== press.shift) return false;
  if (chord.meta && !chord.ctrl) return (press.meta || press.ctrl) && !(press.meta && press.ctrl);
  return chord.meta === press.meta && chord.ctrl === press.ctrl;
}

/** Whether a press is this chord, read as an act's own keys are: for a key kept outside the registry, such as a flight control's. */
export function pressesChord(chord: Chord, press: Press): boolean {
  const pressed = chordOf(press);
  return pressed !== null && same(chord, pressed);
}

/** Which slot of an act a press is, or `-1` for none. */
export function pressedSlot(id: string, press: Press): number {
  const chord = chordOf(press);
  if (chord === null) return -1;
  return slotsOf(id).findIndex((slot) => slot.kind === "chord" && slot.chord !== null && same(slot.chord, chord));
}

/** Whether a press is one of the keys an act answers now. */
export function isPressed(id: string, press: Press): boolean {
  return pressedSlot(id, press) !== -1;
}

/** The digit a press picks from an act's row of digits — `⌘3` is 3 — or `null` for none. */
export function pressedDigit(id: string, press: Press): number | null {
  const chord = chordOf(press);
  if (chord === null || !/^\d$/.test(chord.key)) return null;
  const digit = Number(chord.key);
  for (const slot of slotsOf(id)) {
    if (slot.kind !== "digits" || slot.mods === null || digit < slot.from || digit > slot.to) continue;
    if (same({ ...slot.mods, key: chord.key }, chord)) return digit;
  }
  return null;
}

/** The spelling of one digit of an act's row — `⌘3` — for drawing beside the thing it reaches. */
export function digitKeyFor(id: string, digit: number): string | undefined {
  const slot = slotsOf(id).find((one) => one.kind === "digits");
  if (slot === undefined || slot.kind !== "digits" || slot.mods === null || digit < slot.from || digit > slot.to) return undefined;
  return `${modsText(slot.mods)}${digit}`;
}

// ---- Where two acts would fight over one key --------------------------------------------------

/**
 * Where each scope is listening. Two acts conflict only where both listen at once: `o` opens a row on
 * the list and the output on a Job, and those are never the same screen.
 */
const PLACES: Readonly<Record<ActionScope, readonly string[]>> = {
  anywhere: ["board", "detail", "studio", "dashboard", "deck", "review", "call", "logbook", "issues", "report", "projects", "plan", "thread"],
  // The deck's list takes `j`, `k` and `Enter` as any list does. The acts that need a Job or Session in
  // hand (`list and detail`: attest, redirect, kill and the rest) are on its page, not on the deck.
  // The Reviews list is not here: it moves by `j`, `k` and `Enter` as a list does, but its `r` goes to the reader, where a
  // list's `r` reviews a Job, and a pull request in it has none.
  list: ["board", "dashboard", "deck"],
  "list and detail": ["board", "detail", "dashboard"],
  "open studio": ["studio"],
  // `w` and `?` are the Dashboard's and the deck reads them too.
  dashboard: ["dashboard", "deck"],
  // The deck's own panel, tile and page keys. A call pane in front takes the bare digits and letters it
  // names while it is up, and the deck's handlers stand down for it: that is a state of the deck, which
  // the keymap's places do not hold, so `call` and `deck` do not overlap here.
  deck: ["deck"],
  // The Reviews tab is a tab of the places bar, not the deck, so its letters do not meet the deck's.
  review: ["review"],
  // The Issues tab is a tab of the places bar too, so its letters meet neither the deck's nor Reviews'.
  issues: ["issues"],
  // A reported bug's row or its reader holds focus while these keys are pressed, and they take `D` from the tab's describe key there.
  // It is a place of its own so the two are not flagged as one key bound twice.
  report: ["report"],
  // The Projects tab is a tab of the places bar too, so its letters meet neither the deck's nor Issues'.
  projects: ["projects"],
  call: ["call"],
  // The Logbook is a tab of the places bar too, and its letters meet no other scope's.
  logbook: ["logbook"],
  // A plan card or the plan in full holds focus while its keys are pressed, and takes them before the page it sits on.
  plan: ["plan"],
  // The cards waiting in a Job's thread and on its composer. The `⌥` keys answer from anywhere in the Job's view, the message box
  // included, and meet the global keys there; a hand off band's letters answer only inside the band, so they meet neither a plan's nor a list's.
  thread: ["thread"],
};

const overlaps = (a: ActionScope, b: ActionScope): boolean => PLACES[a].some((place) => PLACES[b].includes(place));

/** The chords a slot covers: one for a key, one per digit for a row of them. */
function chordsIn(slot: Slot): Chord[] {
  if (slot.kind === "chord") return slot.chord === null ? [] : [slot.chord];
  if (slot.mods === null) return [];
  const all: Chord[] = [];
  for (let digit = slot.from; digit <= slot.to; digit += 1) all.push({ ...slot.mods, key: String(digit) });
  return all;
}

const sameChord = (a: Chord, b: Chord): boolean =>
  a.key === b.key && a.alt === b.alt && a.shift === b.shift && a.meta === b.meta && a.ctrl === b.ctrl;

/**
 * The other acts that answer the same key as this slot, somewhere both are listening.
 *
 * **An act that is `unbuilt` listens to nothing**, so it neither conflicts nor is conflicted with: the
 * registry is ahead of the code on purpose, and a key nothing answers cannot fight for one.
 */
export function conflictsOf(id: string, slot: Slot): Action[] {
  const act = ACTION[id];
  if (act === undefined || act.unbuilt !== null) return [];
  const mine = chordsIn(slot);
  if (mine.length === 0) return [];
  return ACTIONS.filter(
    (other) =>
      other.id !== id &&
      other.unbuilt === null &&
      overlaps(act.scope, other.scope) &&
      slotsOf(other.id).some((theirs) => chordsIn(theirs).some((chord) => mine.some((one) => sameChord(one, chord)))),
  );
}

/** Whether two chords would answer the same press, ⌘ standing in for Control as `same` reads it, in either direction. */
export function sameKey(a: Chord, b: Chord): boolean {
  return same(a, b) || same(b, a);
}

/** The built act that answers this chord somewhere, in any scope: a key kept for a row has to be free everywhere. */
export function actOnChord(chord: Chord): Action | undefined {
  return ACTIONS.find((other) => other.unbuilt === null && slotsOf(other.id).some((slot) => chordsIn(slot).some((held) => sameKey(held, chord))));
}

// ---- How Settings groups them -------------------------------------------------------------------

/** One heading in Settings → Keyboard, and the scopes under it. */
export type KeymapGroup = { readonly title: string; readonly scopes: readonly ActionScope[] };

/** Where a person finds an act: by the place they would be standing when they press it. */
export const KEYMAP_GROUPS: readonly KeymapGroup[] = [
  { title: "Anywhere", scopes: ["anywhere"] },
  { title: "Lists", scopes: ["list", "list and detail"] },
  { title: "Dashboard", scopes: ["dashboard", "deck"] },
  { title: "Calls", scopes: ["call"] },
  { title: "Reviews", scopes: ["review"] },
  { title: "Logbook", scopes: ["logbook"] },
  { title: "Plans", scopes: ["plan"] },
  { title: "Job thread", scopes: ["thread"] },
  { title: "Issues", scopes: ["issues", "report"] },
  { title: "Projects", scopes: ["projects"] },
  { title: "Studios", scopes: ["open studio"] },
];

/** The acts under one heading, in registry order. Every act is under exactly one. */
export function actsInGroup(group: KeymapGroup): Action[] {
  return ACTIONS.filter((act) => group.scopes.includes(act.scope));
}
