// The Workflow creator.
// A slice imports protocol and screens, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type {
  SavingWorkflow,
  WorkflowDefinitionRead,
  WorkflowSaveAnswer,
  WorkflowsRead,
} from "../workflows";

export type WorkflowsApi = {
  /**
   * The Workflow creator. The list is what Fleet runs for the picked repository and what it left
   * out; a definition is one file's text, from the place named; a save names the Manifest it is
   * written under, and Fleet's own refusal comes back as it said it.
   */
  readWorkflows: () => Promise<WorkflowsRead>;
  readWorkflowDefinition: (workflowId: string, source: string) => Promise<WorkflowDefinitionRead>;
  saveWorkflow: (saving: SavingWorkflow) => Promise<WorkflowSaveAnswer>;
};

export type WorkflowsState = Record<never, never>;

export const WORKFLOWS_NOTHING_YET: WorkflowsState = {};

export const WORKFLOWS_CHANNELS = {
  // The Workflow creator: the list, one definition, and a save. Fleet names the
  // repository for the first two; a save names the Manifest it is written under.
  readWorkflows: "bridge:read-workflows",
  readWorkflowDefinition: "bridge:read-workflow-definition",
  saveWorkflow: "bridge:save-workflow",
} as const;
