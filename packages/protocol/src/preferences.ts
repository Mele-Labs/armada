// A person's Bridge preferences, kept the way `limits.ts`'s three are kept —
// Fleet-wide, surviving a relaunch. `crates/ipc/src/preferences.rs`.
//
// The header rules in `protocol.ts` hold here: these are hand-written, and
// they drift the day a field moves.

/** Every preference Fleet knows, and what is in force for each — `GET
 * /preferences`. */
/** What he takes a Job's work in. */
export type TakeIn = "read" | "look" | "listen";
/** How he answers a Job's question. */
export type AnswerWith = "type" | "talk" | "buttons";
/** Where a Job's result goes when he reads. */
export type ReadResults = "note_then_page" | "always_page" | "thread_only";
/** How a Job's result shows when he looks. */
export type LookResults = "picture_every" | "picture_screens" | "walk_visible";
/** How a Job's question offers its options when he looks. */
export type LookQuestions = "pictures_mix" | "pictures_whole";

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
  /**
   * The owner's own key bindings, as the JSON Settings → Keyboard wrote: an act's id to the keys it
   * answers in place of the registry's (`docs/concepts/key-bindings.md`). Absent is none changed.
   */
  key_bindings?: string;
  /**
   * How the owner has Bridge draw a time, as the JSON Settings, Time wrote: `{ hour12, seconds, date }`, each true
   * or false. Absent is 24-hour clock with seconds, no date.
   */
  time_format?: string;
  /** What he takes a Job's work in. Absent is `read`. */
  take_in?: TakeIn;
  /** How he answers a Job's question. Absent is `type`. */
  answer_with?: AnswerWith;
  /** Whether he has been asked how he works with Jobs. Absent is `false`. */
  first_run_answered?: boolean;
  /** Where a Job's result goes when he reads. Absent is `note_then_page`. */
  read_results?: ReadResults;
  /** How a Job's result shows when he looks. Absent is `picture_every`. */
  look_results?: LookResults;
  /** How a Job's question offers its options when he looks. Absent is `pictures_mix`. */
  look_questions?: LookQuestions;
  /** Whether a Job that ends is read out in a summary when he listens. Absent is `true`. */
  listen_end_summary?: boolean;
  /** Whether a Job that needs him is read out when he listens. Absent is `true`. */
  listen_needs_you?: boolean;
  /** Whether a Job's progress is read out when he listens. Absent is `false`. */
  listen_progress?: boolean;
  /** Whether the Operator's words are read out when he listens. Absent is `false`. */
  listen_operator?: boolean;
  /** The voice he listens with, by the name the Mac gives it. Absent is the Mac's default voice. */
  listen_voice?: string;
  /** Whether a text box shows under a question's options. Absent is `true`. */
  buttons_words?: boolean;
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
   * The value of a preference that is text and not a switch, which is `theme`, `layout_choices`, `key_bindings`, `time_format`,
   * `listen_voice` and the closed sets `take_in`, `answer_with`, `read_results`, `look_results` and `look_questions` (one word of
   * the set; any other is a 422 `fleet.unacceptable_choice` naming the preference and the word, with nothing stored).
   * `value` is read for every other name and this is read for none of them. An empty `layout_choices`
   * takes the owner's choices back; text that is not a `layout.json` is a 422 `fleet.unacceptable_layout`.
   * An empty `time_format` takes the choice back; text that is not the three switches is a 422
   * `fleet.unacceptable_time_format`.
   */
  text?: string;
};
