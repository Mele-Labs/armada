// What a Job at its dispatch gate will do, as nodes top to bottom: the brief,
// where the work starts, the workflow starting, each step and what gates it,
// the pull request, and where it lands. The approval canvas, prototype.
//
// **The order is the run's.** A step's gate is a stage on the spine between it
// and the next step, because the next step cannot start until it passes
// (the owner, 5 Oct 2026): `Plan → Checks → Judge → Implement`, one stage per
// kind of gate the step has, each with a face of its own. A step with no gate
// has no stage. A gate that sends the step back draws the way back on an edge.
//
// **What lands reshapes the end.** Local delivery has no pull request to open,
// so its node goes and Land names the branch the work stays on — local only
// is no pull request, no merge and no push (the owner, 4 Oct 2026); auto-merge
// says on the edge into Land that it merges on its own. One field each, read here.

import { AUTO, STEP_STATE } from "@armada/components";
import type { GateCommandOutcome, PanelMark, RunNodeGate, RunNodeTrait, StepActivity, WorkflowCanvasEdge } from "@armada/components";
import type { LucideIcon } from "lucide-react";
import type { DeclaredCheck, DeclaredJudge, JobDetail as JobWhole, StepDetail, StepPass, StepPhase, WorkflowStep } from "@armada/protocol";

import type { GateView } from "./draft/proposal";
import type { TaskView } from "./draft/task";
import { checkNameOf, deliveryOf } from "./draft/tuning";
import type { ApprovalTuning } from "./draft/tuning";

export type ApprovalNodeKind =
  | "studio"
  | "brief"
  | "base"
  | "step"
  | "checks"
  | "groups"
  | "group"
  | "jobs"
  | "job"
  | "done"
  | "pr"
  | "land";

/** The kinds of gate a step can have, in the order they run. */
export type GateKind = "checks" | "judge" | "you";

/** What a running gate stage has done so far: what Fleet serves of it, and nothing more. */
export type GateRun = {
  /** Each command's outcome, by name. */
  commands?: readonly { name: string; outcome: GateCommandOutcome }[];
  /** How long the Checks have run. */
  elapsed?: string;
  /** The last line a live command printed. */
  output?: string;
  /** The panel as it stands: one mark per judge. */
  panel?: readonly PanelMark[];
  /** A refusal's first line. */
  refusal?: string;
  /** How long a person has been waited on. */
  waited?: string;
};

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
  /** A gate stage's run so far. */
  run?: GateRun;
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
  /**
   * Where each step has got to, by its id, in the Workflow card's own words —
   * `285h 45m · 1 Drone`. Drawn on the node the Job is at.
   */
  lines?: Readonly<Record<string, string>>;
  /** The settings a person moved since the approval, each drawn on the nodes it governs. */
  changed?: ChangedSince;
};

/**
 * What differs on a running Job from how it was approved — the Settings
 * tab's own reading (`changedOf`), each with what it was. **What it was is
 * named, never drawn**: the node shows the value now, in accent, and its
 * tooltip says the earlier one (the owner, 4 Oct 2026).
 */
export type ChangedSince = {
  /** The model every later Drone is spawned on. */
  model?: string;
  /** The model the review step runs. */
  reviewModel?: string;
  /** How a Drone meets a command it was not given, in the Settings tab's words, and what it was. */
  whenBlocked?: { now: string; was: string };
  /** What a refusing Judge does, likewise. */
  whenRefused?: { now: string; was: string };
  /** The commands allowed on this Job, whole. */
  allowed?: readonly string[];
};

/** One step of the chosen workflow, as the canvas reads it. */
export type StepRead = {
  id: string;
  label: string;
  checks: readonly DeclaredCheck[];
  judges: readonly DeclaredJudge[];
  delivers: boolean;
  /** Which lane it is drawn in: `phase`, which Fleet sends on every step (23.19). */
  phase: StepPhase;
  /** Whether a Drone works each of the plan's tasks on it: `drone_per_task`. */
  perTask: boolean;
  /** Whether its Drone creates the wave's Jobs: `may_dispatch_jobs`. */
  dispatches: boolean;
  /** Where its gate sends the work back to, and the pass it is on: `verdict_routing_target`, `pass`. */
  returns?: { to: string; pass?: StepPass };
};

/** The three phases the run is laid out in, left to right (the owner, 4 Oct 2026). */
export type Lane = "setup" | "work" | "delivery";

export const LANES: readonly Lane[] = ["setup", "work", "delivery"];

export type ApprovalNode = {
  id: string;
  kind: ApprovalNodeKind;
  lane: Lane;
  /** A fanned group or Job, under the node it falls from, by that node's id. Off the spine. */
  from?: string;
  /** The step it belongs to, on a step or its gate. */
  stepId?: string;
  /** Which kind of gate a `checks` node is the stage for. */
  gate?: GateKind;
  /** A gate stage's face. */
  gateFace?: RunNodeGate;
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
  /** The body's second line as values: a step's model and effort. */
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
  /** The Studio the Job came from: a press opens it there, its node picked (#1674). */
  opensStudio?: true;
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

/** A setting left to Armada, in the word the tier map uses for it. */
const AUTO_WORD = AUTO;

/** What a You stage says at the gate, before it is asking anything. */
const I_REVIEW = "I review it";

/**
 * The steps of the chosen workflow in the order the gates hold them. **The
 * frozen step first, then the workflow's** — `gateRowsOf`'s rule, so a newly
 * picked workflow reads its own declarations.
 */
export function stepsReadOf(
  gates: readonly GateView[],
  whole: JobWhole | null,
  declared: ReadonlyMap<string, WorkflowStep>,
  /** Past the gate the Job's own frozen step is the truth, and the catalog's only fills in what it lacks. */
  frozenFirst = false,
): StepRead[] {
  return gates.map((gate) => {
    const frozen = whole?.steps.find((one) => one.step_id === gate.step_id);
    const step: WorkflowStep | StepDetail | undefined = frozenFirst ? (frozen ?? declared.get(gate.step_id)) : (declared.get(gate.step_id) ?? frozen);
    return {
      id: gate.step_id,
      label: step?.label ?? gate.step_id,
      checks: step?.checks ?? [],
      judges: step?.judge_checks ?? [],
      delivers: step?.delivers ?? false,
      phase: step?.phase ?? "work",
      // The Job's own step where it holds one, else what the picked workflow declares.
      perTask: (frozen?.drone_per_task ?? declared.get(gate.step_id)?.drone_per_task) === true,
      dispatches: step?.may_dispatch_jobs === true,
      ...(frozen?.verdict_routing_target === undefined
        ? {}
        : { returns: { to: frozen.verdict_routing_target, ...(frozen.pass === undefined ? {} : { pass: frozen.pass }) } }),
    };
  });
}

/** Whether a Drone works each of the plan's tasks on the step. */
export const perTask = (step: StepRead): boolean => step.perTask;

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

/** Whether a step declares Checks or a Judge. */
export const gateIsANode = (step: StepRead): boolean => step.checks.length > 0 || step.judges.length > 0;

/** The stages a step's gate is, in the order they run: each kind it declares, and You where it stops for a person. */
export function gateKindsOf(step: StepRead, gate: GateView | undefined): GateKind[] {
  return [
    ...(step.checks.length > 0 ? (["checks"] as const) : []),
    ...(step.judges.length > 0 ? (["judge"] as const) : []),
    ...(gate?.you === true ? (["you"] as const) : []),
  ];
}

/** What each delivery is called on the pull request's face. */
const PR_FACE = { draft: "Draft pull request", ready: "Pull request" } as const;

/** The values moved off their defaults, and nothing else. */
const movedOnly = (values: readonly RunNodeTrait[]): RunNodeTrait[] => values.filter((one) => one.tuned === true);

export function approvalNodesOf({
  studio,
  title,
  asked = "",
  criteria = [],
  from,
  steps,
  gates,
  tuning,
  prMode,
  local = false,
  autoMerge = false,
  target,
  branch,
  life,
  droneCap,
  dispatchesFrom,
}: {
  /** The Studio the Job was dispatched from, by its name. Absent draws no node. */
  studio?: string;
  title: string;
  /** The request, the brief's second line. */
  asked?: string;
  /** What counts as done, in order. */
  criteria?: readonly string[];
  from: string;
  steps: readonly StepRead[];
  gates: readonly GateView[];
  tuning: ApprovalTuning;
  prMode: "ready" | "draft";
  /** The local-only delivery field: the work stays on its branch. */
  local?: boolean;
  /** `LandingRule.auto_merge`: the pull request merges on its own. */
  autoMerge?: boolean;
  target: string;
  /** The Job's own branch, once it has one: where local-only work stays. */
  branch?: string;
  life?: LifeRead;
  /** How many Drones the Job may run at once. Absent is the machine's. */
  droneCap?: number;
  /** The step the wave's Jobs are dispatched from, where the workflow dispatches any. */
  dispatchesFrom?: string;
}): { nodes: ApprovalNode[]; edges: WorkflowCanvasEdge[] } {
  const nodes: ApprovalNode[] = [];
  // Which lane the next node goes in: setup, then the work, then from Done when on, delivery.
  let lane: Lane = "setup";
  const put = (node: Omit<ApprovalNode, "ordinal" | "lane">) => {
    const at = life?.nodes[node.id];
    nodes.push({ ...node, lane, ...(at === undefined ? {} : { life: at }), ordinal: nodes.length + 1 });
  };
  const delivery = deliveryOf(local, prMode);

  // Where the work came from, first, and only where it came from a Studio.
  if (studio !== undefined) {
    put({ id: "studio", kind: "studio", name: "Studio", face: studio, opensStudio: true, traits: [], meta: [] });
  }
  put({ id: "brief", kind: "brief", name: "Brief", face: title, ...(asked === "" ? {} : { line: asked }), traits: [], meta: [] });
  put({
    id: "base",
    kind: "base",
    name: "Base branch",
    ...(from === "" ? {} : { face: from, faceMono: true }),
    traits: [],
    meta: [],
  });
  // No Start node: the Work lane's head names the workflow, and at the gate picks it.
  lane = "work";

  // What counts as the work being done, read just before it leaves (the
  // owner, 4 Oct 2026). The same criteria Brief's card edits.
  const done = () => {
    lane = "delivery";
    put({ id: "done", kind: "done", name: "Done when", ...(criteria.length === 0 ? {} : { items: criteria }), traits: [], meta: [] });
  };
  const pr = () => {
    put({
      id: "pr",
      kind: "pr",
      name: "Pull request",
      face: PR_FACE[delivery === "draft" ? "draft" : "ready"],
      traits: [],
      // Only what moved off the default: a draft, auto-merge. A ready pull request you merge says nothing.
      meta: movedOnly([
        { key: "Pull request", value: "draft", tuned: delivery === "draft" },
        { key: "Auto-merge", value: "auto-merge", tuned: autoMerge },
      ]),
    });
  };
  let delivering = false;
  for (const step of steps) {
    const gate = gates.find((one) => one.step_id === step.id);
    const tuned = tuning.steps[step.id];
    // What counts as done, then the pull request, before the step that sends
    // the work out — which is where a person reviews it.
    if (step.phase === "delivery" && !delivering) {
      done();
      if (delivery !== "local") pr();
      delivering = true;
    }
    // Its own lane, by its phase. Once the work is delivering, what follows delivers too.
    lane = delivering ? "delivery" : step.phase === "setup" ? "setup" : "work";
    const ownGate = !gateIsANode(step) && gate?.you === true;
    // A step a person alone reads runs no Drone, so it carries its gate and nothing a Drone is tuned by.
    // Where the step the Job is at has got to: the Workflow card's own line.
    const lineNow = life?.nodes[step.id]?.current === true ? life.lines?.[step.id] : undefined;
    // A setting moved since the approval governs what has not run yet, and a Drone step only.
    const ahead = life !== undefined && life.nodes[step.id]?.activity !== "advanced";
    const since = ahead && !ownGate ? life.changed : undefined;
    put({
      id: step.id,
      kind: "step",
      stepId: step.id,
      name: step.label,
      bandId: step.id,
      ...(lineNow === undefined ? {} : { line: lineNow }),
      // A Drone's settings left to Armada draw nothing (default to no text); what was tuned shows.
      traits: ownGate
        ? []
        : movedOnly([
            since?.model !== undefined
              ? { key: `Model, was ${tuned?.model ?? AUTO_WORD}`, value: since.model, tuned: true }
              : { key: "Model", value: tuned?.model ?? "", tuned: tuned?.model != null },
            { key: "Effort", value: tuned?.effort ?? "", tuned: tuned?.effort != null },
          ]),
      meta: movedOnly([
        ...(perTask(step) ? [{ key: "Drones at once", value: `${droneCap ?? ""} drones`, tuned: droneCap !== undefined }] : []),
        ...(since?.whenBlocked === undefined
          ? []
          : [{ key: `When blocked, was ${since.whenBlocked.was}`, value: since.whenBlocked.now, tuned: true }]),
        ...(since?.allowed === undefined || since.allowed.length === 0
          ? []
          : [{ key: `Allowed since approval: ${since.allowed.join(", ")}. Was none`, value: `${since.allowed.length} allowed`, tuned: true }]),
        ...(step.delivers && since?.reviewModel !== undefined
          ? [{ key: `Review model, was ${AUTO_WORD}`, value: since.reviewModel, tuned: true }]
          : []),
      ]),
    });
    // The plan's groups, worked by this step: a placeholder until Plan has
    // recorded them, and each group once it has.
    if (perTask(step)) {
      if (life?.groups === undefined || life.groups.length === 0) {
        put({ id: "groups", kind: "groups", name: "Plan", line: "Groups drawn once the plan exists", traits: [], meta: [], inert: true });
      } else {
        for (const [at, group] of life.groups.entries()) {
          nodes.push({
            id: `group:${group.id}`,
            kind: "group",
            lane,
            from: step.id,
            band: { depth: 0, index: at, of: life.groups.length },
            waits_on: [],
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
    // Its gate: the next stage on the spine, one for each kind.
    for (const kind of gateKindsOf(step, gate)) putGate(step, gate, tuned, kind);
    // The wave's Jobs, after the step that dispatches them and its gate.
    if (step.id === dispatchesFrom) {
      // Under its last gate stage: the wave is what the step's gate let through.
      const under = [...nodes].reverse().find((one) => one.stepId === step.id && one.from === undefined)?.id ?? step.id;
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
            lane,
            from: under,
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
  /** One stage of a step's gate, with the face its kind has. */
  function putGate(step: StepRead, gate: GateView | undefined, tuned: ApprovalTuning["steps"][string] | undefined, kind: GateKind) {
    const id = `${step.id}:${kind}`;
    const run = life?.nodes[id]?.run;
    // A stage the Job is past, with nothing served of it, passed whole.
    const past = life?.nodes[id]?.activity === "advanced" && run === undefined;
    const meta: RunNodeTrait[] = [];
    let gateFace: RunNodeGate;
    if (kind === "checks") {
      const off = tuned?.checks_off ?? [];
      if (gate?.checks !== true) meta.push({ key: "This Job does not run these Checks", value: "off", tuned: true });
      else if (off.length > 0) meta.push({ key: "Checks this Job does not run", value: `${off.length} off`, tuned: true });
      gateFace = {
        kind,
        commands: checksOf(step, off).map((one) => ({
          name: one.name,
          outcome:
            !one.runs || gate?.checks !== true
              ? "off"
              : (run?.commands?.find((had) => had.name === one.name)?.outcome ?? (past ? "passed" : "waiting")),
        })),
        ...(run?.elapsed === undefined ? {} : { elapsed: run.elapsed }),
        ...(run?.output === undefined ? {} : { output: run.output }),
      };
    } else if (kind === "judge") {
      const judges = tuned?.judges ?? 1;
      const declared = step.judges[0]?.panel_size ?? 1;
      if (gate?.judge !== true) meta.push({ key: "This Job has no Judge here", value: "off", tuned: true });
      else if (judges !== declared) meta.push({ key: "Judges", value: `${judges} judges`, tuned: true });
      const refused = life !== undefined && life.nodes[step.id]?.activity !== "advanced" ? life.changed?.whenRefused : undefined;
      if (gate?.judge === true && refused !== undefined) {
        meta.push({ key: `When the Judge refuses, was ${refused.was}`, value: refused.now, tuned: true });
      }
      gateFace = {
        kind,
        panel: gate?.judge !== true ? [] : (run?.panel ?? Array.from({ length: judges }, (): PanelMark => (past ? "met" : "pending"))),
        ...(run?.refusal === undefined ? {} : { refusal: run.refusal }),
      };
    } else {
      gateFace = {
        kind,
        asking: run?.waited === undefined ? I_REVIEW : step.label,
        ...(run?.waited === undefined ? {} : { waited: run.waited }),
      };
    }
    put({
      id,
      kind: "checks",
      stepId: step.id,
      gate: kind,
      gateFace,
      name: kind === "checks" ? "Checks" : kind === "judge" ? "Judge" : "You",
      bandId: step.id,
      traits: [],
      meta,
    });
  }
  // A workflow with no step that delivers still says what counts as done, and opens its pull request, last.
  if (!delivering) {
    done();
    if (delivery !== "local") pr();
  }
  put({
    id: "land",
    kind: "land",
    name: "Land",
    // Local only: the branch the work stays on, where the Job has one yet. Else where it lands.
    ...(delivery === "local"
      ? branch === undefined
        ? {}
        : { face: branch, faceMono: true }
      : target === ""
        ? {}
        : { face: target, faceMono: true }),
    traits: [],
    meta: movedOnly([
      { key: "No pull request, no merge, no push: the work stays on its branch", value: "local only", tuned: delivery === "local" },
      { key: "Merge", value: "merges on its own", tuned: delivery !== "local" && autoMerge },
    ]),
  });

  const edges: WorkflowCanvasEdge[] = [];
  const lead = (source: ApprovalNode, target: ApprovalNode) => {
    const own = target.kind === "land" && delivery !== "local" && autoMerge;
    edges.push({
      id: `${source.id}->${target.id}`,
      source: source.id,
      target: target.id,
      kind: "leads",
      ...(own ? { label: "merges on its own" } : {}),
    });
  };
  // The spine runs node to node within a lane, and across a lane once, from
  // the bottom of one lane's last to the top of the next one's first, a gate's
  // stages between its step and the next. A fan leads from the node it falls
  // from to each of its members that waits on nothing, member to member where
  // one waits, and from each nobody waits on to the next node on the spine.
  const byId = new Map(nodes.map((one) => [one.id, one]));
  const spine = nodes.filter((one) => one.from === undefined);
  for (const node of nodes) {
    if (node.from !== undefined) {
      const waits = node.waits_on ?? [];
      if (waits.length === 0) {
        const head = byId.get(node.from);
        if (head !== undefined) lead(head, node);
      }
      for (const id of waits) {
        const on = byId.get(id);
        if (on !== undefined) lead(on, node);
      }
    }
  }
  // A gate that sends the step back: the way back from its last stage, with the pass it is on.
  for (const step of steps) {
    if (step.returns === undefined) continue;
    const last = [...nodes].reverse().find((one) => one.kind === "checks" && one.stepId === step.id);
    const to = byId.get(step.returns.to);
    if (last === undefined || to === undefined) continue;
    edges.push({
      id: `${last.id}->${to.id}`,
      source: last.id,
      target: to.id,
      kind: "returns",
      ...(step.returns.pass === undefined ? {} : { label: `attempt ${step.returns.pass.number} of ${step.returns.pass.of}` }),
    });
  }
  for (const [at, node] of spine.entries()) {
    if (at === 0) continue;
    const before = spine[at - 1]!;
    const fan = nodes.filter((one) => one.from === before.id);
    if (fan.length === 0) {
      lead(before, node);
      continue;
    }
    for (const member of fan) {
      if (!fan.some((other) => other.waits_on?.includes(member.id) === true)) lead(member, node);
    }
  }
  return { nodes, edges };
}

/**
 * The edge the Job is moving along: the one into the node it is at. **On a
 * retry that is the way back**, not the forward edge into the step, which was
 * passed this attempt before it was entered by the loop.
 */
export function flowingOf(
  nodes: readonly ApprovalNode[],
  edges: readonly WorkflowCanvasEdge[],
  steps: readonly StepRead[],
): WorkflowCanvasEdge[] {
  return edges.map((edge) => {
    const into = nodes.find((node) => node.id === edge.target);
    if (into?.life?.current !== true) return edge;
    const retrying = into.kind === "step" && (steps.find((one) => one.id === into.stepId)?.returns?.pass?.number ?? 1) > 1;
    // Only one way into a step moves: the loop on a retry, the forward edge otherwise.
    const loop = edge.kind === "returns";
    return into.kind === "step" && loop !== retrying ? edge : { ...edge, flowing: true };
  });
}

/** The declared Checks of a step by name, each with whether this Job runs it. */
export function checksOf(step: StepRead, off: readonly string[]): { name: string; runs: boolean }[] {
  return step.checks.map((check) => {
    const name = checkNameOf(check);
    return { name, runs: !off.includes(name) };
  });
}
