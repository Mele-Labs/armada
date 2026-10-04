// What a Job at its dispatch gate will do, as nodes left to right: the brief,
// where the work starts, the workflow starting, each step and what gates it,
// the pull request, and where it lands. The approval canvas, prototype.
//
// **The order is the run's.** A step's gate follows the step as its own node
// where the step declares Checks or a Judge — what the owner drew, `Plan →
// Checks → Implement` — and sits on the step's own card where it declares
// neither, so a step a person alone reads is one node and not two.
//
// **What lands reshapes the end.** Local delivery has no pull request to open,
// so its node goes and Land reads as a local merge; auto-merge says on the edge
// into Land that it merges on its own. One field each, read here.

import { STEP_STATE } from "@armada/components";
import type { StepActivity, WorkflowCanvasEdge, WorkflowStepFact } from "@armada/components";
import type { LucideIcon } from "lucide-react";
import type { DeclaredCheck, DeclaredJudge, JobDetail as JobWhole, WorkflowStep } from "@armada/protocol";

import type { GateView } from "./draft/proposal";
import { checkNameOf, deliveryOf } from "./draft/tuning";
import type { ApprovalTuning } from "./draft/tuning";

export type ApprovalNodeKind =
  | "brief"
  | "base"
  | "start"
  | "step"
  | "checks"
  | "groups"
  | "group"
  | "done"
  | "pr"
  | "land";

/**
 * Where a node is in the Job's life, once it has one. Absent is a node nothing
 * reached, which is every node at the gate.
 */
export type NodeLife = {
  activity: StepActivity;
  /** The registry's word for it, read to somebody who cannot see the mark. */
  said: string;
  /** A registry row's own glyph and token, where the node is not a step — a plan group. */
  mark?: { icon: LucideIcon; token: string };
  /** The node the Job is at. */
  current?: boolean;
};

/** One of the plan's groups, as the canvas draws it in the Groups node's place. */
export type GroupRead = { id: string; name: string; life: NodeLife };

/** What the running Job says about each node. Absent is the gate. */
export type LifeRead = {
  /** By node id: `brief`, a step's id, `plan:checks`, `land`. */
  nodes: Readonly<Record<string, NodeLife>>;
  /** The plan's groups, once a plan is recorded. Absent draws the placeholder. */
  groups?: readonly GroupRead[];
};

/** One step of the chosen workflow, as the canvas reads it. */
export type StepRead = {
  id: string;
  label: string;
  checks: readonly DeclaredCheck[];
  judges: readonly DeclaredJudge[];
  delivers: boolean;
};

export type ApprovalNode = {
  id: string;
  kind: ApprovalNodeKind;
  /** The step it belongs to, on a step or its gate. */
  stepId?: string;
  name: string;
  /** Its position in the run, from one. */
  ordinal: number;
  line?: string;
  facts: WorkflowStepFact[];
  /** Where the node waits for a person. */
  gate?: string;
  /** Where the Job is on it. Absent is not reached. */
  life?: NodeLife;
  /** Whether a press opens a card. Absent is yes. */
  inert?: true;
};

/** The step-state word every node reads as before the press. */
export const NOT_STARTED = STEP_STATE["not_started"]?.verb ?? "not started";

/** What a gate node and a step's own gate say a person does. */
const YOU = "You";

/**
 * The steps of the chosen workflow in the order the gates hold them. **The
 * frozen step first, then the workflow's** — `gateRowsOf`'s rule, so a newly
 * picked workflow reads its own declarations.
 */
export function stepsReadOf(
  gates: readonly GateView[],
  whole: JobWhole | null,
  declared: ReadonlyMap<string, WorkflowStep>,
): StepRead[] {
  return gates.map((gate) => {
    const frozen = whole?.steps.find((one) => one.step_id === gate.step_id);
    const step = declared.get(gate.step_id) ?? frozen;
    return {
      id: gate.step_id,
      label: step?.label ?? gate.step_id,
      checks: step?.checks ?? [],
      judges: step?.judge_checks ?? [],
      delivers: step?.delivers ?? false,
    };
  });
}

/**
 * Whether the step runs a Drone per task. **A stand-in**: `WorkflowSummary`
 * carries no `drone_per_task`, so the step the plan's tasks are worked on is
 * read by its id. Owed with the rest of `draft/tuning.ts`.
 */
export const perTask = (step: StepRead): boolean => step.id === "implement";

/** Whether a step's gate is a node of its own. */
export const gateIsANode = (step: StepRead): boolean => step.checks.length > 0 || step.judges.length > 0;

export function approvalNodesOf({
  title,
  from,
  workflowName,
  steps,
  gates,
  tuning,
  prMode,
  target,
  life,
}: {
  title: string;
  from: string;
  workflowName: string;
  steps: readonly StepRead[];
  gates: readonly GateView[];
  tuning: ApprovalTuning;
  prMode: "ready" | "draft";
  target: string;
  life?: LifeRead;
}): { nodes: ApprovalNode[]; edges: WorkflowCanvasEdge[] } {
  const nodes: ApprovalNode[] = [];
  const put = (node: Omit<ApprovalNode, "ordinal">) => {
    const at = life?.nodes[node.id];
    nodes.push({ ...node, ...(at === undefined ? {} : { life: at }), ordinal: nodes.length + 1 });
  };
  const delivery = deliveryOf(tuning.local, prMode);

  put({ id: "brief", kind: "brief", name: "Brief", line: title, facts: [] });
  put({ id: "base", kind: "base", name: "Base branch", ...(from === "" ? {} : { line: from }), facts: [] });
  put({ id: "start", kind: "start", name: `Start ${workflowName}`, facts: [] });

  // What counts as the work being done, read just before it leaves (the
  // owner, 4 Oct 2026). The same criteria Brief's card edits.
  const done = () => put({ id: "done", kind: "done", name: "Done when", facts: [] });
  const pr = () => {
    done();
    put({
      id: "pr",
      kind: "pr",
      name: "Pull request",
      line: delivery === "draft" ? "Draft" : "Ready for review",
      facts: tuning.auto_merge ? [{ value: "Auto-merge" }] : [],
    });
  };
  let opened = false;
  for (const step of steps) {
    const gate = gates.find((one) => one.step_id === step.id);
    const tuned = tuning.steps[step.id];
    // The pull request opens before the step that sends the work out, which
    // is where a person reviews it.
    if (step.delivers && delivery !== "local" && !opened) {
      pr();
      opened = true;
    }
    const facts: WorkflowStepFact[] = [];
    if (tuned?.model != null) facts.push({ value: tuned.model, hint: "Model" });
    if (tuned?.effort != null) facts.push({ value: tuned.effort, hint: "Effort" });
    if (tuned?.harness != null) facts.push({ value: tuned.harness, hint: "Harness" });
    if (perTask(step)) facts.push({ value: "Drone per task" });
    const ownGate = !gateIsANode(step) && gate?.you === true;
    put({
      id: step.id,
      kind: "step",
      stepId: step.id,
      name: step.label,
      facts,
      ...(ownGate ? { gate: YOU } : {}),
    });
    // The plan's groups, worked by this step: a placeholder until Plan has
    // recorded them, and each group once it has.
    if (perTask(step)) {
      if (life?.groups === undefined || life.groups.length === 0) {
        put({ id: "groups", kind: "groups", name: "Groups", facts: [], inert: true });
      } else {
        for (const group of life.groups) {
          nodes.push({
            id: `group:${group.id}`,
            kind: "group",
            stepId: step.id,
            name: group.name,
            facts: [],
            life: group.life,
            inert: true,
            ordinal: nodes.length + 1,
          });
        }
      }
    }
    if (gateIsANode(step)) {
      const judges = tuned?.judges ?? 1;
      const said: WorkflowStepFact[] = [];
      if (gate?.checks === true) said.push({ value: "Checks" });
      if (gate?.judge === true) said.push({ value: judges === 1 ? "Judge" : `${judges} Judges` });
      put({
        id: `${step.id}:checks`,
        kind: "checks",
        stepId: step.id,
        name: "Checks",
        facts: said,
        ...(gate?.you === true ? { gate: YOU } : {}),
      });
    }
  }
  // A workflow with no step that delivers still opens its pull request last.
  if (delivery !== "local" && !opened) pr();
  if (delivery === "local") done();
  put({
    id: "land",
    kind: "land",
    name: "Land",
    ...(target === "" ? {} : { line: target }),
    facts: delivery === "local" ? [{ value: "Local merge" }] : [],
    ...(delivery !== "local" && !tuning.auto_merge ? { gate: YOU } : {}),
  });

  const edges: WorkflowCanvasEdge[] = nodes.slice(1).map((node, at) => {
    const from = nodes[at]!;
    const own = node.kind === "land" && delivery !== "local" && tuning.auto_merge;
    return {
      id: `${from.id}->${node.id}`,
      source: from.id,
      target: node.id,
      kind: "leads",
      ...(own ? { label: "merges on its own" } : {}),
    };
  });
  return { nodes, edges };
}

/** The declared Checks of a step by name, each with whether this Job runs it. */
export function checksOf(step: StepRead, off: readonly string[]): { name: string; runs: boolean }[] {
  return step.checks.map((check) => {
    const name = checkNameOf(check);
    return { name, runs: !off.includes(name) };
  });
}
