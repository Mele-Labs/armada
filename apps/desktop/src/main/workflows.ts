// The Workflow creator's three routes: the list, one definition, and a save.
//
// `kit.ts`'s shape: no run and no file, so `port()` and the pick are what a
// route under the picked Manifest needs. **A save names the Manifest it is
// written under**, which is not always the pick's — the creator's scope select
// offers every Manifest — so it is named outright; Kit's is machine-wide and
// takes the pick's.

import type {
  ArmadaProposed,
  AskWorkflowChange,
  LeftOutWorkflow,
  NotADriftMarked,
  ProposeToArmada,
  SaveWorkflow,
  SendBackWorkflowChange,
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

import type {
  NotADriftAnswer,
  WorkflowChangeAnswer,
  WorkflowChangeApproveAnswer,
  WorkflowChangesRead,
  WorkflowCopyDeleteAnswer,
  WorkflowDefinitionRead,
  WorkflowProposeAnswer,
  WorkflowSaveAnswer,
  WorkflowStepsRead,
  WorkflowSupportBuildAnswer,
  WorkflowsRead,
} from "../shared/workflows";
import type { Picked } from "./picked";
import { ask, NOT_SET_UP } from "./request";

/**
 * `list_workflows`, `list_left_out_workflows`, `get_workflow` and `save_workflow`; and the Workflows
 * page's `get_workflow_steps`, `delete_workflow_copy`, `reset_workflow_copy` and `mark_not_a_drift`; and the
 * changes an agent drafts for him: `ask_workflow_change`, `suggest_workflow_change`, `list_workflow_changes`,
 * `approve_workflow_change`, `dismiss_workflow_change`, `send_back_workflow_change`, and
 * `propose_workflow_to_armada`.
 */
export class WorkflowCommands {
  private readonly port: () => number | null;
  private readonly picked: Picked;

  constructor(port: () => number | null, picked: Picked) {
    this.port = port;
    this.picked = picked;
  }

  /**
   * What Fleet runs for the picked repository, and what it left out. **On All it runs every repository's**
   * and leaves out nothing here: a left-out file is one repository's, and shows once that one is picked.
   */
  async list(): Promise<WorkflowsRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const all = this.picked.onAll;
    const runs = all ? "/workflows" : this.picked.manifest("/workflows");
    const left = all ? undefined : this.picked.manifest("/workflows/left_out");
    if (runs === null || left === null) return { ok: false, outcome: NOT_SET_UP };
    const [workflows, leftOut] = await Promise.all([ask(port, "GET", runs), left === undefined ? undefined : ask(port, "GET", left)]);
    if (workflows.ok !== true) return { ok: false, outcome: workflows.outcome };
    // A Fleet that cannot say what it left out has left nothing out that Bridge could name.
    return {
      ok: true,
      workflows: workflows.body as WorkflowSummary[],
      leftOut: leftOut?.ok === true ? (leftOut.body as LeftOutWorkflow[]) : [],
    };
  }

  /** One definition as its file holds it, from the place named, under the copy's own Manifest. */
  async definition(workflowId: string, source: string, manifestId: string | null): Promise<WorkflowDefinitionRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const route = `/workflows/definition?workflow_id=${encodeURIComponent(workflowId)}&source=${encodeURIComponent(source)}`;
    const path = this.manifestPath(route, manifestId);
    if (path === null) return { ok: false, outcome: NOT_SET_UP };
    const answer = await ask(port, "GET", path);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, definition: answer.body as WorkflowDefinition };
  }

  /** Write one. Fleet checks it with the loader's own rules and refuses in its own words. */
  async save(manifestId: string | null, body: SaveWorkflow): Promise<WorkflowSaveAnswer> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const path = this.manifestPath("/workflows/save", manifestId);
    if (path === null) return { ok: false, outcome: NOT_SET_UP };
    const answer = await ask(port, "POST", path, body);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, saved: answer.body as WorkflowSaved };
  }

  /** One definition step by step: the words, what ends each step and the nudges that still count. */
  async steps(workflowId: string, source: string, manifestId: string | null): Promise<WorkflowStepsRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const route = `/workflows/steps?workflow_id=${encodeURIComponent(workflowId)}&source=${encodeURIComponent(source)}`;
    const path = this.manifestPath(route, manifestId);
    if (path === null) return { ok: false, outcome: NOT_SET_UP };
    const answer = await ask(port, "GET", path);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, steps: answer.body as WorkflowSteps };
  }

  /** Delete a repository's or Kit's copy. Fleet refuses Armada's own in its own words. */
  async deleteCopy(manifestId: string | null, copy: WorkflowCopy): Promise<WorkflowCopyDeleteAnswer> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const path = this.manifestPath("/workflows/delete", manifestId);
    if (path === null) return { ok: false, outcome: NOT_SET_UP };
    const answer = await ask(port, "POST", path, copy);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, deleted: answer.body as WorkflowCopyDeleted };
  }

  /** Rewrite a copy with Armada's definition of the same id. It is a save, so it answers as one. */
  async resetCopy(manifestId: string | null, copy: WorkflowCopy): Promise<WorkflowSaveAnswer> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const path = this.manifestPath("/workflows/reset", manifestId);
    if (path === null) return { ok: false, outcome: NOT_SET_UP };
    const answer = await ask(port, "POST", path, copy);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, saved: answer.body as WorkflowSaved };
  }

  /** Mark one nudge not a drift. A nudge is the store's, so no Manifest is named. */
  async notADrift(nudge: number): Promise<NotADriftAnswer> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const answer = await ask(port, "POST", "/workflows/not_a_drift", { nudge });
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, marked: answer.body as NotADriftMarked };
  }

  /** Ask an agent to rewrite a copy as he words it. Answers the change drafting; nothing is saved. */
  async askChange(manifestId: string | null, body: AskWorkflowChange): Promise<WorkflowChangeAnswer> {
    return this.beginChange("/workflows/changes/ask", manifestId, body);
  }

  /** Ask for new words for one step now, from the nudges that count. */
  async suggestChange(manifestId: string | null, body: SuggestWorkflowChange): Promise<WorkflowChangeAnswer> {
    return this.beginChange("/workflows/changes/suggest", manifestId, body);
  }

  /** The open changes to one workflow in one place: drafting, ready or failed. */
  async changes(workflowId: string, source: string, manifestId: string | null): Promise<WorkflowChangesRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const route = `/workflows/changes?workflow_id=${encodeURIComponent(workflowId)}&source=${encodeURIComponent(source)}`;
    const path = this.manifestPath(route, manifestId);
    if (path === null) return { ok: false, outcome: NOT_SET_UP };
    const answer = await ask(port, "GET", path);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, changes: answer.body as WorkflowChanges };
  }

  /** Write a ready change through the save path. Fleet refuses it if the file changed since the draft began. */
  async approveChange(change: number): Promise<WorkflowChangeApproveAnswer> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const answer = await ask(port, "POST", "/workflows/changes/approve", { change });
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, approved: answer.body as WorkflowChangeApproved };
  }

  /** Put an open change aside. */
  async dismissChange(change: number): Promise<WorkflowChangeAnswer> {
    return this.actOnChange("/workflows/changes/dismiss", { change });
  }

  /** Send a ready or failed change back with his notes, to be drafted again. */
  async sendBackChange(back: SendBackWorkflowChange): Promise<WorkflowChangeAnswer> {
    return this.actOnChange("/workflows/changes/send_back", back);
  }

  /** Start a job that builds what a change Armada can't do yet lacks. Fleet closes the change. */
  async buildSupport(change: number): Promise<WorkflowSupportBuildAnswer> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const answer = await ask(port, "POST", "/workflows/changes/build", { change });
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, building: answer.body as WorkflowSupportBuilding };
  }

  /** Start a Job on Armada's own repository that lands a definition. */
  async proposeToArmada(proposing: ProposeToArmada): Promise<WorkflowProposeAnswer> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const answer = await ask(port, "POST", "/workflows/propose", proposing);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, proposed: answer.body as ArmadaProposed };
  }

  private async beginChange(route: string, manifestId: string | null, body: unknown): Promise<WorkflowChangeAnswer> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const path = this.manifestPath(route, manifestId);
    if (path === null) return { ok: false, outcome: NOT_SET_UP };
    const answer = await ask(port, "POST", path, body);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, change: answer.body as WorkflowChange };
  }

  private async actOnChange(route: string, body: unknown): Promise<WorkflowChangeAnswer> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const answer = await ask(port, "POST", route, body);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, change: answer.body as WorkflowChange };
  }

  /** The Manifest an act is under: the one named, or the pick's where none is named. */
  private manifestPath(route: string, manifestId: string | null): string | null {
    return manifestId === null ? this.picked.manifest(route) : this.picked.manifestNamed(route, manifestId);
  }
}
