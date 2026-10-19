// Settings' registration: its members come from `@armada/settings/fake`.

import { MOCK_SETTINGS, savedInto, settingsApi } from "@armada/settings/fake";

import type { SettingsApi, SettingsState } from "../../../../shared/api/settings";
import { SETTINGS_NOTHING_YET } from "../../../../shared/api/settings";
import type { Slice } from "../fake-context";

/** The preferences Fleet keeps as text: `SavePreference.text`. */
const TEXT_PREFERENCES = ["theme", "layout_choices", "key_bindings", "time_format"];

export const settings: Slice<SettingsApi, SettingsState> = {
  name: "settings",
  state: SETTINGS_NOTHING_YET,
  // A connected Fleet has answered `get_settings` by the time a window draws, unless the scenario says what.
  seeded: (state) => (state.settings === null && state.connection.state === "connected" ? { settings: MOCK_SETTINGS } : {}),
  // Fleet answers a save with every preference or setting now in force, and `main` republishes them: so does this.
  api: (_scenario, fleet) => ({
    ...settingsApi(),
    savePreference: async (save) => {
      // The text preferences are answered as their text, the rest as their switch.
      const text = TEXT_PREFERENCES.includes(save.name);
      fleet.publish({ preferences: { ...fleet.state().preferences, [save.name]: text ? save.text : save.value } });
      return { ok: true };
    },
    saveSettings: async (changes) => {
      const held = fleet.state().settings;
      if (held !== null) fleet.publish({ settings: savedInto(held, changes) });
      return { ok: true };
    },
  }),
};
