// The values a person sets for how work behaves, and what each comes to once the layers are laid over each
// other. `crates/ipc/src/values.rs`, `docs/practices/protocol/projects.md`, *Values*.

/**
 * What one layer sets, a project's or Kit's: only the keys it sets, each absent where it says nothing.
 * `auto_merge` is `never`, `checks-pass` or `always`, and a word that is not one is refused 422 `fleet.project_values_refused`.
 */
export type ValuesSet = {
  pr_mode?: "ready" | "draft";
  auto_merge?: string;
  /** The model an agent run that reviews work runs on, as the agent spells it. */
  review_model?: string;
  look?: LookSet;
  /**
   * The tracker a project names for its issues and for the Jobs in it, as the Manifest's `tracker:` block writes it:
   * `provider` and every other key as text, carried as written. A project's only: Kit has none, and a tracker in Kit's
   * values is refused 422 `fleet.project_values_refused`.
   */
  tracker?: Record<string, string>;
  /**
   * How the Operator reviews work before the person sees it. A layer that sets it sets all three, and a kind it
   * leaves out is `off`.
   */
  reviews?: ReviewsSet;
  /**
   * Whether a Job that a Job or the Operator starts begins at once (`at_once`) or waits on the person's start card
   * (`ask_me`). A word that is not one is refused 422 `fleet.project_values_refused`. A Job the person starts
   * himself is never held by it.
   */
  agent_starts?: string;
  /**
   * How a Job in the project lands its work: `auto_land`, `open_pr`, `keep_local` or `ask`. A word that is not one is
   * refused 422 `fleet.project_values_refused`. Unset at every layer, the first answer to the delivery question is kept here.
   */
  delivery?: string;
};

/** `off`, `review` or `review_and_approve`; for `walks` only `off` or `review`. A word that is not one is refused 422 `fleet.project_values_refused`. */
export type ReviewsSet = {
  plans?: string;
  pull_requests?: string;
  walks?: string;
};

/** How Bridge looks. Fleet keeps the words and Bridge decides what each names. `density` is `compact`, `default` or `roomy`. */
export type LookSet = {
  layout?: string;
  theme?: string;
  density?: string;
};

/**
 * The layer a value came from, from the furthest to the nearest. The nearest that sets a key wins:
 * default, Machine, Kit, repository, Project, then the Job's own.
 */
export type ValueLayer = "default" | "machine" | "kit" | "repository" | "project" | "job";

/** One key's answer. `value` is absent where no layer sets the key and the default has no word for it; `from` is then `default`. */
export type ResolvedValue = {
  value?: string;
  from: ValueLayer;
};

/** The tracker a layer named. `value` is absent where none does (the forge's own), and `from` is then `default`. */
export type ResolvedTracker = {
  value?: Record<string, string>;
  from: ValueLayer;
};

/** How each kind of work is reviewed, and the layer that said so. A kind no layer set is `off`, and `from` is then `default`. */
export type ResolvedReviews = {
  /** `off`, `review` or `review_and_approve`. */
  plans: string;
  /** `off`, `review` or `review_and_approve`. */
  pull_requests: string;
  /** `off` or `review`. */
  walks: string;
  from: ValueLayer;
};

/** What autonomy comes to for one kind of act. `from` is `kit`, `project` or `repository` where a piece sets it, else `default`. */
export type ResolvedAutonomy = {
  /** `tracker_write`, `forge_write`, `draft`, `start_job`, `start_watch`, `tell_agent`, `run_command` or `run_agent`. */
  kind: string;
  /** `acts`, `asks` or `off`. */
  value: string;
  from: ValueLayer;
};

/** `GET /projects/values?project=&manifest_id=`, and what a Job froze at its start (`hosted.values`). */
export type ResolvedValues = {
  /** `ready` or `draft`. */
  pr_mode: ResolvedValue;
  /** `never`, `checks-pass` or `always`. */
  auto_merge: ResolvedValue;
  review_model: ResolvedValue;
  layout: ResolvedValue;
  theme: ResolvedValue;
  /** `compact`, `default` or `roomy`. */
  density: ResolvedValue;
  /**
   * The tracker for the repository asked about, a project's replacing its repository's. Absent from what a Job froze:
   * a Job asks its tracker where it acts, and the project's word is read then.
   */
  tracker?: ResolvedTracker;
  /** How the Operator reviews work before the person sees it. A Job is reviewed as its start said, whatever the project says since. */
  reviews?: ResolvedReviews;
  /** `at_once` or `ask_me`. Absent from what a Job froze before the key existed. */
  agent_starts?: ResolvedValue;
  /** `auto_land`, `open_pr`, `keep_local` or `ask`; the value is empty where no layer sets it. Absent from what a Job froze before the key existed. */
  delivery?: ResolvedValue;
  /** One for each kind of act, where a repository was asked about; left out where only a project was. */
  autonomy?: ResolvedAutonomy[];
};

/** `POST /kit/values`: Kit's values, whole. A key left out is taken away, and `{}` takes every value back. */
export type SaveKitValues = {
  values: ValuesSet;
};

/** What `save_kit_values` answers: where the file was written and what Kit now sets. */
export type KitValuesSaved = {
  file: string;
  values: ValuesSet;
};
