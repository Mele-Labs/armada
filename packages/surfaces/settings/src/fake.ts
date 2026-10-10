// Settings' mock members: the preferences and settings a person saves. It has no Fleet of
// its own, so these are the slice's route stubs, and the settings list `get_settings` would answer.
import type { Outcome } from "@armada/protocol";

import type { SettingsApi } from "./api";

export { MOCK_SETTINGS, savedInto } from "./fake-settings";

const OK: Outcome = { ok: true };

export const settingsApi = (): SettingsApi => ({
  savePreference: async () => OK,
  saveSettings: async () => OK,
  openSettingsFile: async () => ({ ok: true }),
  validateMod: async () => null,
  setModEnabled: async () => OK,
  promoteMod: async () => OK,
  phone: async () => ({ ok: false, why: "unreachable" }),
});
