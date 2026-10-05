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
  TaskBarSegment,
  WorkflowCanvasEdge,
  WorkflowCanvasNode,
  WorkflowStackedRow,
  WorkflowStepCardProps,
  WorkflowStepNeed,
} from "@armada/components";
import type { JobDetail as JobWhole, StepDetail } from "@armada/protocol";

import { droneViewsOf, type DroneView } from "./draft/drone";
import { stepTheGroupsWereMadeAt, stepThatWorksTheGroups, type GroupState, type GroupView } from "./draft/group";
import { checksOf, isRunning } from "./gates";
import { ordered } from "./facts";
import type { HeldCommand } from "./drone-held";
import { withAsk } from "./held-card";
import { frozenBeneath } from "./frozen";
import { plural } from "./plan-canvas";
import { activityOf, stateOf, took } from "./run";
import { trackOf } from "./step-phase";

// Where the groups were made and are worked is the groups' own fact, and lives
// beside them; the canvas and its readers still ask here.
export { stepTheGroupsWereMadeAt, stepThatWorksTheGroups };

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

/**
 * One row under a card's name — a need, the line, or the gate's chip — with
 * the gap above it: `--leading-xs` and `--space-2`. **A card leaves a row out
 * where there is nothing to say**, so the spine is placed by what each card
 * draws rather than one fixed step, or a quiet card would sit in a gap twice
 * the others.
 */
const ROW = 28;

/** How far below this card the next one sits. `STEP_APART` is a card with one row. */
function apartAfter(card: WorkflowStepCardProps): number {
  const rows =
    (card.needs?.length ?? 0) + Number(card.line !== undefined || card.bar !== undefined) + Number(card.gate !== undefined);
  return STEP_APART + (rows - 1) * ROW + (card.track === undefined ? 0 : TRACK);
}

/**
 * The phase track's row: `--leading-2xs`, `--space-2` above it inside its rule
 * and `--space-2` of the card's own gap, rounded up to the four-unit grid.
 */
const TRACK = 36;

/** `step:`, so a node id is never mistaken for another kind in a join. */
export const stepNodeId = (stepId: string): string => `step:${stepId}`;

/**
 * Where a step sits in the run, **counted from 1 as a person counts**. Fleet's
 * `ordinal` counts from 0, so the first step of a real Job read `step 0` on
 * the board's corner while every fixture, counting from 1, read `step 1`.
 */
export const placeOf = (step: StepDetail): number => step.ordinal + 1;

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
 * The card's line: how long, then where it has got to — `9m 38s · advanced`
 * on a step that ran, `55m · 2 Drones` on the one at work, whose mark and hue
 * already say running. **Time alone under a need**, which already says what
 * the state is; nothing at all on a step nothing entered. A second attempt
 * says so, since nothing else on the card would.
 */
function lineOf(
  step: StepDetail,
  said: string,
  activity: WorkflowStepCardProps["activity"],
  drones: number,
  lasted: string | undefined,
  needed: boolean,
): string | undefined {
  if (activity === "not_started") return undefined;
  const where = needed ? undefined : activity === "running" ? (drones > 0 ? plural(drones, "Drone") : undefined) : said;
  const parts = [lasted, where];
  if (step.attempts.length > 1) parts.push(`attempt ${step.attempts.length}`);
  const line = parts.filter((part) => part !== undefined).join(" · ");
  return line === "" ? undefined : line;
}

/**
 * A group's state as one of the step bar's task segments. A group being run
 * again after its boundary failed keeps the failure's hue: the bar says where
 * the work got to, and that group's work was refused.
 */
function segmentOf(group: GroupView): TaskBarSegment {
  const by: Record<GroupState, TaskBarSegment> = {
    pending: "open",
    running: "working",
    joining: "working",
    checking: "working",
    retrying: group.verdict === "failed" ? "failed" : "working",
    passed: "done",
    landed: "done",
    failed: "failed",
  };
  return by[group.state];
}

/**
 * The group bar on the step at work: a segment per group, and how many are
 * done in its tooltip.
 */
function barOf(groups: readonly GroupView[]): NonNullable<WorkflowStepCardProps["bar"]> {
  const done = groups.filter((group) => group.state === "passed" || group.state === "landed").length;
  return { groups: groups.map(segmentOf), label: `${done} of ${plural(groups.length, "group")} done` };
}

/**
 * What waits on a person, then what went wrong, most pressing first (owner,
 * 30 Sep 2026). **Empty on a step nothing needs a person at.**
 */
function needsOf(
  whole: JobWhole,
  step: StepDetail,
  activity: WorkflowStepCardProps["activity"],
  groups: readonly GroupView[],
): WorkflowStepNeed[] {
  const needs: WorkflowStepNeed[] = [];
  const here = step.step_id === whole.job.current_step_id;
  if (here && (whole.asking !== undefined || whole.judge_question?.step_id === step.step_id)) {
    needs.push({ says: "Question for you", tone: "waiting" });
  } else if (activity === "awaiting_human") {
    needs.push({ says: "Waiting on you", tone: "waiting" });
  }
  // The wire names the step it is held on, so the card says it wherever that is.
  if (whole.command_waiting?.step_id === step.step_id) needs.push({ says: "Needs you", tone: "waiting" });
  for (const read of checksOf(step)) {
    if (!isRunning(read) && read.run?.outcome === "failed") needs.push({ says: `${read.name} failed`, tone: "failed" });
  }
  for (const group of groups) {
    if (group.state === "failed") needs.push({ says: `Group ${group.ordinal} failed`, tone: "failed" });
  }
  if (needs.length === 0 && (activity === "stopped" || activity === "failed")) {
    needs.push({ says: stateOf(step), tone: "failed" });
  }
  return needs;
}

/**
 * One step's card: a mark, the name, and under it what needs a person, then
 * where the work has got to — the group bar and the line on the step at work.
 * **The Drones are the ones the step's panel lists.**
 */
function stepCard(
  whole: JobWhole,
  step: StepDetail,
  groups: readonly GroupView[],
  drones: readonly DroneView[],
  now: number,
  onOpen: (() => void) | undefined,
): WorkflowStepCardProps {
  const frozen = frozenBeneath(whole.job.status, step.state);
  const activity = frozen?.activity ?? activityOf(step.state);
  const said = frozen?.word ?? stateOf(step);
  const gate = gateOf(step.advance_gate);
  const working = activity === "running";
  const needs = needsOf(whole, step, activity, groups);
  const running = drones.filter((one) => one.step === step.step_id && one.state === "running").length;
  const line = lineOf(step, said, activity, running, took(step, now, frozen !== undefined), needs.length > 0);
  const track = trackOf(whole, step, activity, drones);
  return {
    kind: "step",
    name: step.label,
    ...(step.label === step.step_id ? { nameIsAnIdentifier: true } : {}),
    activity,
    said,
    ordinal: placeOf(step),
    ...(line === undefined ? {} : { line }),
    current: step.step_id === whole.job.current_step_id && frozen === undefined,
    ...(gate === undefined ? {} : { gate }),
    ...(onOpen === undefined ? {} : { onOpen }),
    ...(needs.length === 0 ? {} : { needs }),
    ...(working && groups.length > 0 ? { bar: barOf(groups) } : {}),
    ...(track === undefined ? {} : { track }),
  };
}

export type WorkflowRunReading = {
  whole: JobWhole;
  /** The plan's groups, for what the step at work counts on its card. */
  groups: readonly GroupView[];
  /**
   * Every Drone the Job has had, for how many run on each step's card.
   * **Absent is the Job's own Drone alone**, `droneViewsOf` with no list.
   */
  drones?: readonly DroneView[];
  /** Opens a step in the inspector. Absent draws cards that are not controls. */
  onOpen?: (nodeId: string) => void;
  /** The node a person has open, so the card being read says which one it is. */
  selected?: string | null;
  /** What a running step measures to. The caller's clock, so a test can hold it. */
  now?: number;
  /** The command a Drone is held on: a card beside the step it is held at asks. */
  held?: HeldCommand;
};

/**
 * The whole run, placed. **One derivation for both arrangements**, so the
 * toggle changes the shape of the page and never what a step says.
 */
export function workflowRunOf({
  whole,
  groups,
  drones = droneViewsOf(undefined, whole),
  onOpen,
  selected,
  now = Date.now(),
  held,
}: WorkflowRunReading): WorkflowRun {
  let y = 0;
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
    // The step that works the groups draws them as its bar while it works
    // them, and reads its failed groups and its Drones off them.
    const worked = step.step_id === worksAt ? mine : [];
    const card = read(id, stepCard(whole, step, worked, drones, now, opener(id)));
    nodes.push({ id, position: { x: 0, y }, card });
    y += apartAfter(card);

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
  const asked = withAsk(nodes, edges, held === undefined ? undefined : stepNodeId(held.stepId), held);
  return { nodes: asked.nodes, rows, edges: asked.edges, running, opensOn: opensOn(steps, at) };
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
