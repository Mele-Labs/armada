// Settings' mock members: the limits and preferences a person saves. It has no Fleet of its own, so
// these are the slice's route stubs and nothing else.
import type { Outcome } from "@armada/protocol";

import type { SettingsApi } from "./api";

const OK: Outcome = { ok: true };

export const settingsApi = (): SettingsApi => ({
  saveLimits: async () => OK,
  savePreference: async () => OK,
  validateMod: async () => null,
  setModEnabled: async () => OK,
  promoteMod: async () => OK,
});
