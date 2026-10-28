// What the Workflow creator asks the host for, and what it is answered with.
// `main/workflows.ts` builds these from Fleet's three routes; nothing else does.

import type {
  ArmadaProposed,
  AskWorkflowChange,
  LeftOutWorkflow,
  NotADriftMarked,
  Outcome,
  SaveWorkflow,
  SuggestWorkflowChange,
  WorkflowChange,
  WorkflowChangeApproved,
  WorkflowChanges,
  WorkflowCopy,
  WorkflowCopyDeleted,
  WorkflowDefinition,
  WorkflowSaved,
  WorkflowSteps,
  WorkflowSummary,
  WorkflowSupportBuilding,
} from "@armada/protocol";

/** `GET /workflows` and `GET /workflows/left_out`, for the picked repository, read together. */
export type WorkflowsRead =
  | { ok: true; workflows: WorkflowSummary[]; leftOut: LeftOutWorkflow[] }
  | { ok: false; outcome: Outcome };

/** `GET /workflows/definition`: one file's text. A left-out file is not read here; the list names it. */
export type WorkflowDefinitionRead = { ok: true; definition: WorkflowDefinition } | { ok: false; outcome: Outcome };

/** `POST /workflows/save`. A refusal carries Fleet's own sentence on `outcome`. */
export type WorkflowSaveAnswer = { ok: true; saved: WorkflowSaved } | { ok: false; outcome: Outcome };

/** One save: the body, and the Manifest it is written under, which Kit leaves to the picked repository's. */
export type SavingWorkflow = { manifestId: string | null; body: SaveWorkflow };

/** `GET /workflows/steps`: one definition step by step, with the nudges that still count. */
export type WorkflowStepsRead = { ok: true; steps: WorkflowSteps } | { ok: false; outcome: Outcome };

/** One copy to delete or reset, and the Manifest it is acted under, which Kit's leaves to the picked repository's. */
export type ActingOnWorkflowCopy = { manifestId: string | null; copy: WorkflowCopy };

/** `POST /workflows/delete`. A refusal carries Fleet's own sentence on `outcome`. */
export type WorkflowCopyDeleteAnswer = { ok: true; deleted: WorkflowCopyDeleted } | { ok: false; outcome: Outcome };

/** `POST /workflows/not_a_drift`. */
export type NotADriftAnswer = { ok: true; marked: NotADriftMarked } | { ok: false; outcome: Outcome };

/** The Manifest a change is begun under, which Kit's leaves to the picked repository's, and what is asked. */
export type AskingForWorkflowChange = { manifestId: string | null; body: AskWorkflowChange };

/** The Manifest a suggestion is begun under, and the step. */
export type SuggestingWorkflowChange = { manifestId: string | null; body: SuggestWorkflowChange };

/** `POST /workflows/changes/ask`, `.../suggest`, `.../send_back` and `.../dismiss`: the change. A refusal carries Fleet's own sentence. */
export type WorkflowChangeAnswer = { ok: true; change: WorkflowChange } | { ok: false; outcome: Outcome };

/** `GET /workflows/changes`: the open changes to one workflow in one place. */
export type WorkflowChangesRead = { ok: true; changes: WorkflowChanges } | { ok: false; outcome: Outcome };

/** `POST /workflows/changes/approve`: the change and the save it made. A refusal says the file changed, or why it does not fit. */
export type WorkflowChangeApproveAnswer = { ok: true; approved: WorkflowChangeApproved } | { ok: false; outcome: Outcome };

/** `POST /workflows/changes/build`: the job started to build what a change Armada can't do yet lacks. */
export type WorkflowSupportBuildAnswer = { ok: true; building: WorkflowSupportBuilding } | { ok: false; outcome: Outcome };

/** `POST /workflows/propose`: the Job started on Armada's repository. */
export type WorkflowProposeAnswer = { ok: true; proposed: ArmadaProposed } | { ok: false; outcome: Outcome };
