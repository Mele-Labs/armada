// Settings → Keyboard: every act Bridge has a key for, grouped by where a person stands when they press
// it, each key a control that takes the next key pressed — a game's controls screen. What changes here
// is laid over `actions.toml`'s keys and kept by Fleet, `docs/concepts/key-bindings.md`.

import { useState } from "react";
import {
  ALIASES,
  Button,
  KEYMAP_GROUPS,
  KeyBindingRow,
  actsInGroup,
  conflictsOf,
  isRebound,
  keyBindings,
  keyFor,
  parseSlots,
  rebind,
  resetAllBindings,
  resetBinding,
  slotsOf,
  SLOT_NAMES,
  useKeyBindings,
  type Action,
  type Slot,
} from "@armada/components";

import { matches } from "./sections";

/** Whether an act is what a search is after: by its verb, a word that finds it, its key, or where it is. */
function found(act: Action, place: string, query: string): boolean {
  return matches([act.verb, act.id.replace(/_/g, " "), ...(ALIASES[act.id] ?? []), keyFor(act.id), place], query);
}

/** How many acts a search finds, for the count beside the category. */
export function keyboardMatches(query: string): number {
  return KEYMAP_GROUPS.reduce((all, group) => all + actsInGroup(group).filter((act) => found(act, group.title, query)).length, 0);
}

/** A slot with one key changed: the key pressed, or nothing where `spelled` is empty. */
function changed(slot: Slot, spelled: string): Slot {
  if (spelled === "") return slot.kind === "chord" ? { kind: "chord", chord: null } : { ...slot, mods: null };
  return parseSlots(spelled)[0] ?? slot;
}

export function KeyboardSettings({ query = "" }: { query?: string }) {
  useKeyBindings();
  const [listening, setListening] = useState<{ id: string; slot: number } | null>(null);
  const searching = query.trim() !== "";
  const groups = KEYMAP_GROUPS.map((group) => ({
    group,
    acts: actsInGroup(group).filter((act) => !searching || found(act, group.title, query)),
  })).filter((one) => one.acts.length > 0);

  return (
    <div className="armada-keyboard-settings">
      {searching ? null : (
        <p className="armada-keyboard-settings__lead">
          Press a key to change it, then the key you want. Esc cancels and Delete unbinds. A dialog or menu always closes on Esc.
        </p>
      )}
      {groups.map(({ group, acts }) => (
        <div key={group.title} className="armada-keyboard-settings__group" role="group" aria-label={group.title}>
          <p className="armada-keyboard-settings__caption">{group.title}</p>
          {acts.map((act) => {
            const slots = slotsOf(act.id);
            return (
              <KeyBindingRow
                key={act.id}
                verb={act.verb}
                icon={act.icon}
                slots={slots.map((slot, at) => {
                  const name = SLOT_NAMES[act.id]?.[at];
                  const conflicts = conflictsOf(act.id, slot).map((other) => other.verb);
                  return { slot, ...(name === undefined ? {} : { name }), ...(conflicts.length === 0 ? {} : { conflicts }) };
                })}
                listening={listening?.id === act.id ? listening.slot : null}
                onListen={(slot) => setListening(slot === null ? null : { id: act.id, slot })}
                onBind={(at, spelled) => rebind(act.id, slotsOf(act.id).map((slot, i) => (i === at ? changed(slot, spelled) : slot)))}
                {...(isRebound(act.id) ? { onReset: () => resetBinding(act.id) } : {})}
                {...(act.unbuilt === null ? {} : { note: `Nothing answers it yet · ${act.unbuilt}` })}
              />
            );
          })}
        </div>
      ))}
      {searching ? null : (
        <div className="armada-keyboard-settings__foot">
          <Button variant="secondary" disabled={Object.keys(keyBindings()).length === 0} onClick={resetAllBindings}>
            Reset all to defaults
          </Button>
        </div>
      )}
    </div>
  );
}
