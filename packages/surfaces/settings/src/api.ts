// Settings: Fleet's admission limits and a person's preferences.
// Types and values only, no React. A slice imports protocol and screens, never another slice;
// desktop's `shared/api/settings.ts` re-exports it and `shared/api.ts` composes it.

import type {
  Outcome,
  SaveLimits,
  SavePreference,
  FleetLimits,
  ModChecked,
  ModList,
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
  /**
   * Fleet's check of one mod, and the stylesheet if it passed. **The only text Bridge injects for a
   * mod** (`docs/concepts/mods.md`). `null` where Fleet could not be asked or knows no such mod.
   */
  validateMod: (name: string) => Promise<ModChecked | null>;
  /** This machine's switch for one mod. `mods` is republished with it, and `mods.changed` follows. */
  setModEnabled: (name: string, enabled: boolean) => Promise<Outcome>;
  /** Put a valid mod on a new branch of the repository. `modPromoted` on the answer names the branch; nothing is pushed. */
  promoteMod: (name: string) => Promise<Outcome>;
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
  /**
   * The mods on this machine, or `null` before the first read. **`null` is not "none"**: a theme
   * the preference names is not a mod that has gone until this has been read. Read once per
   * connection and replaced whole by every `mods.changed`.
   */
  mods: ModList | null;
};

export const SETTINGS_NOTHING_YET: SettingsState = {
  limits: null,
  preferences: { where_things_are_open: false },
  mods: null,
};

export const SETTINGS_CHANNELS = {
  saveLimits: "bridge:save-limits",
  savePreference: "bridge:save-preference",
  validateMod: "bridge:validate-mod",
  setModEnabled: "bridge:set-mod-enabled",
  promoteMod: "bridge:promote-mod",
} as const;
