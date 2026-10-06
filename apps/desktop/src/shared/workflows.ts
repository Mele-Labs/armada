// What the Workflow creator asks the host for, and what it is answered with.
// `main/workflows.ts` builds these from Fleet's three routes; nothing else does.

import type {
  LeftOutWorkflow,
  Outcome,
  SaveWorkflow,
  WorkflowDefinition,
  WorkflowSaved,
  WorkflowSummary,
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
