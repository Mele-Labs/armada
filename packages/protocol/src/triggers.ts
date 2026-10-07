// Triggers: what a repository runs at a moment in a Job, saving one, and what a Job did with each.
// Hand-mirrored from `crates/ipc/src/triggers.rs`. Since protocol 23.58.

import type { KeptFrom } from "./added-steps";

export type TriggerMoment = "step_starts" | "step_passes" | "pr_opened";

/** Where a copy was read from, least specific first. A machine's replaces a repository's whole. */
export type TriggerLevel = "armada" | "repository" | "machine";

/** Where a saved Trigger goes. */
export type TriggerScope = "repository" | "machine";

export type TriggerRuns = { kind: "command"; name: string } | { kind: "skill"; name: string };

/** `by_owner`: it failed and held the Job, and the owner skipped it. Since 23.68. */
export type TriggerSkipReason = "not_in_this_repo" | "skill_not_run" | "by_owner";

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

export type SaveTrigger = {
  scope: TriggerScope;
  definition: string;
  overwrite?: boolean;
  /** Keeps a Job's added step for every Job: the definition is the Trigger drawn from it. Since 23.68. */
  kept_from?: KeptFrom;
};

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

export type TriggerFiringState =
  | "pending"
  | "skipped"
  | "running"
  | "passed"
  | "failed"
  | "awaiting_owner"
  /** Failed with `repair` on, and a repair Drone is working on a branch of its own. Since 23.68. */
  | "repairing"
  /** The Command is running again, on the repair branch or on the Job's. Since 23.68. */
  | "rerunning"
  /** The repair branch passes. Held for the owner's `choose_trigger_fix`. Since 23.68. */
  | "fix_ready"
  /** Failed with `block` on, and the Job waits on it: `rerun_trigger` or `skip_trigger` lets it go. Since 23.68. */
  | "held";

/** Where a held fix goes: onto the Job's branch, or into a pull request of its own. */
export type TriggerFixChoice = "this_branch" | "new_pr";

/** A pull request a repair opened. `number` is read off the end of `url` and absent where it has none. */
export type TriggerPullRequest = { url: string; number?: number };

/** What a failed Trigger's repair has come to. Present from the first repair Drone on. Since 23.68. */
export type TriggerRepair = {
  /** Which try this is, 1 or 2. Never drawn as a count. */
  attempt: number;
  /** The branch the repair Drone writes on, cut from the Job's. */
  branch?: string;
  /** The files the fix changes, repository-relative. Empty until the fix is held. */
  files?: string[];
  /** What the owner chose. A `this_branch` kept until the Job's Drone is done shows here with `fix_ready` still the state. */
  choice?: TriggerFixChoice;
  /** For `new_pr`, once it is open. */
  pull_request?: TriggerPullRequest;
};

/** `choose_trigger_fix`'s body: the Trigger's name, and where its held fix goes. */
export type ChooseTriggerFix = { trigger: string; choice: TriggerFixChoice };

/** What the act came to: where the firing stands now. `fix_ready` is a choice kept until it can be placed. */
export type TriggerFixChosen = { state: TriggerFiringState; pull_request?: TriggerPullRequest };

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
  /** Absent until a repair Drone has been put on it, and where `repair` is off. Since 23.68. */
  repair?: TriggerRepair;
  /** The Trigger blocks, so a failure holds the Job. Absent where it does not. Since 23.68. */
  blocks?: boolean;
};

/** `job.trigger_changed`: one row of the Job's Triggers, whole. */
export type JobTriggerChanged = { job_id: string; trigger: JobTrigger; at: string };
