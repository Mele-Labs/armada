// What a person decides about a Job before it runs, and how it reads back.
// `crates/ipc/src/approving.rs`. Since 23.8, spike 022 slice 4.
//
// The header rules in `protocol.ts` hold: hand-written, and every closed set
// left as `string`.

import type { TierModels } from "./work-plan";

/**
 * `approve_dispatch`'s body: the proposal as the person left it at the press
 * (#1641). **A field left out is the proposal as it stands**, except
 * `drone_cap`, which is the machine's cap where left out. No body at all
 * approves it as it stands.
 */
export type ApproveDispatch = {
  title?: string;
  /** The request, rewritten. `JobDetail.facts`'s name. */
  facts?: string;
  /** A workflow this Job's repository holds. Its steps replace the gates. */
  workflow_id?: string;
  /** One per step a person set; a step left out keeps its declared gate. */
  gates?: GateChoice[];
  criteria?: CriterionWritten[];
  /** Left out is the map as it stands; `{}` is Armada picking every tier. */
  tiers?: TierModels;
  /** Kept from 23.8, enforced from slice 5. */
  drone_cap?: number;
  landing?: LandingChoice;
  /** One per step a person tuned, `gates`' shape; a step left out runs as declared. Since 23.20. */
  tuning?: StepTuning[];
};

/**
 * What a person tuned on one step at the press, frozen with the Job and read
 * where the step runs. **A field left out is the step as declared.** An
 * unknown field is refused, so a `harness` is not silently dropped. Since 23.20.
 */
export type StepTuning = {
  step_id: string;
  /** Refused unless `list_models` offers it. */
  model?: string;
  /** Left out is Armada picking. */
  effort?: Effort;
  /** Words handed to the step's Drone beside its brief. Blank is none. */
  context?: string;
  /** The harness, in its own name. Refused unless `ModelChoices.harnesses` lists it. Since 23.23. */
  harness?: string;
  /** Every judge check's `panel_size`. Refused at zero and on a step with no Judge. */
  judges?: number;
  /** Manifest Checks the step declares that this Job does not run, by name. */
  checks_off?: string[];
};

/** How hard a step's Drone thinks. Since 23.20. */
export type Effort = "low" | "medium" | "high";

/** `edit_job`'s body (#1699's route): the fields a person changed, saved without releasing. */
export type EditJob = {
  title?: string;
  facts?: string;
  criteria?: CriterionWritten[];
};

/** What one step is gated by, as the approval sets it. */
export type GateChoice = {
  step_id: string;
  checks: boolean;
  judge: boolean;
  you: boolean;
  /** Overrides the repository's rule for this Job, for the life of the Job. */
  overridden?: boolean;
};

/** One criterion as a person leaves it. An absent id is a new line, and Fleet mints one. */
export type CriterionWritten = {
  criterion_id?: string;
  text: string;
  /** `check`, `judge` or `attested`. A new line is `judge` or `attested`. */
  source: string;
};

/** How the work lands, as the approval sets it. `land_together` does not cross. */
export type LandingChoice = {
  /** Left out is the Manifest's base. Refused unless the repository holds it. */
  target?: string;
  /**
   * Left out is `target`'s reading. Refused unless the repository holds it, or
   * `start_point` says where Fleet makes it.
   */
  from_ref?: string;
  /**
   * A branch the repository holds, where Fleet makes `from_ref` when the
   * repository holds no `from_ref` yet. Read only then; `target` may name the
   * new branch too. Since 23.21.
   */
  start_point?: string;
  /** `job` or `group`; only `job` is run, and `group` is refused. */
  branching?: string;
  /** `ready` or `draft`. Left out is `ready`. */
  pr_mode?: string;
  /** Only `delivered` is run; the other three are refused. */
  complete_when?: string;
  /**
   * Stop at the branch: commit on the Job's own branch, with no pull request,
   * merge or push, and keep it. `pr_mode` is ignored while it holds. Since 23.24.
   */
  local?: boolean;
  /**
   * Turn on the forge's auto-merge for the pull request when it opens. Refused with
   * `local`. Since 23.24.
   */
  auto_merge?: boolean;
};

/**
 * `set_landing_target`'s body, `POST /jobs/{job_id}/set_landing_target`: the
 * branch an approved Job landing in the base lands in instead, once, before its
 * work goes out. Since 23.22.
 */
export type SetLandingTarget = {
  target: string;
};

/** How one Job lands, frozen at approval, on `JobDetail.landing`. */
export type LandingRule = {
  /** Absent is the Manifest's base, which is not a branch name to print. */
  target?: string;
  from_ref?: string;
  /** `ready` or `draft`. */
  pr_mode: string;
  /**
   * What finishes the Job: `delivered` or `all_members_landed`. Since 23.13;
   * absent from a Fleet before it, whose Jobs finished delivered.
   */
  complete_when?: string;
  /** The work stops at the Job's branch: no pull request. Absent is false. Since 23.24. */
  local?: boolean;
  /** The pull request is set to merge itself on the forge. Absent is false. Since 23.24. */
  auto_merge?: boolean;
};

/**
 * What Approve the plan sends to `approve_wave` (#1694) at an Epic Job's plan
 * gate: every Job of the proposed wave, each at `awaiting_approval` and
 * dispatched by the Epic, released together. Since 23.13. **Refused unless it
 * names exactly the wave Fleet holds**, so a person releases what they read.
 */
export type ApproveWave = {
  jobs: readonly string[];
};

/** What this Job's approval said in place of the repository, in `armada.yml`'s words. */
export type PolicyOverrides = {
  auto_merge?: string;
  review_gate?: string;
};

/**
 * Where a criterion's words came from (#1642). `kind` is `issue`, `prompt` or
 * `person`; an `issue` carries `ref` and `url`, the forge's own address and
 * the only one Bridge's main process opens.
 */
export type CriterionOrigin = {
  kind: string;
  ref?: string;
  url?: string;
};

/** What a person set while typing the request, on `JobRequest.settings`. */
export type DispatchSettings = {
  workflow_id?: string;
  tiers?: TierModels;
  drone_cap?: number;
  /** `auto` or `you_at_review`. */
  lands?: string;
};

/** `list_branches`' answer (#1605): the repository's local branches, the base first. */
export type Branches = {
  branches: BranchRow[];
};

/** One branch a Job may start from or land in. */
export type BranchRow = {
  name: string;
  /** The branch a worktree is cut from where nobody names another. One at most. */
  base: boolean;
};
