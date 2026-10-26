// Settings: settings.json, Fleet's admission limits and a person's preferences.
// Types and values only, no React. A slice imports protocol and screens, never another slice;
// desktop's `shared/api/settings.ts` re-exports it and `shared/api.ts` composes it.

import type {
  Outcome,
  SavePreference,
  FleetLimits,
  ModChecked,
  ModList,
  Preferences,
  SaveSettings,
  SettingsList,
} from "@armada/protocol";

/**
 * One thing Bridge asks about the phone. **A named operation and never a path**, so the renderer
 * can reach exactly these ten routes and no other. Main makes the call: the Gateway's admin routes
 * refuse any request that carries `Origin`, which a renderer's own fetch always does.
 *
 * `access` and `turn` are Fleet's `GET /phone` and `POST /phone`, answering a `PhoneAccess`; Fleet
 * runs the Gateway itself, so the switch is Fleet's. The rest go to the Gateway.
 */
export type PhoneRequest =
  | { op: "access" }
  | { op: "turn"; on: boolean }
  | { op: "start" }
  | { op: "pending" }
  | { op: "confirm"; code: string }
  | { op: "devices" }
  | { op: "unpair"; id: string }
  | { op: "status" }
  /** How far this Mac is along the way in: Tailscale installed, signed in, and Funnel on. */
  | { op: "setup" }
  /** Turn Funnel on in front of the Gateway. */
  | { op: "funnel" };

/** Fleet's answer on `/phone`. `why` is Fleet's sentence where access is on and nothing listens. */
export type PhoneAccess = { on: boolean; port: number; listening: boolean; why?: string };

/**
 * `body` is the route's JSON, or `null` for a 204. `unreachable` is a refused connection to the
 * Gateway. A refusal from Fleet arrives as `refused`, in Fleet's words.
 */
export type PhoneAnswer =
  | { ok: true; body: unknown }
  | { ok: false; why: "unreachable" }
  | { ok: false; why: "refused"; said: string };

/**
 * What happened to a press of "Open settings.json". `not_read` is Fleet not having said where the
 * file is yet; `refused` carries the sentence the editor or the OS gave.
 */
export type SettingsFileOpened =
  | { ok: true }
  | { ok: false; why: "not_read" }
  | { ok: false; why: "refused"; detail: string };

export type SettingsApi = {
  /**
   * Change settings in settings.json, by key. `null` removes a key, so its shipped default is back.
   * **Every change in one save or none**: Fleet refuses a value out of range whole. `settings` is
   * republished with Fleet's answer.
   */
  saveSettings: (changes: SaveSettings["changes"]) => Promise<Outcome>;
  /**
   * Have Fleet ask the harness for its models again. **Nothing returns**: the answer is
   * `holds.models`, republished.
   */
  askModels: () => Promise<void>;
  /**
   * Open settings.json in the editor `editor.command` names. **No path crosses this**: main opens the
   * one `settings.path` named, so the renderer cannot ask for any other file.
   */
  openSettingsFile: () => Promise<SettingsFileOpened>;
  /**
   * Save one preference by name. **Fleet-wide and never a Job's own act**, and
   * one preference at a time: saving one leaves every other one untouched.
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
  /** Put a valid mod into the repository by a Job of its own. `modPromoted` on the answer names the Job and its branch, and the pull request or why none opened. */
  promoteMod: (name: string) => Promise<Outcome>;
  /** Try a tracker mod against the first repository Fleet serves. `trackerTried` on the answer says how it went, passed or not; a refusal is the Outcome's. */
  tryTrackerMod: (name: string) => Promise<Outcome>;
  /** One request about the phone, to Fleet or to the Gateway on loopback, made by Bridge's main process. */
  phone: (request: PhoneRequest) => Promise<PhoneAnswer>;
};

export type SettingsState = {
  /**
   * Every setting in settings.json and what is in force for each, or `null` before the first read.
   * Read once per connection and replaced whole by every `settings.changed` and every save.
   */
  settings: SettingsList | null;
  /**
   * Fleet's three admission limits, and what Armada ships them at, or `null`
   * before the first read.
   *
   * **Read once per connection and again on every `settings.changed`** — `capacity`'s
   * reason for `null` rather than a stale figure, and `manifestReading`'s for
   * not re-reading on a timer: nothing but a change to settings.json moves it.
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
  settings: null,
  limits: null,
  preferences: { where_things_are_open: false },
  mods: null,
};

export const SETTINGS_CHANNELS = {
  saveSettings: "bridge:save-settings",
  askModels: "bridge:ask-models",
  openSettingsFile: "bridge:open-settings-file",
  savePreference: "bridge:save-preference",
  validateMod: "bridge:validate-mod",
  setModEnabled: "bridge:set-mod-enabled",
  promoteMod: "bridge:promote-mod",
  tryTrackerMod: "bridge:try-tracker-mod",
  phone: "bridge:phone",
} as const;
