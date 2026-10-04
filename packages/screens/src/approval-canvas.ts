// What a Job at its dispatch gate will do, as nodes top to bottom: the brief,
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

import { AUTO, STEP_STATE } from "@armada/components";
import type { RunNodeTrait, StepActivity, WorkflowCanvasEdge } from "@armada/components";
import type { LucideIcon } from "lucide-react";
import type { DeclaredCheck, DeclaredJudge, JobDetail as JobWhole, WorkflowStep } from "@armada/protocol";

import type { GateView } from "./draft/proposal";
import type { TaskView } from "./draft/task";
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
  | "jobs"
  | "job"
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

/** A plan group, with the tasks its card lists. */
export type PlanGroupRead = GroupRead & { tasks: readonly TaskView[] };

/** One Job the wave dispatched, and the ones it waits on, by id. */
export type MemberRead = GroupRead & { waits_on: readonly string[] };

/** What the running Job says about each node. Absent is the gate. */
export type LifeRead = {
  /** By node id: `brief`, a step's id, `plan:checks`, `land`. */
  nodes: Readonly<Record<string, NodeLife>>;
  /** The plan's groups, once a plan is recorded. Absent draws the placeholder. */
  groups?: readonly PlanGroupRead[];
  /** The Jobs the wave dispatched, on its live pass. Absent draws the placeholder. */
  jobs?: readonly MemberRead[];
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
  /** The body's first line where it is not the name — the brief's title, the base's branch. */
  face?: string;
  /** A branch, so mono. */
  faceMono?: boolean;
  /** The id the band carries, right-aligned: a step's, a group's. */
  bandId?: string;
  /** The body's second line, in words — the brief's request. */
  line?: string;
  /** The body's second line as values: a step's model, effort and harness. */
  traits: RunNodeTrait[];
  /** The body's third line: Drones, Judges, how it merges. */
  meta: RunNodeTrait[];
  /** Done when's criteria. */
  items?: readonly string[];
  /** Where the Job is on it. Absent is not reached. */
  life?: NodeLife;
  /** Whether a press opens a card. Absent is yes. */
  inert?: true;
  /** A dispatched Job: a press opens that Job rather than a card. */
  opensJob?: string;
  /** A plan group's tasks, which its card lists. */
  tasks?: readonly TaskView[];
  /**
   * A dispatched Job's place in the wave: its depth — how many Jobs stand
   * before it — and where it sits among the Jobs of that depth.
   */
  band?: { depth: number; index: number; of: number };
  /** The Jobs this one waits on, by node id. */
  waits_on?: readonly string[];
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

/**
 * The step the wave's Jobs are dispatched from. **A stand-in**:
 * `WorkflowSummary` carries no `may_dispatch_jobs`, so it is Epic's first
 * step, which is the one `epic.json` declares it on. Owed beside
 * `drone_per_task`.
 */
export function dispatchesFromOf(workflowId: string, steps: readonly StepRead[]): string | undefined {
  return workflowId === "epic" ? steps[0]?.id : undefined;
}

/** How many Jobs stand before each one, by `waits_on` — the wave canvas's own reading. */
function depthsOf(jobs: readonly MemberRead[]): Map<string, number> {
  const byId = new Map(jobs.map((job) => [job.id, job]));
  const depth = new Map<string, number>();
  const of = (id: string, reaching: ReadonlySet<string>): number => {
    const held = depth.get(id);
    if (held !== undefined) return held;
    if (reaching.has(id)) return 0;
    const waits = (byId.get(id)?.waits_on ?? []).filter((one) => byId.has(one));
    const mine = waits.length === 0 ? 0 : Math.max(...waits.map((one) => of(one, new Set([...reaching, id])))) + 1;
    depth.set(id, mine);
    return mine;
  };
  for (const job of jobs) of(job.id, new Set());
  return depth;
}

/** Whether a step's gate is a node of its own. */
export const gateIsANode = (step: StepRead): boolean => step.checks.length > 0 || step.judges.length > 0;

/** What each delivery is called on the pull request's face. */
const PR_FACE = { draft: "Draft pull request", ready: "Pull request" } as const;

/** A value that names itself — a gate's Checks, a pull request's auto-merge. */
const plain = (value: string, tuned = false): RunNodeTrait => ({ key: value, value, ...(tuned ? { tuned } : {}) });

export function approvalNodesOf({
  title,
  asked = "",
  criteria = [],
  from,
  workflowName,
  steps,
  gates,
  tuning,
  prMode,
  target,
  life,
  droneCap,
  dispatchesFrom,
}: {
  title: string;
  /** The request, the brief's second line. */
  asked?: string;
  /** What counts as done, in order. */
  criteria?: readonly string[];
  from: string;
  workflowName: string;
  steps: readonly StepRead[];
  gates: readonly GateView[];
  tuning: ApprovalTuning;
  prMode: "ready" | "draft";
  target: string;
  life?: LifeRead;
  /** How many Drones the Job may run at once. Absent is the machine's. */
  droneCap?: number;
  /** The step the wave's Jobs are dispatched from, where the workflow dispatches any. */
  dispatchesFrom?: string;
}): { nodes: ApprovalNode[]; edges: WorkflowCanvasEdge[] } {
  const nodes: ApprovalNode[] = [];
  const put = (node: Omit<ApprovalNode, "ordinal">) => {
    const at = life?.nodes[node.id];
    nodes.push({ ...node, ...(at === undefined ? {} : { life: at }), ordinal: nodes.length + 1 });
  };
  const delivery = deliveryOf(tuning.local, prMode);

  put({ id: "brief", kind: "brief", name: "Brief", face: title, ...(asked === "" ? {} : { line: asked }), traits: [], meta: [] });
  put({
    id: "base",
    kind: "base",
    name: "Base branch",
    ...(from === "" ? {} : { face: from, faceMono: true }),
    traits: [],
    meta: [],
  });
  put({ id: "start", kind: "start", name: `Start ${workflowName}`, face: workflowName, traits: [], meta: [] });

  // What counts as the work being done, read just before it leaves (the
  // owner, 4 Oct 2026). The same criteria Brief's card edits.
  const done = () =>
    put({ id: "done", kind: "done", name: "Done when", ...(criteria.length === 0 ? {} : { items: criteria }), traits: [], meta: [] });
  const pr = () => {
    done();
    put({
      id: "pr",
      kind: "pr",
      name: "Pull request",
      face: PR_FACE[delivery === "draft" ? "draft" : "ready"],
      traits: [],
      meta: [
        { key: "Pull request", value: delivery === "draft" ? "draft" : "ready", tuned: delivery === "draft" },
        { key: "Auto-merge", value: tuning.auto_merge ? "auto-merge" : "you merge", tuned: tuning.auto_merge },
      ],
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
    const ownGate = !gateIsANode(step) && gate?.you === true;
    // A step a person alone reads runs no Drone, so it carries its gate and nothing a Drone is tuned by.
    put({
      id: step.id,
      kind: "step",
      stepId: step.id,
      name: step.label,
      bandId: step.id,
      traits: ownGate
        ? [plain(YOU)]
        : [
            { key: "Model", value: tuned?.model ?? AUTO, tuned: tuned?.model != null },
            { key: "Effort", value: tuned?.effort ?? AUTO, tuned: tuned?.effort != null },
            { key: "Harness", value: tuned?.harness ?? AUTO, tuned: tuned?.harness != null },
          ],
      meta: perTask(step)
        ? [{ key: "Drones at once", value: `drones ${droneCap ?? AUTO}`, tuned: droneCap !== undefined }]
        : [],
    });
    // The plan's groups, worked by this step: a placeholder until Plan has
    // recorded them, and each group once it has.
    if (perTask(step)) {
      if (life?.groups === undefined || life.groups.length === 0) {
        put({ id: "groups", kind: "groups", name: "Groups", traits: [], meta: [], inert: true });
      } else {
        for (const group of life.groups) {
          nodes.push({
            id: `group:${group.id}`,
            kind: "group",
            stepId: step.id,
            name: group.name,
            bandId: group.id,
            line: group.tasks.map((task) => task.id).join(" "),
            traits: [],
            meta: [],
            life: group.life,
            tasks: group.tasks,
            ordinal: nodes.length + 1,
          });
        }
      }
    }
    if (gateIsANode(step)) {
      putGate(step, gate, tuned);
    }
    // The wave's Jobs, after the step that dispatches them and its gate.
    if (step.id === dispatchesFrom) {
      if (life?.jobs === undefined || life.jobs.length === 0) {
        put({ id: "jobs", kind: "jobs", name: "Jobs", traits: [], meta: [], inert: true });
      } else {
        const depths = depthsOf(life.jobs);
        const ordered = [...life.jobs].sort((a, b) => depths.get(a.id)! - depths.get(b.id)!);
        for (const job of ordered) {
          const depth = depths.get(job.id)!;
          const peers = ordered.filter((one) => depths.get(one.id) === depth);
          nodes.push({
            id: `job:${job.id}`,
            kind: "job",
            name: job.name,
            traits: [],
            meta: [],
            life: job.life,
            opensJob: job.id,
            band: { depth, index: peers.indexOf(job), of: peers.length },
            waits_on: job.waits_on.filter((one) => life.jobs!.some((other) => other.id === one)).map((one) => `job:${one}`),
            ordinal: nodes.length + 1,
          });
        }
      }
    }
  }
  /** A step's gate, as a node of its own. */
  function putGate(step: StepRead, gate: GateView | undefined, tuned: ApprovalTuning["steps"][string] | undefined) {
    const judges = tuned?.judges ?? 1;
    const declared = step.judges[0]?.panel_size ?? 1;
    // Who decides, in order: the gate's title. What was tuned on it: its meta.
    const who = [gate?.checks === true ? "Checks" : "", gate?.judge === true ? "Judge" : "", gate?.you === true ? "You" : ""]
      .filter((one) => one !== "")
      .join(" · ");
    const meta: RunNodeTrait[] = [];
    if (gate?.judge === true) meta.push({ key: "Judges", value: `judges ${judges}`, tuned: judges !== declared });
    const off = tuned?.checks_off.length ?? 0;
    if (off > 0) meta.push({ key: "Checks this Job does not run", value: `${off} off`, tuned: true });
    put({
      id: `${step.id}:checks`,
      kind: "checks",
      stepId: step.id,
      name: "Checks",
      ...(who === "" ? {} : { face: who }),
      bandId: step.id,
      traits: [],
      meta,
    });
  }
  // A workflow with no step that delivers still opens its pull request last.
  if (delivery !== "local" && !opened) pr();
  if (delivery === "local") done();
  put({
    id: "land",
    kind: "land",
    name: "Land",
    ...(target === "" ? {} : { face: target, faceMono: true }),
    traits: [],
    meta: [
      {
        key: "Merge",
        value: delivery === "local" ? "local merge" : tuning.auto_merge ? "merges on its own" : "you merge",
        tuned: delivery === "local" || tuning.auto_merge,
      },
    ],
  });

  const edges: WorkflowCanvasEdge[] = [];
  const lead = (source: ApprovalNode, target: ApprovalNode) => {
    const own = target.kind === "land" && delivery !== "local" && tuning.auto_merge;
    edges.push({
      id: `${source.id}->${target.id}`,
      source: source.id,
      target: target.id,
      kind: "leads",
      ...(own ? { label: "merges on its own" } : {}),
    });
  };
  // One after another, except the wave's Jobs: the node before them leads to
  // each that waits on nothing, each leads to the ones waiting on it, and each
  // nobody waits on leads to the node after them.
  for (const [at, node] of nodes.entries()) {
    if (at === 0) continue;
    const before = nodes[at - 1]!;
    if (node.kind === "job") {
      const waits = node.waits_on ?? [];
      if (waits.length === 0) {
        const head = nodes.slice(0, at).reverse().find((one) => one.kind !== "job");
        if (head !== undefined) lead(head, node);
      }
      for (const id of waits) {
        const on = nodes.find((one) => one.id === id);
        if (on !== undefined) lead(on, node);
      }
      continue;
    }
    if (before.kind === "job") {
      const wave = nodes.filter((one) => one.kind === "job");
      for (const job of wave) {
        if (!wave.some((other) => other.waits_on?.includes(job.id) === true)) lead(job, node);
      }
      continue;
    }
    lead(before, node);
  }
  return { nodes, edges };
}

/** The declared Checks of a step by name, each with whether this Job runs it. */
export function checksOf(step: StepRead, off: readonly string[]): { name: string; runs: boolean }[] {
  return step.checks.map((check) => {
    const name = checkNameOf(check);
    return { name, runs: !off.includes(name) };
  });
}
