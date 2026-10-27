// The Workflow creator.
// A slice imports protocol and screens, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type { ProposeToArmada, SendBackWorkflowChange } from "@armada/protocol";

import type {
  ActingOnWorkflowCopy,
  AskingForWorkflowChange,
  NotADriftAnswer,
  SavingWorkflow,
  SuggestingWorkflowChange,
  WorkflowChangeAnswer,
  WorkflowChangeApproveAnswer,
  WorkflowChangesRead,
  WorkflowCopyDeleteAnswer,
  WorkflowDefinitionRead,
  WorkflowProposeAnswer,
  WorkflowSaveAnswer,
  WorkflowStepsRead,
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
  /**
   * The Workflows page beside a definition's text: its steps with their words, what ends each and
   * the nudges that still count; a copy deleted, or reset to Armada's (a save, so it answers as one);
   * and one nudge marked not a drift. Making my copy of Armada's is `readWorkflowDefinition` with
   * `armada`, then `saveWorkflow` into Kit without `overwrite`.
   */
  readWorkflowSteps: (workflowId: string, source: string) => Promise<WorkflowStepsRead>;
  deleteWorkflowCopy: (acting: ActingOnWorkflowCopy) => Promise<WorkflowCopyDeleteAnswer>;
  resetWorkflowCopy: (acting: ActingOnWorkflowCopy) => Promise<WorkflowSaveAnswer>;
  markNotADrift: (nudge: number) => Promise<NotADriftAnswer>;
  /**
   * An agent drafts a change for him to approve; nothing saves until `approveWorkflowChange`. Asking and
   * suggesting answer the change `drafting`; `readWorkflowChanges` is how the page sees it become `ready`
   * or `failed`, polled while one is drafting. Send back drafts again with his notes.
   */
  askWorkflowChange: (asking: AskingForWorkflowChange) => Promise<WorkflowChangeAnswer>;
  suggestWorkflowChange: (suggesting: SuggestingWorkflowChange) => Promise<WorkflowChangeAnswer>;
  readWorkflowChanges: (workflowId: string, source: string) => Promise<WorkflowChangesRead>;
  approveWorkflowChange: (change: number) => Promise<WorkflowChangeApproveAnswer>;
  dismissWorkflowChange: (change: number) => Promise<WorkflowChangeAnswer>;
  sendBackWorkflowChange: (back: SendBackWorkflowChange) => Promise<WorkflowChangeAnswer>;
  /** A Job on Armada's own repository that lands a definition, so it ships to every install. */
  proposeWorkflowToArmada: (proposing: ProposeToArmada) => Promise<WorkflowProposeAnswer>;
};

export type WorkflowsState = Record<never, never>;

export const WORKFLOWS_NOTHING_YET: WorkflowsState = {};

export const WORKFLOWS_CHANNELS = {
  // The Workflow creator: the list, one definition, and a save. Fleet names the
  // repository for the first two; a save names the Manifest it is written under.
  readWorkflows: "bridge:read-workflows",
  readWorkflowDefinition: "bridge:read-workflow-definition",
  saveWorkflow: "bridge:save-workflow",
  readWorkflowSteps: "bridge:read-workflow-steps",
  deleteWorkflowCopy: "bridge:delete-workflow-copy",
  resetWorkflowCopy: "bridge:reset-workflow-copy",
  markNotADrift: "bridge:mark-not-a-drift",
  askWorkflowChange: "bridge:ask-workflow-change",
  suggestWorkflowChange: "bridge:suggest-workflow-change",
  readWorkflowChanges: "bridge:read-workflow-changes",
  approveWorkflowChange: "bridge:approve-workflow-change",
  dismissWorkflowChange: "bridge:dismiss-workflow-change",
  sendBackWorkflowChange: "bridge:send-back-workflow-change",
  proposeWorkflowToArmada: "bridge:propose-workflow-to-armada",
} as const;
