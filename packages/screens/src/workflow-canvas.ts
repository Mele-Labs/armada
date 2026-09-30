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
  WorkflowStepCardDesign,
  WorkflowStepCardProps,
  WorkflowStepNeed,
} from "@armada/components";
import type { JobDetail as JobWhole, StepDetail } from "@armada/protocol";

import { droneViewsOf } from "./draft/drone";
import type { GroupState, GroupView } from "./draft/group";
import { checksOf, isRunning } from "./gates";
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

/**
 * The board's tighter spine, for the `compact` card: its card is two lines of
 * `--text-2xs` under `--space-2` padding, so this is that card's height plus
 * `--space-8`, still room for an arrowhead.
 */
const COMPACT_STEP_APART = 88;

/**
 * One row under a card's name — a line, a need, or the gate's chip — with the
 * gap above it: `--leading-xs` and `--space-2`. **The `progress` and `needs`
 * cards leave a row out where there is nothing to say**, so their spine is
 * placed by what each card draws rather than one fixed step, or a quiet card
 * would sit in a gap twice the others.
 */
const ROW = 28;

/** How many rows a `progress` or `needs` card draws under its name. */
function rowsUnder(card: WorkflowStepCardProps): number {
  if (card.design === "needs") return card.needs?.length ?? 0;
  const line = card.progress !== undefined || (card.activity !== "not_started" && card.line !== undefined);
  return Number(line) + Number(card.gate !== undefined);
}

/** How far below this card the next one sits. */
function apartAfter(card: WorkflowStepCardProps): number {
  if (card.design === "compact") return COMPACT_STEP_APART;
  if (card.design === undefined) return STEP_APART;
  // `STEP_APART` is a card with one row under its name.
  return STEP_APART + (rowsUnder(card) - 1) * ROW;
}

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
 * `progress`'s reading of the step at work: a segment per group, how many are
 * done in the bar's tooltip, and how long beside how many Drones are on it.
 * **The Drones are the plan's tasks' own**, as the step's panel lists them.
 */
function progressOf(
  step: StepDetail,
  groups: readonly GroupView[],
  lasted: string | undefined,
): NonNullable<WorkflowStepCardProps["progress"]> {
  const done = groups.filter((group) => group.state === "passed" || group.state === "landed").length;
  const drones = droneViewsOf(groups).filter((one) => one.step === step.step_id && one.state === "running").length;
  const line = [lasted, drones > 0 ? plural(drones, "Drone") : undefined].filter((part) => part !== undefined).join(" · ");
  return {
    groups: groups.map(segmentOf),
    label: `${done} of ${plural(groups.length, "group")} done`,
    ...(line === "" ? {} : { line }),
  };
}

/**
 * `needs`'s lines: what waits on a person, then what went wrong, most pressing
 * first. **Empty on a step nothing needs a person at**, which draws the mark
 * and the name alone.
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
  if (here && whole.command_waiting !== undefined) needs.push({ says: "Command to allow", tone: "waiting" });
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

/** One step's card, as the Workflow board draws it: a mark, the name, one line. */
function stepCard(
  whole: JobWhole,
  step: StepDetail,
  groups: readonly GroupView[],
  now: number,
  onOpen: (() => void) | undefined,
  design: WorkflowStepCardDesign | undefined,
): WorkflowStepCardProps {
  const frozen = frozenBeneath(whole.job.status, step.state);
  const activity = frozen?.activity ?? activityOf(step.state);
  const said = frozen?.word ?? stateOf(step);
  const gate = gateOf(step.advance_gate);
  const lasted = took(step, now, frozen !== undefined);
  const working = activity === "running";
  return {
    kind: "step",
    name: step.label,
    ...(step.label === step.step_id ? { nameIsAnIdentifier: true } : {}),
    activity,
    said,
    ordinal: step.ordinal,
    line: lineOf(step, said, working, groups.length, lasted),
    current: step.step_id === whole.job.current_step_id && frozen === undefined,
    ...(gate === undefined ? {} : { gate }),
    ...(onOpen === undefined ? {} : { onOpen }),
    ...(design === undefined ? {} : { design }),
    ...(design === "progress" && working ? { progress: progressOf(step, groups, lasted) } : {}),
    ...(design === "needs" ? { needs: needsOf(whole, step, activity, groups) } : {}),
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
  /** Which step-card design under comparison draws the run. Absent draws today's. */
  design?: WorkflowStepCardDesign;
};

/**
 * The whole run, placed. **One derivation for both arrangements**, so the
 * toggle changes the shape of the page and never what a step says.
 */
export function workflowRunOf({ whole, groups, onOpen, selected, now = Date.now(), design }: WorkflowRunReading): WorkflowRun {
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
    // The step that works the groups counts them while it works them — the
    // board's `55m · 4 groups`.
    const worked = step.step_id === worksAt ? mine : [];
    const card = read(id, stepCard(whole, step, worked, now, opener(id), design));
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
