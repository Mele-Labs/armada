// settings.json on the wire: `get_settings`, `save_settings` and `settings.changed`.
// `crates/ipc/operations/get_settings.toml`, `save_settings.toml`, `settings.changed.toml`.
//
// **The one place the shape is spelled on this side**, mirroring `crates/ipc/src/settings.rs`. When
// the wire is generated from `crates/ipc`, this file is what the generated one replaces. The header
// rules in `protocol.ts` hold otherwise. **Every field is always present**: Fleet writes `null` for
// nothing saved, no variable and no refusal, rather than leaving the key out.

/** What kind of value a setting holds, with its bounds or its choices. Tagged on `kind`. */
export type SettingKind =
  | { kind: "integer"; min: number; max: number; unit?: string | null }
  /** Stored in seconds; Bridge reads it out in minutes or hours as well. */
  | { kind: "seconds"; min: number; max: number }
  | { kind: "boolean" }
  | { kind: "choice"; options: string[] }
  | { kind: "text" }
  /** Several lines of text, whose default is the prompt Armada ships. */
  | { kind: "prompt" }
  | { kind: "text_list" }
  /** A value Bridge owns and draws on its own pane, never in the generic form. */
  | { kind: "json" };

/** A setting's value as JSON carries it. Its kind says which of these it is. */
export type SettingValue = number | boolean | string | string[] | { [key: string]: unknown } | unknown[] | null;

/** One setting, as Fleet resolved it. */
export type Setting = {
  /** Dotted camelCase, `limits.dronesAtOnce`. The name in settings.json. */
  key: string;
  /** The group the section is listed under in Settings' index, as `Agents` over Harness, Model and Effort. */
  group: string;
  /** The sub-heading inside the group. */
  section: string;
  title: string;
  description: string;
  kind: SettingKind;
  default: SettingValue;
  /** What is in force: an environment variable, then settings.json, then the default. */
  value: SettingValue;
  /** What settings.json holds for it, or `null` where the key is not in the file. */
  saved: SettingValue | null;
  applies: "live" | "at_restart";
  /** The saved value is not the one in force until Fleet restarts. */
  pending_restart: boolean;
  /** The environment variable that wins over settings.json on this machine, as `ARMADA_MODEL`. */
  overridden_by_env: string | null;
};

/** Why the last read of settings.json was refused. The last good settings stay in force. */
export type SettingsRefused = {
  /** The key at fault, or `null` where the file is not a JSON object at all. */
  key: string | null;
  reason: string;
};

/** `GET /settings`, the answer to `POST /settings/save`, and the body of `settings.changed`. */
export type SettingsList = {
  /** settings.json's absolute path. It need not exist yet. */
  path: string;
  refused: SettingsRefused | null;
  settings: Setting[];
};

/**
 * `POST /settings/save`'s body. `null` removes the key, so the shipped default is back in force. A
 * value out of range or a key Fleet does not know is a 422 `fleet.unacceptable_settings`, whose
 * `key` field names the setting at fault, and nothing is saved.
 */
export type SaveSettings = { changes: Record<string, SettingValue | null> };
