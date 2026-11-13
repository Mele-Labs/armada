// Moving running Jobs to a workflow's new version: field for field with `crates/ipc/src/workflow_migrating.rs`. Protocol 23.97,
// `docs/practices/protocol/protocol-23-97-moving-running-jobs-to-a-new-workflow-version.md`.
//
// A publish moves no Job: it pins each live Job it would touch to the text that Job was running (`WorkflowPublished.pinned`), and the person
// picks which Jobs move. `GET /workflows/affected` lists them and `POST /workflows/migrate` moves the ones he picks.

/** One live Job that follows the workflow, and where it stands against the text a publish takes. */
export type AffectedJob = {
  session_id: string;
  title?: string;
  manifest_id: string;
  /** The step the rail stands on now, as the Job reads its own steps. Absent where it stands on none. */
  step_id?: string;
  step_label?: string;
  /** The step's id is one of the target's steps, so a move keeps the Job's place. True also where the Job stands on no step. */
  survives: boolean;
  /** A publish pinned the Job to the text it was running. */
  pinned: boolean;
  /** The Job edited the steps of its own copy. Listed so he sees it; a move refuses it. */
  edited: boolean;
  /** The version in the place's log whose hash is the Job's copy text. Absent where none matches and where the Job holds no copy. */
  on_version?: number;
};

/**
 * `GET /workflows/affected?workflow_id=&source=&project=&manifest_id=`: every live hosted Job that follows `workflow_id` in a repository the place
 * reaches, against the draft's text where a draft is kept for the place, else the published text. `source` is `kit`, `repository` or `project`.
 * 422 `fleet.workflow_project_unnamed`, 404 `fleet.workflow_project_unknown` for a project's; 422 `fleet.no_such_workflow_definition` for any other source.
 */
export type AffectedJobs = {
  jobs: AffectedJob[];
};

/** `POST /workflows/migrate?manifest_id=`'s body: move the Jobs named to the published workflow. `project` is required where `source` is `project`. */
export type MigrateJobs = {
  workflow_id: string;
  source: string;
  project?: string;
  session_ids: string[];
};

/** What `POST /workflows/migrate` did for one Job. */
export type MigrateResult = {
  session_id: string;
  /** The Job was moved. False for a refusal and for a Job that already reads the published text. */
  moved: boolean;
  /** Where the Job stands now, where it moved. */
  step_id?: string;
  step_label?: string;
  /**
   * Why a Job did not move: `fleet.workflow_migrate_not_live`, `fleet.workflow_migrate_not_following` or `fleet.workflow_migrate_edited`.
   * Absent on a move, and on a Job that holds no copy and so already reads the published workflow (`moved` false, with a `message`).
   */
  code?: string;
  message?: string;
};

/** `POST /workflows/migrate`'s answer: one result for each id asked, in order. The call is 200 whatever the results are. */
export type MigratedJobs = {
  results: MigrateResult[];
};
