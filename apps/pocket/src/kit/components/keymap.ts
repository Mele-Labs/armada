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

/** One key with its modifiers. `key` is a letter in lower case, a digit, a symbol, or a named key. */
export type Chord = {
  readonly ctrl: boolean;
  readonly alt: boolean;
  readonly shift: boolean;
  readonly meta: boolean;
  readonly key: string;
};

/** One thing a person can bind: a key, or the modifiers that go with a row of digits. */
export type Slot =
  | {
      readonly kind: "chord";
      readonly chord: Chord | null;
    }
  | {
      readonly kind: "digits";
      readonly mods: Omit<Chord, "key"> | null;
      readonly from: number;
      readonly to: number;
    };

/** A press, as a handler receives it. `code` reads a key by place when Option has changed what it types. */
export type Press = Pick<KeyboardEvent, "key" | "metaKey" | "ctrlKey" | "altKey"> & {
  readonly shiftKey?: boolean;
  readonly code?: string;
};

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
const MODIFIER_KEYS = new Set([
  "Meta",
  "Control",
  "Alt",
  "Shift",
  "CapsLock",
  "Fn",
  "OS",
  "Hyper",
  "Super",
]);

const isLetter = (key: string): boolean => /^[a-z]$/i.test(key);
const isNamed = (key: string): boolean =>
  Object.values(NAMED).includes(key) || /^F\d{1,2}$/.test(key);

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
  if (named !== undefined)
    return {
      ctrl,
      alt,
      shift,
      meta,
      key: named,
    };
  if (/^F\d{1,2}$/.test(rest))
    return {
      ctrl,
      alt,
      shift,
      meta,
      key: rest,
    };
  if (
    [
      ...rest,
    ].length !== 1
  )
    return null;
  if (isLetter(rest)) {
    // Bare, an upper-case letter is the shifted one (`B`); beside ⌘, ⌃ or ⌥ it is only how it is drawn.
    const bare = !ctrl && !alt && !meta;
    return {
      ctrl,
      alt,
      meta,
      shift: shift || (bare && rest !== rest.toLowerCase()),
      key: rest.toLowerCase(),
    };
  }
  return {
    ctrl,
    alt,
    meta,
    shift: shiftCounts(rest) ? shift : false,
    key: rest,
  };
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
    return [
      {
        kind: "digits",
        mods: probe === null ? null : withoutKey(probe),
        from: Number(range[2]),
        to: Number(range[3]),
      },
    ];
  }
  const parts = text.includes(" / ") ? text.split(" / ") : text.split(" ");
  return parts.map((part) => ({
    kind: "chord",
    chord: part.trim() === "" ? null : parseChord(part),
  }));
}

const withoutKey = ({ ctrl, alt, shift, meta }: Chord): Omit<Chord, "key"> => ({
  ctrl,
  alt,
  shift,
  meta,
});

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
  history: [
    "Back",
    "Forward",
  ],
  move_focus: [
    "Down",
    "Up",
    "Down",
    "Up",
  ],
  deck_reorder: [
    "Up",
    "Down",
  ],
  projects_reorder: [
    "Up",
    "Down",
  ],
  kit_move: [
    "Down",
    "Up",
    "Down",
    "Up",
  ],
  kit_pane: [
    "Next",
    "Previous",
  ],
  kit_scroll: [
    "Page down",
    "Page up",
    "Page down",
    "Page up",
    "Top",
    "Bottom",
  ],
  features_move: [
    "Down",
    "Up",
    "Down",
    "Up",
  ],
  finding_move: [
    "Down",
    "Up",
    "Down",
    "Up",
  ],
};

// ---- What this person has changed ------------------------------------------------------------

/** The acts a person has rebound, by id, each to the spelling of every slot it holds. */
export type KeyBindings = Readonly<Record<string, readonly string[]>>;

let overrides: KeyBindings = {};
let version = 0;
const listeners = new Set<() => void>();

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
      if (text === "")
        return {
          ...slot,
          mods: null,
        };
      const read = parseSlots(text)[0];
      return read?.kind === "digits"
        ? {
            ...slot,
            mods: read.mods,
          }
        : slot;
    }
    return {
      kind: "chord",
      chord: text === "" ? null : parseChord(text),
    };
  });
}

/** The key an act answers now, spelled for drawing. `""` where a person has unbound every slot. */
export function boundKeyFor(id: string): string {
  const act = ACTION[id];
  if (act !== undefined && overrides[id] === undefined) return act.shortcut;
  return formatSlots(id, slotsOf(id));
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
  if (press.key === "Dead" && !(press.altKey && /^(Key[A-Z]|Digit\d)$/.test(press.code ?? "")))
    return null;
  const key = keyOf(press);
  // An upper-case letter is a shifted one whatever the flag says — Caps Lock types it too, and
  // `B` has always answered to the letter typed rather than to the key held.
  const upper = isLetter(press.key) && press.key !== press.key.toLowerCase();
  const shift = shiftCounts(key) && (press.shiftKey === true || upper);
  return {
    ctrl: press.ctrlKey,
    alt: press.altKey,
    meta: press.metaKey,
    shift,
    key,
  };
}

/**
 * Whether a press is a chord. **⌘ answers to Control as well**, unless the chord asks for ⌃ itself:
 * every modified binding in Bridge has always taken either, so one learned on a Mac still works on a
 * keyboard without the key.
 */
function same(chord: Chord, press: Chord): boolean {
  if (chord.key !== press.key || chord.alt !== press.alt || chord.shift !== press.shift)
    return false;
  if (chord.meta && !chord.ctrl) return (press.meta || press.ctrl) && !(press.meta && press.ctrl);
  return chord.meta === press.meta && chord.ctrl === press.ctrl;
}

let recording = false;

/** Which slot of an act a press is, or `-1` for none. */
export function pressedSlot(id: string, press: Press): number {
  if (recording) return -1;
  const chord = chordOf(press);
  if (chord === null) return -1;
  return slotsOf(id).findIndex(
    (slot) => slot.kind === "chord" && slot.chord !== null && same(slot.chord, chord),
  );
}

/** Whether a press is one of the keys an act answers now. */
export function isPressed(id: string, press: Press): boolean {
  return pressedSlot(id, press) !== -1;
}
