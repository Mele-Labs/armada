// The Workflow creator's three routes: the list, one definition, and a save.
//
// `kit.ts`'s shape: no run and no file, so `port()` and the pick are what a
// route under the picked Manifest needs. **A save names the Manifest it is
// written under**, which is not always the pick's — the creator's scope select
// offers every Manifest — so it is named outright; Kit's is machine-wide and
// takes the pick's.

import type { LeftOutWorkflow, SaveWorkflow, WorkflowDefinition, WorkflowSaved, WorkflowSummary } from "@armada/protocol";

import type { WorkflowDefinitionRead, WorkflowSaveAnswer, WorkflowsRead } from "../shared/workflows";
import type { Picked } from "./picked";
import { ask, NOT_SET_UP } from "./request";

/** `list_workflows`, `list_left_out_workflows`, `get_workflow` and `save_workflow`. */
export class WorkflowCommands {
  private readonly port: () => number | null;
  private readonly picked: Picked;

  constructor(port: () => number | null, picked: Picked) {
    this.port = port;
    this.picked = picked;
  }

  /** What Fleet runs for the picked repository, and what it left out. */
  async list(): Promise<WorkflowsRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const runs = this.picked.manifest("/workflows");
    const left = this.picked.manifest("/workflows/left_out");
    if (runs === null || left === null) return { ok: false, outcome: NOT_SET_UP };
    const [workflows, leftOut] = await Promise.all([ask(port, "GET", runs), ask(port, "GET", left)]);
    if (workflows.ok !== true) return { ok: false, outcome: workflows.outcome };
    // A Fleet that cannot say what it left out has left nothing out that Bridge could name.
    return {
      ok: true,
      workflows: workflows.body as WorkflowSummary[],
      leftOut: leftOut.ok === true ? (leftOut.body as LeftOutWorkflow[]) : [],
    };
  }

  /** One definition as its file holds it, from the place named. */
  async definition(workflowId: string, source: string): Promise<WorkflowDefinitionRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const route = `/workflows/definition?workflow_id=${encodeURIComponent(workflowId)}&source=${encodeURIComponent(source)}`;
    const path = this.picked.manifest(route);
    if (path === null) return { ok: false, outcome: NOT_SET_UP };
    const answer = await ask(port, "GET", path);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, definition: answer.body as WorkflowDefinition };
  }

  /** Write one. Fleet checks it with the loader's own rules and refuses in its own words. */
  async save(manifestId: string | null, body: SaveWorkflow): Promise<WorkflowSaveAnswer> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const path = manifestId === null ? this.picked.manifest("/workflows/save") : this.picked.manifestNamed("/workflows/save", manifestId);
    if (path === null) return { ok: false, outcome: NOT_SET_UP };
    const answer = await ask(port, "POST", path, body);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, saved: answer.body as WorkflowSaved };
  }
}
