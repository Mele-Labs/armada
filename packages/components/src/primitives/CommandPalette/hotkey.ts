// The rules for a key given to a palette row, kept in one place because the palette's panel and Settings' table both
// take one, and a key either refuses must be refused by both.

import { actOnChord, sameKey, parseChord, type Chord } from "../../keymap";

/** Another row's key, with the name it goes by. */
export type HeldHotkey = { label: string; hotkey: string };

/** What a Mac or a text field already does with these, which no window of Bridge should take. */
const SYSTEM: readonly string[] = ["⌘C", "⌘V", "⌘X", "⌘A", "⌘Z", "⇧⌘Z", "⌘Q", "⌘W", "⌘M", "⌘H", "⌘,"];

const sameAs = (spelled: string, chord: Chord): boolean => {
  const held = parseChord(spelled);
  return held !== null && sameKey(held, chord);
};

/**
 * Why a chord cannot be a row's key, in a few words, or nothing where it can. It needs ⌘ or ⌃, and it is not ⌘1 to ⌘9
 * (which open results), not a key an act answers anywhere in Bridge, not one the system keeps, and not another row's.
 */
export function refuseHotkey(chord: Chord, others: readonly HeldHotkey[]): string | undefined {
  if (!chord.meta && !chord.ctrl) return "Needs ⌘ or ⌃";
  if (chord.meta && !chord.ctrl && !chord.alt && !chord.shift && /^[1-9]$/.test(chord.key)) return "⌘1 to ⌘9 open results";
  const act = actOnChord(chord);
  if (act !== undefined) return `Bound to ${act.verb}`;
  if (SYSTEM.some((one) => sameAs(one, chord))) return "Kept by the system";
  const row = others.find((one) => sameAs(one.hotkey, chord));
  return row === undefined ? undefined : `Bound to ${row.label}`;
}
