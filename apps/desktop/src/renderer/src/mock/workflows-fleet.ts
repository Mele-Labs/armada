// The Workflows page's routes on the mock Fleet: the list, a definition's text, a save, a definition step by step with the
// nudges at it, a copy deleted or reset, and the changes an agent drafts. They answer from the definitions in
// `workflows-managed.ts`, which this file holds as files by place and changes as they are asked.
//
// **The same shapes Fleet sends**, so the surface that reads them is the one the app draws: a definition is the file's
// text, a refusal is a `WireError` with Fleet's own code, a save over an id that is already there is refused until it
// says `overwrite`, and a change asked for is `drafting` until the beat an agent takes has passed. Only the refusals a
// person can reach from the page are served.

import { stepsOfDefinition, withWords } from "@armada/components";
import type {
  LeftOutWorkflow,
  Outcome,
  RepositorySummary,
  StepNudge,
  StepReading,
  WorkflowChange,
  WorkflowSummary,
} from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import type { BridgeApi } from "../../../shared/api";
import { manifesting } from "./manifest-fake";
import { connected, type Scenario } from "./moment";
import { MANAGED_DEFINITIONS, MANAGED_ENDS, MANAGED_NUDGES, SUGGESTED_SENTENCE, WIRE_AND_TESTS } from "./workflows-managed";

type Served = Pick<
  BridgeApi,
  | "readWorkflows"
  | "readWorkflowDefinition"
  | "saveWorkflow"
  | "readWorkflowSteps"
  | "deleteWorkflowCopy"
  | "resetWorkflowCopy"
  | "markNotADrift"
  | "askWorkflowChange"
  | "suggestWorkflowChange"
  | "readWorkflowChanges"
  | "approveWorkflowChange"
  | "dismissWorkflowChange"
  | "sendBackWorkflowChange"
  | "proposeWorkflowToArmada"
>;

const refusal = (code: string, message: string): Outcome => ({
  ok: false,
  why: "refused",
  error: { code, message, run_id: "mock", fields: {}, chain: [] },
});

const KIT_FOLDER = "~/.armada/workflows";
const REPOSITORY_FOLDER = ".armada/workflows";

/** How long an agent takes to draft a change before the page reads it `ready`. */
export const DRAFTING_MS = 1200;

type Place = "armada" | "repository" | "kit";

/** A definition held in a place. */
type Held = { source: Place; id: string; file: string; text: string; leftOut?: string };

/** The more specific place answers: Kit's, then the repository's, then Armada's. */
const SPECIFIC: readonly Place[] = ["kit", "repository", "armada"];

const fileOf = (source: Place, id: string) =>
  source === "armada" ? `[armada]/${id}` : `${source === "kit" ? KIT_FOLDER : REPOSITORY_FOLDER}/${id}.json`;

/** A change as the mock keeps it: the wire's, and the moment an agent finishes it. */
type Kept = { change: WorkflowChange; readyAt: number };

/** A nudge as the mock keeps it. */
type KeptNudge = { id: number; source: Place; workflow: string; step: string; nudge: StepNudge };

/** A fresh Fleet's workflows. `current` is the Manifest the window is in, which a repository save is listed under. */
export function workflowsServed(current: () => string | null, options: { leftOut?: LeftOutWorkflow[] } = {}): Served {
  const started = Date.now();
  let files: Held[] = MANAGED_DEFINITIONS.map(({ source, definition }) => ({
    source,
    id: definition.workflow_id,
    file: fileOf(source, definition.workflow_id),
    text: JSON.stringify(definition, null, 2),
  }));
  let nudges: KeptNudge[] = MANAGED_NUDGES.map((one, at) => ({
    id: at + 1,
    source: one.source,
    workflow: one.workflow,
    step: one.step,
    nudge: { id: at + 1, job_id: one.job_id, job_title: one.job_title, fact: one.fact, at: new Date(started - one.ago).toISOString() },
  }));
  let changes: Kept[] = [];
  let nextChange = 1;

  const held = (id: string, source: Place) => files.find((one) => one.id === id && one.source === source && one.leftOut === undefined);
  const missing = (id: string, source: string): Outcome =>
    refusal("fleet.no_such_workflow_definition", `No ${source} definition of \`${id}\` is held`);
  const place = (source: string): Place | undefined => (source === "armada" || source === "repository" || source === "kit" ? source : undefined);

  const insertSentence = (words: string, sentence: string): string => {
    const closing = "Notes come back as another take.";
    return words.includes(closing) ? words.replace(closing, `${sentence} ${closing}`) : `${words} ${sentence}`;
  };

  /** The change as a read sees it: `drafting` until its beat has passed, then the draft or why none fit. */
  const settled = (kept: Kept): WorkflowChange => {
    const { change } = kept;
    if (change.state !== "drafting" || Date.now() < kept.readyAt) return change;
    const text = change.before;
    const asked = change.origin.kind === "asked" ? change.origin.text : undefined;
    let after: string | undefined;
    if (asked !== undefined) {
      const steps = JSON.parse(text) as { steps: { id: string }[] };
      if (/split\s+refine/i.test(asked) && steps.steps.some((one) => one.id === "refine")) {
        after = JSON.stringify({ ...steps, steps: steps.steps.flatMap((one): unknown[] => (one.id === "refine" ? [...WIRE_AND_TESTS] : [one])) }, null, 2);
      }
    } else if (change.step_id !== undefined) {
      const sentence = SUGGESTED_SENTENCE[change.step_id];
      const words = stepsOfDefinition(text)?.find((one) => one.id === change.step_id)?.instructions;
      if (sentence !== undefined && words !== undefined) after = withWords(text, change.step_id, insertSentence(words, sentence));
    }
    if (after === undefined) {
      return { ...change, state: "failed", reason: "The drafted workflow is the same as the one it was made from" };
    }
    return { ...change, state: "ready", after };
  };

  const open = (id: string, source: string) =>
    changes.filter(({ change }) => change.workflow_id === id && change.source === source && change.state !== "approved" && change.state !== "dismissed");

  const stepsOf = (one: Held): StepReading[] =>
    (stepsOfDefinition(one.text) ?? []).map((step) => {
      const end = MANAGED_ENDS[`${one.id}/${step.id}`] ?? MANAGED_ENDS[step.id];
      const here = nudges.filter((each) => each.source === one.source && each.workflow === one.id && each.step === step.id).map((each) => each.nudge);
      return {
        id: step.id,
        label: step.label,
        ...(step.instructions === undefined ? {} : { instructions: step.instructions }),
        ...(end === undefined ? {} : { ends_when: end.ends }),
        ...(end?.passedOver === undefined ? {} : { passed_over_when: end.passedOver }),
        ...(here.length === 0 ? {} : { nudges: here }),
      };
    });

  /** A change begun on a copy: kept `drafting`, with the beat an agent takes. */
  const begin = (id: string, source: Place, origin: WorkflowChange["origin"], stepId?: string): WorkflowChange => {
    const change: WorkflowChange = {
      id: nextChange++,
      workflow_id: id,
      source,
      ...(stepId === undefined ? {} : { step_id: stepId }),
      origin,
      before: held(id, source)!.text,
      state: "drafting",
      at: new Date().toISOString(),
    };
    changes = [...changes, { change, readyAt: Date.now() + DRAFTING_MS }];
    return change;
  };

  // The suggestion already drawn up at Walk in the repository's `feature`, from the two Jobs Fleet nudged there.
  const walked = held("feature", "repository");
  if (walked !== undefined) {
    const from = nudges.filter((one) => one.source === "repository" && one.step === "walk").map((one) => one.id);
    const change = begin("feature", "repository", { kind: "drift", nudges: from }, "walk");
    changes = changes.map((one) => (one.change.id === change.id ? { ...one, readyAt: 0 } : one));
  }

  const outcomeOfChange = (id: number) => changes.find((one) => one.change.id === id);
  const noChange = refusal("fleet.no_such_workflow_change", "No change by that id is held");
  const revise = (id: number, to: (kept: Kept) => Kept) => {
    changes = changes.map((one) => (one.change.id === id ? to(one) : one));
  };

  return {
    readWorkflows: async () => {
      const ids = [...new Set(files.filter((one) => one.leftOut === undefined).map((one) => one.id))];
      return {
        ok: true,
        workflows: ids.map((id): WorkflowSummary => {
          const here = SPECIFIC.map((source) => held(id, source)).filter((one) => one !== undefined);
          const [runs, ...replaced] = here;
          const forRequests = (JSON.parse(runs!.text) as { for_requests?: string }).for_requests;
          return {
            id,
            name: id,
            version: 1,
            steps: [],
            manifest_id: current() ?? "",
            source: runs!.source,
            file: runs!.file,
            ...(forRequests === undefined ? {} : { for_requests: forRequests }),
            ...(replaced.length === 0 ? {} : { overrides: replaced.map((one) => ({ source: one.source, file: one.file })) }),
            ...(runs!.source === "kit" && replaced.some((one) => one.source === "repository" && one.text !== runs!.text) ? { hides: "repository" } : {}),
          };
        }),
        leftOut: options.leftOut ?? [],
      };
    },
    readWorkflowDefinition: async (workflowId, source) => {
      const one = place(source) === undefined ? undefined : held(workflowId, source as Place);
      return one === undefined
        ? { ok: false, outcome: missing(workflowId, source) }
        : { ok: true, definition: { workflow_id: one.id, source: one.source, file: one.file, definition: one.text } };
    },
    saveWorkflow: async ({ manifestId, body }) => {
      const id = String((JSON.parse(body.definition) as { workflow_id?: unknown }).workflow_id ?? "");
      if (!/^[A-Za-z0-9_-]+$/.test(id)) {
        return {
          ok: false,
          outcome: refusal("fleet.workflow_id_not_a_name", `\`${id}\` cannot be a file's name: a workflow_id here is letters, digits, \`-\` and \`_\``),
        };
      }
      const source: Place = body.scope === "kit" ? "kit" : "repository";
      const file = fileOf(source, id);
      const was = held(id, source);
      // Another Manifest's folder is not the one the list is read from.
      const here = source === "kit" || manifestId === current();
      if (here && was !== undefined && body.overwrite !== true) {
        return {
          ok: false,
          outcome: refusal("fleet.workflow_exists", `${was.file} already defines \`${id}\` and nothing was written; save again with overwrite to replace it`),
        };
      }
      if (here) files = [...files.filter((one) => one !== was), { source, id, file, text: body.definition }];
      return { ok: true, saved: { workflow_id: id, scope: body.scope, file, replaced: was !== undefined, runs_from: source } };
    },
    readWorkflowSteps: async (workflowId, source) => {
      const one = place(source) === undefined ? undefined : held(workflowId, source as Place);
      return one === undefined
        ? { ok: false, outcome: missing(workflowId, source) }
        : { ok: true, steps: { workflow_id: one.id, source: one.source, steps: stepsOf(one) } };
    },
    deleteWorkflowCopy: async ({ copy }) => {
      const source = place(copy.source);
      if (source === "armada") return { ok: false, outcome: refusal("fleet.workflow_is_armadas", "Armada's workflows are read-only here") };
      const one = source === undefined ? undefined : held(copy.workflow_id, source);
      if (one === undefined) return { ok: false, outcome: missing(copy.workflow_id, copy.source) };
      files = files.filter((each) => each !== one);
      const runs = SPECIFIC.map((where) => held(copy.workflow_id, where)).find((each) => each !== undefined);
      return { ok: true, deleted: { workflow_id: one.id, source: one.source, file: one.file, ...(runs === undefined ? {} : { runs_from: runs.source }) } };
    },
    resetWorkflowCopy: async ({ copy }) => {
      const source = place(copy.source);
      if (source === "armada") return { ok: false, outcome: refusal("fleet.workflow_is_armadas", "Armada's workflows are read-only here") };
      const one = source === undefined ? undefined : held(copy.workflow_id, source);
      const armada = held(copy.workflow_id, "armada");
      if (one === undefined) return { ok: false, outcome: missing(copy.workflow_id, copy.source) };
      if (armada === undefined) return { ok: false, outcome: refusal("fleet.workflow_not_carried", `Armada carries no \`${copy.workflow_id}\``) };
      files = files.map((each) => (each === one ? { ...one, text: armada.text } : each));
      return { ok: true, saved: { workflow_id: one.id, scope: one.source === "kit" ? "kit" : "repository", file: one.file, replaced: true, runs_from: one.source } };
    },
    markNotADrift: async (nudge) => {
      if (!nudges.some((one) => one.id === nudge)) return { ok: false, outcome: refusal("fleet.no_such_nudge", "No nudge by that id is kept") };
      nudges = nudges.filter((one) => one.id !== nudge);
      return { ok: true, marked: { nudge } };
    },
    askWorkflowChange: async ({ body }) => {
      const source = place(body.source);
      if (source === "armada") return { ok: false, outcome: refusal("fleet.workflow_is_armadas", "Armada's workflows are read-only here") };
      if (source === undefined || held(body.workflow_id, source) === undefined) return { ok: false, outcome: missing(body.workflow_id, body.source) };
      if (body.text.trim() === "") return { ok: false, outcome: refusal("fleet.workflow_change_empty", "Say what to change") };
      return { ok: true, change: begin(body.workflow_id, source, { kind: "asked", text: body.text.trim() }) };
    },
    suggestWorkflowChange: async ({ body }) => {
      const source = place(body.source);
      if (source === "armada") return { ok: false, outcome: refusal("fleet.workflow_is_armadas", "Armada's workflows are read-only here") };
      const one = source === undefined ? undefined : held(body.workflow_id, source);
      if (source === undefined || one === undefined) return { ok: false, outcome: missing(body.workflow_id, body.source) };
      if (!(stepsOfDefinition(one.text) ?? []).some((step) => step.id === body.step_id)) {
        return { ok: false, outcome: refusal("fleet.workflow_step_not_here", `\`${body.step_id}\` is not a step of \`${body.workflow_id}\``) };
      }
      const counted = nudges.filter((each) => each.source === source && each.workflow === body.workflow_id && each.step === body.step_id);
      if (counted.length === 0) return { ok: false, outcome: refusal("fleet.workflow_nothing_drifted", "No Job drifted at that step") };
      const standing = open(body.workflow_id, body.source).find(({ change }) => change.step_id === body.step_id);
      if (standing !== undefined) return { ok: false, outcome: refusal("fleet.workflow_change_open", "That step has a change waiting") };
      return { ok: true, change: begin(body.workflow_id, source, { kind: "drift", nudges: counted.map((each) => each.id) }, body.step_id) };
    },
    readWorkflowChanges: async (workflowId, source) => ({
      ok: true,
      changes: { changes: open(workflowId, source).map((kept) => settled(kept)) },
    }),
    approveWorkflowChange: async (id) => {
      const kept = outcomeOfChange(id);
      if (kept === undefined) return { ok: false, outcome: noChange };
      const change = settled(kept);
      if (change.state !== "ready" || change.after === undefined) {
        return { ok: false, outcome: refusal("fleet.workflow_change_not_ready", "That change is not ready to approve") };
      }
      const source = place(change.source)!;
      const one = held(change.workflow_id, source);
      if (one === undefined || one.text !== change.before) {
        return { ok: false, outcome: refusal("fleet.workflow_changed_since", `${one?.file ?? change.workflow_id} changed since the draft was made. Dismiss the draft and ask again`) };
      }
      files = files.map((each) => (each === one ? { ...one, text: change.after! } : each));
      const approved: WorkflowChange = { ...change, state: "approved" };
      revise(id, (each) => ({ ...each, change: approved }));
      return { ok: true, approved: { change: approved, saved: { workflow_id: one.id, scope: source === "kit" ? "kit" : "repository", file: one.file, replaced: true, runs_from: source } } };
    },
    dismissWorkflowChange: async (id) => {
      const kept = outcomeOfChange(id);
      if (kept === undefined) return { ok: false, outcome: noChange };
      const dismissed: WorkflowChange = { ...settled(kept), state: "dismissed" };
      revise(id, (each) => ({ ...each, change: dismissed }));
      return { ok: true, change: dismissed };
    },
    sendBackWorkflowChange: async ({ change: id, notes }) => {
      const kept = outcomeOfChange(id);
      if (kept === undefined) return { ok: false, outcome: noChange };
      if (notes.trim() === "") return { ok: false, outcome: refusal("fleet.workflow_change_empty", "Say what was wrong with the draft") };
      const again: WorkflowChange = { ...settled(kept), state: "drafting", notes: notes.trim() };
      delete again.after;
      delete again.reason;
      revise(id, () => ({ change: again, readyAt: Date.now() + DRAFTING_MS }));
      return { ok: true, change: again };
    },
    proposeWorkflowToArmada: async ({ workflow_id, definition }) =>
      definition.trim() === ""
        ? { ok: false, outcome: refusal("fleet.workflow_change_empty", "Nothing to propose") }
        : { ok: true, proposed: { workflow_id, session_id: "mock-armada-proposal" } },
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
  const state = connected([], [repository(), LEDGER, SITE]);
  return {
    name: "workflows",
    says: "A repository picked, workflow files of every kind, and one set aside",
    state: {
      ...state,
      repository: repository().root,
      health: { state: "read", health: { probes: [], not_probed: [], helm_action_authority: "acting", workflows_left_out: true } },
    },
    // The Commands a Trigger names are the ones this repository's Manifest declares.
    behaves: (fleet) => ({ readManifestFile: manifesting().behaves?.(fleet).readManifestFile! }),
  };
}

/** Fleet's health read with a probe of every module and one warning and one failure, which Doctor is walked on. */
export function doctoring(): Scenario {
  const base = workflowing();
  const probe = (module: string, outcome: string, detail: string) => ({ module, outcome, detail });
  return {
    ...base,
    name: "doctor",
    says: "Doctor with every module probed, one warning and one failure",
    state: {
      ...base.state,
      health: {
        state: "read",
        health: {
          probes: [
            probe("Fleet", "pass", "daemon alive, pid 48213, up 3h"),
            probe("Kit", "pass", "/Users/user/.armada/kit.yml"),
            probe("Machine", "pass", "/Users/user/.armada/machine.yml"),
            probe("Manifest", "warn", "/Users/user/armada/armada.yml, drifted from main"),
            probe("Manifest", "pass", "/Users/user/ledger/armada.yml"),
            probe("SQLite", "pass", "opens and writes"),
            probe("System stats", "pass", "cpu 21% in use, memory 62% in use, 184320000 KiB free on the volume"),
            probe("Slots", "fail", "slot 7 stranded, holder gone"),
          ],
          not_probed: [{ owner: "adapters", because: "No adapter is configured on this machine." }],
          helm_action_authority: "acting",
        },
      },
    },
  };
}
