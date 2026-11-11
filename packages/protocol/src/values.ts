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
