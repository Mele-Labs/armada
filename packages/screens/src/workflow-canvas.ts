// The run, as the workflow the Job froze — placed for the canvas and ordered
// for the stacked column. `#1539`.
//
// **Placement is computed here, from step order.** The canvas holds none, so
// the numbers below are the layout and they are unit-tested in this package.
//
// **The steps are the ones the Job's own workflow file declares** — no
// `verify-and-ship` is added (#1530, 22 Sep).
//
// **The steps, and nothing of the plan** (owner, 29 Sep 2026, `nm0h`). The plan
// was one node hanging off the step that recorded it, after 25 Sep took its
// group and task nodes to the Plan tab. The Workflow board draws no such node,
// and the owner asked for the step's own panel to link to the plan instead, so
// the relation is said where the step is read — `workflow-inspector.ts`.

import type {
  WorkflowCanvasEdge,
  WorkflowCanvasNode,
  WorkflowStackedRow,
  WorkflowStepCardProps,
} from "@armada/components";
import type { JobDetail as JobWhole, StepDetail } from "@armada/protocol";

import type { GroupView } from "./draft/group";
import { ordered } from "./facts";
import { frozenBeneath } from "./frozen";
import { plural } from "./plan-canvas";
import { activityOf, stateOf, took } from "./run";

/**
 * The layout, in the canvas's own coordinates. **The spine runs down** (owner,
 * 29 Sep 2026): one step under the last, so the run reads top to bottom beside
 * the panel that opens over its right.
 *
 * `STEP_APART` is a step card's height plus `--space-12`, which leaves room
 * for an arrowhead. A number rather than a token because React Flow places by
 * number and a `var()` cannot reach it.
 */
const STEP_APART = 112;

/** `step:`, so a node id is never mistaken for another kind in a join. */
export const stepNodeId = (stepId: string): string => `step:${stepId}`;

/** The run, in both arrangements, off one reading. */
export type WorkflowRun = {
  nodes: WorkflowCanvasNode[];
  rows: WorkflowStackedRow[];
  edges: WorkflowCanvasEdge[];
  /** The node the Job is on, for *Stay on the running step*. */
  running: string | null;
  /**
   * What the canvas opens on when the whole run will not read, widest first —
   * the step a person is on with its neighbours, then that step alone.
   */
  opensOn: string[][];
};

/**
 * The gate as the board's chip, where a step waits on somebody — **a person,
 * or the repository's policy**, the two the shapes board marks. The Judge's
 * gate draws nothing here: what it is asked is the panel's, one press away.
 */
function gateOf(gate: string | undefined): string | undefined {
  if (gate === "human_always") return "will ask you";
  if (gate?.startsWith("manifest_rule:")) return "repository policy";
  return undefined;
}

/**
 * The card's one line: how long, then where it has got to — `9m 38s ·
 * advanced`, or `not started`. The step at work names the groups it works in
 * place of the word, as the board draws `55m · 4 groups`: its mark and its
 * hue already say running. A second attempt says so, since nothing else on
 * the card would.
 */
function lineOf(step: StepDetail, said: string, working: boolean, groups: number, lasted: string | undefined): string {
  const parts = [lasted, working && groups > 0 ? plural(groups, "group") : said];
  if (step.attempts.length > 1) parts.push(`attempt ${step.attempts.length}`);
  return parts.filter((part) => part !== undefined).join(" · ");
}

/** One step's card, as the Workflow board draws it: a mark, the name, one line. */
function stepCard(
  whole: JobWhole,
  step: StepDetail,
  groups: number,
  now: number,
  onOpen: (() => void) | undefined,
): WorkflowStepCardProps {
  const frozen = frozenBeneath(whole.job.status, step.state);
  const activity = frozen?.activity ?? activityOf(step.state);
  const said = frozen?.word ?? stateOf(step);
  const gate = gateOf(step.advance_gate);
  return {
    kind: "step",
    name: step.label,
    ...(step.label === step.step_id ? { nameIsAnIdentifier: true } : {}),
    activity,
    said,
    ordinal: step.ordinal,
    line: lineOf(step, said, activity === "running", groups, took(step, now, frozen !== undefined)),
    current: step.step_id === whole.job.current_step_id && frozen === undefined,
    ...(gate === undefined ? {} : { gate }),
    ...(onOpen === undefined ? {} : { onOpen }),
  };
}

/**
 * The step that made the groups: the one the plan was recorded at.
 *
 * Absent where no plan was recorded and where a person wrote it — a plan no
 * step produced has no node to come off, so its groups are left undrawn rather
 * than hung somewhere they were not made.
 */
export function stepTheGroupsWereMadeAt(whole: JobWhole): string | undefined {
  const at = whole.work_plan?.recorded_by;
  if (at === undefined || at.by !== "step") return undefined;
  return ordered(whole).some((step) => step.step_id === at.step_id) ? at.step_id : undefined;
}

/**
 * The step that works the groups: the one after the step the plan was recorded
 * at, since a plan is written at one step and worked at the next.
 *
 * Absent where the recording step is the last. **Nothing on this canvas comes
 * off it any more** — it is read for what a step's own card counts, and for
 * what its panel says about the plan.
 */
export function stepThatWorksTheGroups(whole: JobWhole): string | undefined {
  const made = stepTheGroupsWereMadeAt(whole);
  if (made === undefined) return undefined;
  const steps = ordered(whole);
  return steps[steps.findIndex((step) => step.step_id === made) + 1]?.step_id;
}

export type WorkflowRunReading = {
  whole: JobWhole;
  /** The plan's groups, for what the step at work counts on its card. */
  groups: readonly GroupView[];
  /** Opens a step in the inspector. Absent draws cards that are not controls. */
  onOpen?: (nodeId: string) => void;
  /** The node a person has open, so the card being read says which one it is. */
  selected?: string | null;
  /** What a running step measures to. The caller's clock, so a test can hold it. */
  now?: number;
};

/**
 * The whole run, placed. **One derivation for both arrangements**, so the
 * toggle changes the shape of the page and never what a step says.
 */
export function workflowRunOf({ whole, groups, onOpen, selected, now = Date.now() }: WorkflowRunReading): WorkflowRun {
  const steps = ordered(whole);
  const madeAt = stepTheGroupsWereMadeAt(whole);
  const worksAt = stepThatWorksTheGroups(whole);
  const mine = madeAt === undefined ? [] : groups;
  const opener = (id: string) => (onOpen === undefined ? undefined : () => onOpen(id));
  const read = (id: string, card: WorkflowStepCardProps): WorkflowStepCardProps =>
    id === selected ? { ...card, selected: true } : card;

  const nodes: WorkflowCanvasNode[] = [];
  const rows: WorkflowStackedRow[] = [];
  const edges: WorkflowCanvasEdge[] = [];

  steps.forEach((step, at) => {
    const id = stepNodeId(step.step_id);
    // The step that works the groups counts them while it works them — the
    // board's `55m · 4 groups`.
    const counts = step.step_id === worksAt ? mine.length : 0;
    const card = read(id, stepCard(whole, step, counts, now, opener(id)));
    nodes.push({ id, position: { x: 0, y: at * STEP_APART }, card });

    const loop =
      step.verdict_routing_target === undefined || step.pass === undefined
        ? undefined
        : { to: step.verdict_routing_target, label: `up to ${step.pass.of} passes` };
    const back = loop === undefined ? undefined : steps.find((one) => one.step_id === loop.to);
    rows.push({
      id,
      card,
      ...(back === undefined || loop === undefined
        ? {}
        : { returns: { toName: back.label, label: loop.label } }),
    });
    if (loop !== undefined && back !== undefined) {
      edges.push({
        id: `${step.step_id}>${loop.to}`,
        source: id,
        target: stepNodeId(loop.to),
        kind: "returns",
        label: loop.label,
      });
    }

    const next = steps[at + 1];
    if (next !== undefined) {
      edges.push({
        id: `${step.step_id}>${next.step_id}`,
        source: id,
        target: stepNodeId(next.step_id),
        kind: "leads",
      });
    }
  });

  const at = whole.job.current_step_id;
  const running = at !== undefined && steps.some((step) => step.step_id === at) ? stepNodeId(at) : null;
  return { nodes, rows, edges, running, opensOn: opensOn(steps, at) };
}

/**
 * What to open on, widest first: the step a person is reading with its
 * neighbours, then that step alone.
 */
function opensOn(steps: readonly StepDetail[], at: string | undefined): string[][] {
  const where = steps.findIndex((step) => step.step_id === at);
  if (where === -1) return [steps.map((step) => stepNodeId(step.step_id))];
  const near = steps.slice(Math.max(0, where - 1), where + 2).map((step) => stepNodeId(step.step_id));
  return [near, [stepNodeId(steps[where]!.step_id)]];
}
