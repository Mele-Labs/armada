// Workflows switched off, for the walk `workflows-off`: Kit, Workflows with All repositories picked, a switch on each row, one workflow off for the machine
// (`prototype`) and one off for armada only (`bug`), under `workflows.off` in settings.json. Two Jobs in armada: one held on its start card, whose Workflow picker lists
// neither, and one with no workflow, whose right column reads Brief, Done, Now and Open. Fleet serves `workflows.off` in the settings list; here the mock's settings slice does.

import type { WaitingItem, WorkflowSummary } from "@armada/protocol";
import { MANIFEST_ID } from "@armada/screens/src/fixtures/repository";
import { MOCK_SETTINGS, savedInto } from "@armada/settings/fake";

import { FakeSessionsFleet, hosted, pullRequest } from "../sessions-fleet";
import type { Scenario } from "../moment";
import { workflowing } from "../workflows-fleet";

const AT = "2026-10-29T09:14:02.000Z";
const HELD = "01WFOFFHELDAAAAAAAAAAAAAAA";
const PLAIN = "01WFOFFPLAINAAAAAAAAAAAAAA";

/** The repository's workflows as the Workflow picker lists them: five, two of them off. */
const WORKFLOWS = [
  { id: "feature", name: "Feature", steps: ["Propose", "Discuss", "Approve plan", "Walk", "Refine", "Deliver"] },
  { id: "bug", name: "Bug", steps: ["Find the cause", "Fix", "Short walk", "Tests", "Deliver"] },
  { id: "design_plan", name: "Design plan", steps: ["Propose", "Sketch", "Record plan"] },
  { id: "prototype", name: "Prototype", steps: ["Propose", "Build", "Show"] },
  { id: "docs_only", name: "Docs only", steps: ["Draft", "Deliver"] },
].map((one): WorkflowSummary => ({ id: one.id, name: one.name, version: 1, manifest_id: MANIFEST_ID, steps: one.steps.map((label) => ({ step_id: label.toLowerCase().replace(/ /g, "_"), label, checks: [], judge_checks: [] })) }) as unknown as WorkflowSummary);

const OFF = { machine: ["prototype"], repositories: { [MANIFEST_ID]: ["bug"] } };

const ASK: WaitingItem = {
  id: "cap-ask",
  text: "Cap retries at 3 or at 5?",
  since: AT,
  source: "agent",
  act: { kind: "answer", target: "" },
  options: [{ label: "3" }, { label: "5" }],
};

const records = () => [
  hosted(HELD, {
    title: "Add a retry cap to the store client",
    manifest_id: MANIFEST_ID,
    last_turn_at: AT,
    workflow: "feature",
    hosted: { turn: { state: "idle" }, mode: "auto", running: false, start: { asks_delivery: true, message: "Add a retry cap to the store client" } },
  }),
  hosted(PLAIN, {
    title: "Retry cap in the store client",
    manifest_id: MANIFEST_ID,
    last_turn_at: AT,
    attachments: [
      pullRequest(2101, { state: "merged", title: "Read the retry cap from settings", branch: "fix/retry-cap-read" }),
      pullRequest(2102, { state: "open", checks: "pending", title: "Cap retries in the store client", branch: "fix/retry-cap" }),
    ],
    waiting_for: [ASK],
  }),
];

const threads = () => ({
  [PLAIN]: [
    { kind: "message" as const, id: "m1", at: AT, from: { kind: "you" as const }, text: "Cap the retries in the store client." },
    { kind: "message" as const, id: "m2", at: AT, from: { kind: "agent" as const }, text: "The cap is read from settings. Next is the client's loop." },
  ],
});

function build(): Scenario {
  const base = workflowing();
  const board: Scenario = {
    ...base,
    // All repositories picked, so the Workflows page opens on the machine's switches.
    state: {
      ...base.state,
      repository: null,
      holds: { ...base.state.holds, workflows: WORKFLOWS },
      settings: savedInto(MOCK_SETTINGS, { "workflows.off": OFF }),
    },
  };
  const served = new FakeSessionsFleet(records(), threads()).scenario(board);
  return {
    ...served,
    name: "workflows-off",
    says: "Workflows with a switch on each row, one off for the machine and one off for armada; a held Job whose picker leaves both out; a Job with no workflow",
    behaves: (handle) => ({ ...base.behaves?.(handle), ...served.behaves?.(handle) }),
  };
}

export const workflowsOff: Scenario = build();
