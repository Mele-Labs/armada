// Settings' members: the limits and preferences a person saves.

import type { Outcome } from "@armada/protocol";

import type { SettingsApi, SettingsState } from "../../../../shared/api/settings";
import { SETTINGS_NOTHING_YET } from "../../../../shared/api/settings";
import type { Slice } from "../fake-context";

const OK: Outcome = { ok: true };

export const settings: Slice<SettingsApi, SettingsState> = {
  name: "settings",
  state: SETTINGS_NOTHING_YET,
  api: () => ({
    saveLimits: async () => OK,
    savePreference: async () => OK,
  }),
};
