// How the owner has Bridge draw a time, over Fleet: read from the `time_format` preference into the store
// `@armada/components` draws every stamp from, and saved back there when Settings → Time moves a switch.
// `fleet-keymap.ts`' shape: a refused save puts the format back as Fleet has it and says why.

import type { Outcome, Preferences, SavePreference } from "@armada/protocol";
import { parseTimeFormat, setTimeFormat, setTimeFormatSaver } from "@armada/components";
import { refusalWords } from "@armada/screens/src/refusal-words";

import { askToTell } from "./tell";

type Facts = { preferences: Preferences };

/** What Bridge reaches Fleet through: `window.armada` in the app, a fake in a test. */
export type FleetTimeFormat = {
  state: () => Promise<Facts>;
  subscribe: (on: (facts: Facts) => void) => () => void;
  savePreference: (save: SavePreference) => Promise<Outcome>;
};

export function wireFleetTimeFormat(fleet: FleetTimeFormat, tell: (sentence: string) => void = askToTell): void {
  let saving = 0;
  let held = "";

  const ingest = (facts: Facts): void => {
    held = facts.preferences.time_format ?? "";
    // A save in flight is newer than whatever Fleet published before it landed.
    if (saving === 0) setTimeFormat(parseTimeFormat(held));
  };

  setTimeFormatSaver((text) => {
    saving += 1;
    void fleet.savePreference({ name: "time_format", value: false, text }).then((outcome) => {
      saving -= 1;
      if (outcome.ok) return;
      setTimeFormat(parseTimeFormat(held));
      tell(refusalWords(outcome));
    });
  });

  fleet.state().then(ingest, () => undefined);
  fleet.subscribe(ingest);
}
