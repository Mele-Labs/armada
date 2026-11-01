// Mods: a folder on this machine that changes how Bridge looks —
// `crates/ipc/src/mods.rs`, `docs/concepts/mods.md`.
//
// Read off `list_mods` once per connection and replaced whole by every `mods.changed`. The header
// rules in `events.ts` hold: hand-written, and every closed set left as `string` unless Bridge
// matches on it, which `ModKind` is.

import type { SessionNote } from "./session-notes";
import type { FleetRule } from "./teaching";

/**
 * What a mod changes. A `mod.toml` naming another kind makes the mod invalid, so a valid row's is one of these.
 * `words` is a pattern Bridge links, `fleet` what Fleet does with a notice, `tweak` token values layered over the theme,
 * `commands` rows added to the command palette, `tab` a tab Bridge loads as part of itself, drawn with the whole Fleet API.
 */
export type ModKind = "theme" | "layout" | "words" | "fleet" | "tweak" | "tracker" | "commands" | "tab";

/** Where a mod comes from: this machine's mods folder, or `packages/mods/` in the build Fleet runs. */
export type ModOrigin = "local" | "built_in";

/** One mod on disk. A broken one is a row with `valid` false, never a failed read. */
export type ModSummary = {
  /** The folder's name, a slug. A built-in mod's is `built-in:` and its folder's name, so it never answers to a local mod's. */
  name: string;
  /** Absent from a Fleet built before built-in mods, and read as `local`. */
  origin?: ModOrigin;
  /** Absent where `mod.toml` could not be read or names a kind this build does not have. */
  kind?: ModKind;
  version?: string;
  description?: string;
  /** What Teach was pressed on, where a person taught this mod. Absent for a mod written by hand. */
  taught_on?: string;
  /** The Logbook entry a teaching from a Job made this mod with. Absent for a mod written by hand or taught where no entry was made. */
  entry?: string;
  /** The Job whose `scaffold_mod` made this mod. Absent for a mod made by hand, by a teaching, or by a call that named no Job. */
  made_by?: string;
  /** This machine's switch. A mod nobody switched is on. */
  enabled: boolean;
  /** Whether Bridge may load it. */
  valid: boolean;
  /** The first thing wrong, and how many more. Present only where `valid` is false. */
  reason?: string;
  /** When the newest of the mod's own files was written. */
  changed_at?: string;
  /** A tracker mod's last try, present only where it tried the `tracker.toml` that is there now. */
  tried?: TrackerTry;
  /** A tracker mod that was tried, and whose `tracker.toml` has been edited since, so `tried` is absent. Left out otherwise, and for one never tried. */
  stale?: boolean;
  /** The pull request a promote opened for this mod, while it is open, and the Job that holds it. Absent once it merges or closes. */
  pull_request?: ModPullRequest;
};

/** A promoted mod's open pull request, as the Job of its own holds it on its row. */
export type ModPullRequest = {
  address: string;
  number: number;
  /** What the promote titled the pull request and its Job: `Add the calm theme mod`. */
  title: string;
  /** The Job of its own that holds it. */
  session_id: string;
};

/** `GET /mods`, and the body of `mods.changed`: every mod by name, which a client replaces its list with. */
export type ModList = { mods: ModSummary[] };

/** `POST /mods/scaffold`'s body. */
export type ScaffoldMod = {
  name: string;
  description?: string;
  /** Absent is a theme. */
  kind?: ModKind;
  /** The Job calling, which the mod records as `made_by`. */
  session_id?: string;
};

/**
 * `POST /mods/session`'s body: a Mod Job, a hosted Job on Armada's own repository for modding Armada
 * itself. Not both `note` and `said`, nor a blank `said`: a 422 `fleet.unacceptable_mod_start`. `note` is what he
 * pointed at, as `send_session_note` takes it, and is the Job's first message.
 * Where Fleet does not serve that repository, the Job starts in the mods folder and makes mods only.
 */
export type StartModSession = {
  /** Absent is the first line of the note's words, or of `said`. */
  title?: string;
  model?: string;
  effort?: string;
  /** What he pointed at. Exactly one of `note` and `said`. Neither: an empty Mod Job, whose brief rides ahead of his first message. */
  note?: SessionNote;
  /** His words, where Mod was pressed on nothing: the Job's first message after the brief. */
  said?: string;
};

/** What he picked in an empty Mod Job: the kind of mod he is aiming for, `source` for a change to Armada's code. Read with his first message only. */
export type ModAim = "theme" | "layout" | "words" | "fleet" | "tracker" | "commands" | "source";

/** A Mod Job started. `send_failed` is why its first message was not sent, where it was refused; open the Job and send by hand. */
export type ModSessionStarted = { session_id: string; title: string; send_failed?: string };

/** A mod made. `path` is the folder, where `theme.css` or `layout.json` is edited. */
export type ModScaffolded = { name: string; path: string };

/** `POST /mods/enable`'s body: this machine's switch for one mod. */
export type SetModEnabled = { name: string; enabled: boolean };

/** `GET /mods/validate?name=`. `problems` is empty exactly when `valid`. */
export type ModChecked = {
  name: string;
  valid: boolean;
  problems: string[];
  /** A theme's or a tweak's stylesheet as it was checked, present only where it passed. Bridge injects this text and no other. */
  css?: string;
  /** A layout mod's `layout.json` as it was checked, present only where it passed. Bridge applies this text and no other. */
  layout?: string;
  /** A Words mod's `words.toml` as it was checked, present only where it passed. */
  words?: WordsFile;
  /** A Fleet mod's `fleet.toml` as it was checked, present only where it passed. */
  fleet?: FleetFile;
  /** A Commands mod's `commands.toml` as it was checked, present only where it passed. */
  commands?: CommandsFile;
};

/** Where a pattern opens in one repository. */
export type WordsAddress = {
  /** A Manifest id, or `*` for every repository without an address of its own. */
  repository: string;
  /** An `http` or `https` address with `{match}` where the matched words go. */
  template: string;
  /** `settings`, `pull_request #N` or `person`. */
  found: string;
};

/** A `words.toml`: a pattern learned once, and an address for each repository that has one. */
export type WordsFile = {
  /** What the pattern finds, as a person would say it: `Jira key`. */
  name: string;
  /** A regular expression Bridge runs as a JavaScript one. */
  pattern: string;
  /** Words the pattern matches whole. */
  example: string;
  /** Absent when the file holds none. */
  address?: WordsAddress[];
};

/** A `fleet.toml`: what Fleet does with notices. */
export type FleetFile = {
  /** Absent when the file holds no rule. */
  rule?: FleetRule[];
};

/**
 * One row a Commands mod adds to the command palette. **Exactly one of `prompt`, `url`, `shell` and `chain` is set**
 * in a file that passed, which is what the row does: start a Job on the filled prompt, open the address, run the line
 * in the picked repository's checkout, or run the palette rows named, in order.
 */
export type PaletteCommand = {
  /** The row's label, and what it is told apart by: no two rows in a file share one. */
  name: string;
  /** Searched, never drawn. Absent when the row has none. */
  aliases?: string[];
  /** A chord in the registry's spelling: `⌥⌘S`. */
  hotkey?: string;
  prompt?: string;
  url?: string;
  shell?: string;
  /** Palette row ids, run in order. */
  chain?: string[];
  /** The `{name}` blanks the prompt, address or line holds, once each in the order they first appear. Absent when it holds none. */
  blanks?: string[];
};

/** A `commands.toml`: rows for the command palette. */
export type CommandsFile = {
  command: PaletteCommand[];
};

/** `POST /mods/try`'s body: the tracker mod, and the repository to try it against. Absent is the first served whose Manifest names this mod as its tracker, else the first served. */
export type TryTrackerMod = { name: string; manifest_id?: string };

/** How a tracker mod's try went. Only a tracker that passed one is used by a Manifest naming it. */
export type TrackerTry = {
  at: string;
  manifest_id: string;
  passed: boolean;
  /** Why it did not pass. Present only where `passed` is false. */
  reason?: string;
  /** How many issues the Open query listed. */
  listed: number;
  /** The reference read back, where the Open query listed one. */
  read?: string;
};

/** `POST /mods/promote`'s body. */
export type PromoteMod = {
  name: string;
  /** Absent is the first repository Fleet serves. */
  manifest_id?: string;
};

/** A mod committed by a Job of its own, on that Job's branch, and the pull request it opened. */
export type ModPromoted = {
  name: string;
  /** The Job made for this promote, which watches its pull request until it merges. */
  session_id: string;
  /** What the commit, the Job and the pull request are titled: `Add the calm theme mod`. */
  title: string;
  branch: string;
  commit: string;
  /** The pull request's address, where one was opened. */
  pull_request?: string;
  /** Why no pull request was opened, where none was. The commit stays on the Job's branch. */
  pull_request_not_opened?: string;
};
