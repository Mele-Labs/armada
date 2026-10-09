// Mods: a folder on this machine that changes how Bridge looks —
// `crates/ipc/src/mods.rs`, `docs/concepts/mods.md`.
//
// Read off `list_mods` once per connection and replaced whole by every `mods.changed`. The header
// rules in `events.ts` hold: hand-written, and every closed set left as `string` unless Bridge
// matches on it, which `ModKind` is.

/** What a mod changes. A `mod.toml` naming another kind makes the mod invalid, so a valid row's is one of these. */
export type ModKind = "theme" | "layout";

/** One mod on disk. A broken one is a row with `valid` false, never a failed read. */
export type ModSummary = {
  /** The folder's name, a slug. */
  name: string;
  /** Absent where `mod.toml` could not be read or names a kind this build does not have. */
  kind?: ModKind;
  version?: string;
  description?: string;
  /** This machine's switch. A mod nobody switched is on. */
  enabled: boolean;
  /** Whether Bridge may load it. */
  valid: boolean;
  /** The first thing wrong, and how many more. Present only where `valid` is false. */
  reason?: string;
  /** When the newest of the mod's own files was written. */
  changed_at?: string;
};

/** `GET /mods`, and the body of `mods.changed`: every mod by name, which a client replaces its list with. */
export type ModList = { mods: ModSummary[] };

/** `POST /mods/scaffold`'s body. */
export type ScaffoldMod = {
  name: string;
  description?: string;
  /** Absent is a theme. */
  kind?: ModKind;
};

/** A mod made. `path` is the folder, where `theme.css` or `layout.json` is edited. */
export type ModScaffolded = { name: string; path: string };

/** `POST /mods/enable`'s body: this machine's switch for one mod. */
export type SetModEnabled = { name: string; enabled: boolean };

/** `GET /mods/validate?name=`. `problems` is empty exactly when `valid`. */
export type ModChecked = {
  name: string;
  valid: boolean;
  problems: string[];
  /** The stylesheet as it was checked, present only where it passed. Bridge injects this text and no other. */
  css?: string;
  /** A layout mod's `layout.json` as it was checked, present only where it passed. Bridge applies this text and no other. */
  layout?: string;
};

/** `POST /mods/promote`'s body. */
export type PromoteMod = {
  name: string;
  /** Absent is the first repository Fleet serves. */
  manifest_id?: string;
};

/** A mod on a branch of the repository. Pushed nowhere, and no pull request opened. */
export type ModPromoted = { name: string; branch: string; commit: string };
