// Settings: Fleet's admission limits and a person's preferences.
// A slice imports protocol and screens, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type {
  Outcome,
  SaveLimits,
  SavePreference,
  FleetLimits,
  Preferences,
} from "@armada/protocol";

export type SettingsApi = {
  /**
   * Change one or more of Fleet's three admission limits: drones at once, the
   * memory and the disk Fleet keeps free before starting another. **Fleet-wide
   * and never a Job's own act.**
   *
   * Applies the next time a Job is ready to start; nothing running stops. An
   * omitted field keeps its value. Fleet refuses a figure out of range in the
   * API's own error shape, and nothing here saves any of the three.
   */
  saveLimits: (values: SaveLimits) => Promise<Outcome>;
  /**
   * Save one preference by name. **Fleet-wide and never a Job's own act**,
   * `saveLimits`' reason — and one preference at a time, unlike `saveLimits`,
   * which sends every field it has an opinion on in one request: a preference
   * is a row keyed by name, so saving one leaves every other one untouched.
   *
   * What comes back is every preference now in force, and `BridgeState.preferences`
   * is republished with it — `readPreferences`' terms otherwise.
   */
  savePreference: (save: SavePreference) => Promise<Outcome>;
};

export type SettingsState = {
  /**
   * Fleet's three admission limits, and what Armada ships them at, or `null`
   * before the first read.
   *
   * **Read once per connection and republished on every save** — `capacity`'s
   * reason for `null` rather than a stale figure, and `manifestReading`'s for
   * not re-reading on a timer: nothing but a save changes it.
   */
  limits: FleetLimits | null;
  /**
   * A person's Bridge preferences, and what is in force for each.
   *
   * **Never `null`, unlike `limits`.** A screen reading this draws the
   * shipped default until the first read lands and its own choice after —
   * there is no "not read yet" state worth a screen's own branch, because the
   * default is itself a real, showable value. Read once per connection and
   * republished on every save, `limits`' terms otherwise.
   */
  preferences: Preferences;
};

export const SETTINGS_NOTHING_YET: SettingsState = {
  limits: null,
  preferences: { where_things_are_open: false },
};

export const SETTINGS_CHANNELS = {
  saveLimits: "bridge:save-limits",
  savePreference: "bridge:save-preference",
} as const;
