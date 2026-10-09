// A person's Bridge preferences, kept the way `limits.ts`'s three are kept —
// Fleet-wide, surviving a relaunch. `crates/ipc/src/preferences.rs`.
//
// The header rules in `protocol.ts` hold here: these are hand-written, and
// they drift the day a field moves.

/** Every preference Fleet knows, and what is in force for each — `GET
 * /preferences`. */
export type Preferences = {
  /** Whether Job detail's *Where things are* chapter opens collapsed
   * (`false`) or expanded (`true`). */
  where_things_are_open: boolean;
  /** Whether this machine offers a pull request as a draft unless the repository, the
   * workflow or the Job says otherwise. Since 23.68. Absent is `false`. */
  draft_pull_requests?: boolean;
  /**
   * The theme Bridge draws with: a built-in's id, a catalogue theme's, or a mod's name. Fleet holds
   * the word and Bridge decides what it names. Absent is `dark`.
   */
  theme?: string;
  /**
   * The owner's own layout choices, as the text of a `layout.json`: what Settings → Layout wrote, which outranks every
   * layout mod (`docs/concepts/layout-mods.md`). Absent is none made.
   */
  layout_choices?: string;
};

/**
 * A save — `POST /preferences/save`. **One preference, not the set** — unlike
 * `SaveLimits`, which sends every field it has an opinion on in one request.
 * `name` outside the closed set is refused by name, `fleet.unknown_preference`.
 */
export type SavePreference = {
  name: string;
  value: boolean;
  /**
   * The value of a preference that is text and not a switch, which is `theme` and `layout_choices`.
   * `value` is read for every other name and this is read for none of them. An empty `layout_choices`
   * takes the owner's choices back; text that is not a `layout.json` is a 422 `fleet.unacceptable_layout`.
   */
  text?: string;
};
