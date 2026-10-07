// The Workflow creator's three routes on the mock Fleet: the list, one
// definition, and a save, answered from the fixture in `@armada/components`.
//
// **The same shapes Fleet sends**, so the surface that reads them is the one the
// app draws: a definition is the file's text, a refusal is a `WireError` with
// Fleet's own code, and a save over an id that is already there is refused until
// it says `overwrite`. Two of Fleet's refusals are served, the two a person can
// reach from the creator itself.

import { leftOutRowsOf, MOCK_FILES, workflowRowsOf, type MockFile } from "@armada/components";
import type { Outcome, RepositorySummary, WorkflowSummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/build/base";

import type { BridgeApi } from "../../../shared/api";
import { manifesting } from "./manifest-fake";
import { connected, type Scenario } from "./moment";

type Served = Pick<BridgeApi, "readWorkflows" | "readWorkflowDefinition" | "saveWorkflow">;

const refusal = (code: string, message: string): Outcome => ({
  ok: false,
  why: "refused",
  error: { code, message, run_id: "mock", fields: {}, chain: [] },
});

const KIT_FOLDER = "~/.armada/workflows";
const REPOSITORY_FOLDER = ".armada/workflows";

/** A fresh Fleet's workflows. `current` is the Manifest the window is in, which a repository save is listed under. */
export function workflowsServed(current: () => string | null): Served {
  let files: MockFile[] = [...MOCK_FILES];
  return {
    readWorkflows: async () => ({
      ok: true,
      workflows: workflowRowsOf(files).map(
        (one): WorkflowSummary => ({ name: one.id, version: 1, steps: [], manifest_id: current() ?? "", ...one }),
      ),
      leftOut: leftOutRowsOf(files),
    }),
    readWorkflowDefinition: async (workflowId, source) => {
      const held = files.find((one) => one.id === workflowId && one.source === source && one.leftOut === undefined);
      return held === undefined
        ? { ok: false, outcome: refusal("fleet.no_such_workflow_definition", `No ${source} definition of \`${workflowId}\` is held`) }
        : { ok: true, definition: { workflow_id: held.id, source: held.source, file: held.file, definition: held.text } };
    },
    saveWorkflow: async ({ manifestId, body }) => {
      const id = String((JSON.parse(body.definition) as { workflow_id?: unknown }).workflow_id ?? "");
      if (!/^[A-Za-z0-9_-]+$/.test(id)) {
        return {
          ok: false,
          outcome: refusal("fleet.workflow_id_not_a_name", `\`${id}\` cannot be a file's name: a workflow_id here is letters, digits, \`-\` and \`_\``),
        };
      }
      const source = body.scope === "kit" ? "kit" : "repository";
      const file = `${source === "kit" ? KIT_FOLDER : REPOSITORY_FOLDER}/${id}.json`;
      const held = files.find((one) => one.source === source && one.id === id && one.leftOut === undefined);
      // Another Manifest's folder is not the one the list is read from.
      const here = source === "kit" || manifestId === current();
      if (here && held !== undefined && body.overwrite !== true) {
        return {
          ok: false,
          outcome: refusal("fleet.workflow_exists", `${held.file} already defines \`${id}\` and nothing was written; save again with overwrite to replace it`),
        };
      }
      if (here) files = [...files.filter((one) => one !== held), { source, id, file, text: body.definition }];
      return { ok: true, saved: { workflow_id: id, scope: body.scope, file, replaced: held !== undefined, runs_from: source } };
    },
  };
}

const LEDGER: RepositorySummary = {
  root: "/Users/user/ledger",
  records_root: "/Users/user/Library/Application Support/Armada/records/ledger",
  manifest: { ...repository().manifest!, id: "01M1CNPKTV0018H2M1CXDNBK07", repository: "ledger", path: "ledger/armada.yml" },
};
const SITE: RepositorySummary = {
  root: "/Users/user/site",
  records_root: "/Users/user/Library/Application Support/Armada/records/site",
  manifest: { ...repository().manifest!, id: "01M1CNPKTV0018H2M1CXDNBK08", repository: "site", path: "site/armada.yml" },
};

/**
 * Three repositories with a Manifest each, the first picked, and Fleet saying it
 * left a workflow file out: what the Workflow creator is walked on.
 */
export function workflowing(): Scenario {
  const state = connected([], [], [repository(), LEDGER, SITE]);
  return {
    name: "workflows",
    says: "A repository picked, workflow files of every kind, and one set aside",
    state: {
      ...state,
      repository: repository().root,
      health: { state: "read", health: { probes: [], not_probed: [], helm_action_authority: "acting", workflows_left_out: true } },
    },
    reads: {},
    // The Commands a Trigger names are the ones this repository's Manifest declares.
    behaves: (fleet) => ({ readManifestFile: manifesting().behaves?.(fleet).readManifestFile! }),
  };
}
