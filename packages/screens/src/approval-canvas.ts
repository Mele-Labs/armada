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
import type { WorkflowCanvasEdge, WorkflowStepFact } from "@armada/components";
import type { DeclaredCheck, DeclaredJudge, JobDetail as JobWhole, WorkflowStep } from "@armada/protocol";

import type { GateView } from "./draft/proposal";
import { checkNameOf, deliveryOf } from "./draft/tuning";
import type { ApprovalTuning } from "./draft/tuning";

export type ApprovalNodeKind = "brief" | "base" | "start" | "step" | "checks" | "pr" | "land";

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
}: {
  title: string;
  from: string;
  workflowName: string;
  steps: readonly StepRead[];
  gates: readonly GateView[];
  tuning: ApprovalTuning;
  prMode: "ready" | "draft";
  target: string;
}): { nodes: ApprovalNode[]; edges: WorkflowCanvasEdge[] } {
  const nodes: ApprovalNode[] = [];
  const put = (node: Omit<ApprovalNode, "ordinal">) => nodes.push({ ...node, ordinal: nodes.length + 1 });
  const delivery = deliveryOf(tuning.local, prMode);

  put({ id: "brief", kind: "brief", name: "Brief", line: title, facts: [] });
  put({ id: "base", kind: "base", name: "Base branch", ...(from === "" ? {} : { line: from }), facts: [] });
  put({ id: "start", kind: "start", name: `Start ${workflowName}`, facts: [] });

  const pr = () =>
    put({
      id: "pr",
      kind: "pr",
      name: "Pull request",
      line: delivery === "draft" ? "Draft" : "Ready for review",
      facts: tuning.auto_merge ? [{ value: "Auto-merge" }] : [],
    });
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
