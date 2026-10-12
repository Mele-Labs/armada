import { useEffect } from "react";
import type { LucideIcon } from "lucide-react";

import { chordOf, formatChord, formatSlot, type Slot } from "../../keymap";
import { Button } from "../../primitives/Button/Button";
import { Kbd } from "../../primitives/Kbd/Kbd";

/** One key an act answers, as Settings → Keyboard draws it. */
export type KeyBindingSlotView = {
  slot: Slot;
  /** What this key does, where an act's keys do different things — `Back`, `Forward`. */
  name?: string;
  /** The other acts that answer the same key where this one listens. */
  conflicts?: readonly string[];
};

/**
 * One act and every key it answers, each a control: press it and the next key pressed is the new
 * binding, as a game's controls screen works. **Esc lets go without changing anything** and Delete
 * unbinds the key, so neither can be bound by pressing it — Reset puts either back.
 *
 * A key another act answers where this one listens is drawn as a warning naming that act, and kept:
 * a person moving two keys at once passes through a moment where both hold one.
 */
export type KeyBindingRowProps = {
  verb: string;
  icon?: LucideIcon | null;
  slots: readonly KeyBindingSlotView[];
  /** Which slot is waiting for a key, or `null` for none. One row in a screen listens at a time. */
  listening: number | null;
  onListen: (slot: number | null) => void;
  /** A key pressed for a slot, spelled as the registry spells one; `""` unbinds it. */
  onBind: (slot: number, spelled: string) => void;
  /** Present where the act is off the registry's keys; puts them back. */
  onReset?: () => void;
  /** Said under the verb — an act nothing answers yet. */
  note?: string;
};

export function KeyBindingRow({ verb, icon: Icon, slots, listening, onListen, onBind, onReset, note }: KeyBindingRowProps) {
  useEffect(() => {
    if (listening === null) return;
    const view = slots[listening];
    if (view === undefined) return;
    // The capture phase and stopped there, so nothing under Settings acts on the key being bound.
    const pressed = (event: KeyboardEvent): void => {
      event.preventDefault();
      event.stopImmediatePropagation();
      if (event.key === "Escape") return onListen(null);
      if (event.key === "Backspace" || event.key === "Delete") return bind("");
      const chord = chordOf(event);
      if (chord === null) return;
      if (view.slot.kind === "chord") return bind(formatChord(chord));
      // A row of digits takes the modifiers held with any digit.
      if (!/^\d$/.test(chord.key)) return;
      const { key: _digit, ...mods } = chord;
      bind(formatSlot({ ...view.slot, mods }));
    };
    const bind = (spelled: string) => {
      onBind(listening, spelled);
      onListen(null);
    };
    window.addEventListener("keydown", pressed, true);
    return () => window.removeEventListener("keydown", pressed, true);
  }, [listening, slots, onBind, onListen]);

  const conflicted = slots.flatMap((one) => one.conflicts ?? []);
  return (
    <div className="armada-key-binding-row" role="group" aria-label={verb}>
      <div className="armada-key-binding-row__act">
        {Icon == null ? <span className="armada-key-binding-row__mark" /> : <Icon className="armada-key-binding-row__mark" size={16} aria-hidden />}
        <span className="armada-key-binding-row__verb">
          {verb}
          {note === undefined ? null : <span className="armada-key-binding-row__note">{note}</span>}
          {conflicted.length === 0 ? null : <span className="armada-key-binding-row__conflict">Also {[...new Set(conflicted)].join(", ")}</span>}
        </span>
      </div>
      <div className="armada-key-binding-row__keys">
        {slots.map((one, at) => {
          const spelled = formatSlot(one.slot);
          const waiting = listening === at;
          const named = one.name === undefined ? verb : `${verb}, ${one.name.toLowerCase()}`;
          return (
            <button
              key={at}
              type="button"
              className="armada-key-binding-row__key"
              data-listening={waiting ? "" : undefined}
              data-conflict={(one.conflicts?.length ?? 0) > 0 ? "" : undefined}
              aria-pressed={waiting}
              aria-label={waiting ? `${named}: press a key, Esc to cancel, Delete to unbind` : `${named}: ${spelled === "" ? "unbound" : spelled}`}
              onClick={() => onListen(waiting ? null : at)}
            >
              {one.name === undefined ? null : <span className="armada-key-binding-row__slot">{one.name}</span>}
              {waiting ? <span className="armada-key-binding-row__waiting">Press a key</span> : spelled === "" ? <span className="armada-key-binding-row__unbound">Unbound</span> : <Kbd>{spelled}</Kbd>}
            </button>
          );
        })}
        <span className="armada-key-binding-row__reset">
          {onReset === undefined ? null : (
            <Button variant="ghost" size="sm" aria-label={`Reset ${verb}`} onClick={onReset}>
              Reset
            </Button>
          )}
        </span>
      </div>
    </div>
  );
}
