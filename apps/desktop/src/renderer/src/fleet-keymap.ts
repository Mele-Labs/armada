// The owner's key bindings, over Fleet: read from the `key_bindings` preference and laid over the
// registry's, and saved back there when Settings → Keyboard moves one. `docs/concepts/key-bindings.md`.
//
// **The keymap lives in `@armada/components`**, where every handler already asks it what a press
// means; this only feeds it what Fleet holds and hands it Fleet's save. A refused save puts the keys
// back as Fleet has them and says why, `fleet-layout.ts`' terms.

import type { Outcome, Preferences, SavePreference } from "@armada/protocol";
import { parseKeyBindings, setKeyBindingSaver, setKeyBindings } from "@armada/components";
import { refusalWords } from "@armada/screens/src/refusal-words";

import { askToTell } from "./tell";

type Facts = { preferences: Preferences };

/** What Bridge reaches Fleet through: `window.armada` in the app, a fake in a test. */
export type FleetKeymap = {
  state: () => Promise<Facts>;
  subscribe: (on: (facts: Facts) => void) => () => void;
  savePreference: (save: SavePreference) => Promise<Outcome>;
};

export function wireFleetKeymap(fleet: FleetKeymap, tell: (sentence: string) => void = askToTell): void {
  let saving = 0;
  let held = "";

  const ingest = (facts: Facts): void => {
    held = facts.preferences.key_bindings ?? "";
    // A save in flight is newer than whatever Fleet published before it landed.
    if (saving === 0) setKeyBindings(parseKeyBindings(held));
  };

  setKeyBindingSaver((text) => {
    saving += 1;
    void fleet.savePreference({ name: "key_bindings", value: false, text }).then((outcome) => {
      saving -= 1;
      if (outcome.ok) return;
      setKeyBindings(parseKeyBindings(held));
      tell(refusalWords(outcome));
    });
  });

  fleet.state().then(ingest, () => undefined);
  fleet.subscribe(ingest);
}
