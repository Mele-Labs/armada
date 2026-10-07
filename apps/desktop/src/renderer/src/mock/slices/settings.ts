// Settings' registration: its members come from `@armada/settings/fake`.

import { settingsApi } from "@armada/settings/fake";

import type { SettingsApi, SettingsState } from "../../../../shared/api/settings";
import { SETTINGS_NOTHING_YET } from "../../../../shared/api/settings";
import type { Slice } from "../fake-context";

export const settings: Slice<SettingsApi, SettingsState> = {
  name: "settings",
  state: SETTINGS_NOTHING_YET,
  api: () => settingsApi(),
};
