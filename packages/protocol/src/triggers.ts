// Triggers: what a repository runs at a moment in a Job, saving one, and what a Job did with each.
// Hand-mirrored from `crates/ipc/src/triggers.rs`. Since protocol 23.58.

export type TriggerMoment = "step_starts" | "step_passes" | "pr_opened";

/** Where a copy was read from, least specific first. A machine's replaces a repository's whole. */
export type TriggerLevel = "armada" | "repository" | "machine";

/** Where a saved Trigger goes. */
export type TriggerScope = "repository" | "machine";

export type TriggerRuns = { kind: "command"; name: string } | { kind: "skill"; name: string };

export type TriggerSkipReason = "not_in_this_repo" | "skill_not_run";

/** Why a Trigger does not run. `said` is rendered and never matched on. */
export type TriggerSkip = { reason: TriggerSkipReason; name: string; said: string };

/** A copy a more specific level replaced: drawn struck through beside the one that runs. */
export type OverriddenTrigger = { level: TriggerLevel; file: string };

export type TriggerSummary = {
  name: string;
  when: TriggerMoment;
  /** Absent is every workflow. */
  workflow?: string;
  /** Absent is every step, and always on `pr_opened`. */
  step?: string;
  runs: TriggerRuns;
  /** Carried and not acted on yet. */
  block: boolean;
  /** Carried and not acted on yet. */
  repair: boolean;
  level: TriggerLevel;
  file: string;
  /** Absent where it runs in this repository. */
  skipped?: TriggerSkip;
  overrides?: OverriddenTrigger[];
};

/** A file Fleet runs without. `said` is the loader's sentence. */
export type LeftOutTrigger = { level: TriggerLevel; file: string; said: string };

/** `list_triggers`. */
export type TriggerList = { triggers: TriggerSummary[]; left_out: LeftOutTrigger[] };

/** `get_trigger`: the file's YAML text, in the shape `save_trigger` takes back. */
export type TriggerDefinition = {
  name: string;
  when: TriggerMoment;
  step?: string;
  level: TriggerLevel;
  file: string;
  definition: string;
  /** The level whose copy runs instead, where this one is replaced. */
  overridden_by?: TriggerLevel;
};

export type SaveTrigger = { scope: TriggerScope; definition: string; overwrite?: boolean };

export type TriggerSaved = {
  name: string;
  when: TriggerMoment;
  step?: string;
  scope: TriggerScope;
  file: string;
  replaced: boolean;
  /** Whose copy of this identity runs in this repository now. */
  runs_from?: TriggerLevel;
  /** A repository's file is read from the base branch, so it runs once it is on `main`. */
  waits_for_main?: boolean;
  /** Where it does not run here: a machine save can reach it, a repository save is refused. */
  skipped?: TriggerSkip;
};

export type RemoveTrigger = { scope: TriggerScope; when: TriggerMoment; step?: string; name: string };

export type TriggerRemoved = {
  scope: TriggerScope;
  file: string;
  runs_from?: TriggerLevel;
  waits_for_main?: boolean;
};

export type TriggerFiringState = "pending" | "skipped" | "running" | "passed" | "failed" | "awaiting_owner";

/** One of a Job's Triggers: frozen, and each firing of it. */
export type JobTrigger = {
  name: string;
  when: TriggerMoment;
  step: string;
  level: TriggerLevel;
  state: TriggerFiringState;
  skipped?: TriggerSkip;
  exit_code?: number;
  /** Absent while pending. */
  started_at?: string;
  ended_at?: string;
  /** The Job's log note for this firing: `get_job_log`'s whose `at` is this and whose `trigger` field is `name`. */
  log_at?: string;
};

/** `job.trigger_changed`: one row of the Job's Triggers, whole. */
export type JobTriggerChanged = { job_id: string; trigger: JobTrigger; at: string };
