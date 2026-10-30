// A Job's lanes, read off what Fleet serves of it: the lifecycle definitions and instances `get_job_lifecycles` carries, the workflow the record names,
// and the sentences `workflow_waiting_on` words. One direction and no state: wire in, a `SessionLanes` out (`draft/sessions.ts`).
//
// **A Job that holds no instance has no lanes**, so its Workflow view is not offered: an empty place stays empty, and nothing is drawn for it.
// **The wire has no columns.** A step is placed by its order and a state by the longest run of expected moves before it, each at a column of its own in its
// lane, so a lane reads left to right and the layout does not move when an instance does.

import type { AppliedRule, HeldInstance, InstanceMove, JobLifecycles, LifecycleDefinition, LifecycleRule, RuleActReading, RuleTarget } from "@armada/protocol";
import type { SessionLanes } from "@armada/components/src/compositions/LaneCanvas/build";
import { CURRENT_STEP } from "@armada/components/src/compositions/LaneCanvas/model";
import type { Autonomy, Fired, FiredAct, Instance, JobLanes, Kept, LaneStep, Lifecycle, LifecycleKind, LifecycleState, Rule } from "@armada/components/src/compositions/LaneCanvas/model";

/** What the lanes are read from. */
export type LaneInput = {
  /** `get_job_lifecycles`, or the last `job_lifecycles.changed`. */
  held: JobLifecycles;
  /** The workflow the Job follows and where it stands, as the rail reads it. Absent for a Job with none: its lanes have no top lane. */
  workflow?: {
    name: string;
    steps: readonly { id: string; label: string }[];
    /** The step the Job is on. Absent where Fleet names none. */
    at?: number;
    /** Whether any step has begun, so a Job past its last step reads as done and not as not begun. */
    began: boolean;
    skipped?: readonly number[];
    /** When each step began, as the record has it, so an act is placed on the step the Job was on when it was planned. */
    visits?: readonly { step: number; since: string }[];
  };
  /** The rules that apply to the Job's repository (`list_lifecycle_rules`). Without them the acts that were planned have no edge to be drawn on. */
  rules?: readonly AppliedRule[];
  /** The agent is working on its step now. */
  working: boolean;
  /** `SessionRecord.workflow_waiting_on`. */
  waiting?: readonly string[];
};

/** The id an instance is drawn under: its lifecycle's, and its key as its source calls it. A key can hold a slash, which a node id cannot. */
export const instanceId = (lifecycle: string, key: string): string => `${lifecycle}:${encodeURIComponent(key)}`;

/** Where a lifecycle's states are read from, in a word under the lane's title. */
const SOURCE: Readonly<Record<string, { kind: LifecycleKind; said: string }>> = {
  tracker: { kind: "tracker", said: "Tracker" },
  forge: { kind: "forge", said: "Forge" },
  watch: { kind: "watch", said: "Watch" },
  fleet: { kind: "job", said: "Fleet" },
};

/**
 * The column of each state: after the states that lead to it, and never beside another in its lane. States are taken in the order the definition lists them, so a
 * move back to an earlier state leads nowhere new, and one nothing leads to follows the states before it.
 */
export function columnsOf(lifecycle: Pick<LifecycleDefinition, "states" | "transitions">): ReadonlyMap<string, number> {
  const order = new Map(lifecycle.states.map((state, at) => [state.id, at] as const));
  const cols = new Map<string, number>();
  const taken = new Set<number>();
  let last = -1;
  for (const [at, state] of lifecycle.states.entries()) {
    const behind = lifecycle.transitions.flatMap((move) => (move.to === state.id && (order.get(move.from) ?? at) < at ? [cols.get(move.from)!] : []));
    let col = behind.length > 0 ? Math.max(...behind) + 1 : at === 0 ? 0 : last + 1;
    while (taken.has(col)) col += 1;
    taken.add(col);
    cols.set(state.id, col);
    last = Math.max(last, col);
  }
  return cols;
}

function lifecycleOf(wire: LifecycleDefinition): Lifecycle {
  const cols = columnsOf(wire);
  const from = SOURCE[wire.states[0]?.read.source ?? "tracker"] ?? SOURCE["tracker"]!;
  const states = wire.states.map((state): LifecycleState => ({ id: state.id, label: state.label, reads: [], col: cols.get(state.id)!, ...(wire.ends.includes(state.id) ? { end: true as const } : {}) }));
  return { id: wire.id, label: wire.name, from: from.said, kind: from.kind, states, next: wire.transitions.map((move) => [move.from, move.to] as const) };
}

/** The states an instance stood in, in the order it moved through them, the one it is in last. */
function visitedBy(held: HeldInstance, moves: readonly InstanceMove[]): string[] {
  const mine = moves.filter((move) => move.lifecycle === held.lifecycle && move.key === held.key);
  const seen: string[] = [];
  const note = (state: string) => {
    if (!seen.includes(state)) seen.push(state);
  };
  const first = mine[0]?.from;
  if (first?.kind === "in") note(first.state);
  for (const move of mine) if (move.to.kind === "in") note(move.to.state);
  if (held.standing.kind === "in") note(held.standing.state);
  return seen;
}

function instanceOf(held: HeldInstance, moves: readonly InstanceMove[]): Instance {
  return {
    id: instanceId(held.lifecycle, held.key),
    lifecycle: held.lifecycle,
    label: held.label,
    state: held.standing.kind === "in" ? held.standing.state : "",
    visited: visitedBy(held, moves),
    ...(held.standing.kind === "unmapped" ? { title: `Unmapped: ${held.standing.said}`, unmapped: held.standing.said } : {}),
    ...(held.ended_at === undefined ? {} : { ended: true as const }),
  };
}

// ---- Rules and what they did -----------------------------------------------------------------------------------------------

/** The lifecycle a rule's target is of: the one named, or the one whose entering set the rule off. */
function laneOfTarget(target: RuleTarget, rule: LifecycleRule): string | undefined {
  return target.among.kind === "entered" ? (rule.when.kind === "enters" ? rule.when.lifecycle : undefined) : target.among.lifecycle;
}

const KEPT: Readonly<Record<string, Kept>> = { armada: "armada", kit: "yours", repository: "repository" };

/** The strongest word among a rule's acts: any that asks makes it Asks, and all of them off make it Off. */
function autonomyOf(rule: LifecycleRule): Autonomy {
  const words = rule.autonomy === undefined ? rule.does.map((one) => one.autonomy) : [rule.autonomy];
  if (words.includes("asks")) return "asks";
  return words.length > 0 && words.every((one) => one === "off") ? "off" : "acts";
}

/** The lifecycle a watch's `deploy` names an instance of. */
const DEPLOY = "deploy";

/**
 * A rule as a Job's lanes draw it: an edge from where it fired to where it lands. **Where it lands is what the rule already says**, in this order: the state a `tracker_move`
 * moves a ticket to; the first state of the deploy a `start_watch` takes on, `landsOn` being that deploy; the step an `at_step` condition names, before and after it fires;
 * and where none of these names a place, the step the Job was on when it fired. A rule a step sets off leaves from that step. A rule a Job event sets off (`started`,
 * `landed`, `ended`) has no node to leave from, and is not drawn.
 */
function readRule(applied: AppliedRule, lifecycles: readonly LifecycleDefinition[]): { rule: Rule; landsOn?: string } | undefined {
  const { rule, kept } = applied;
  const when = rule.when;
  if (when.kind === "job") return undefined;
  const from = when.kind === "enters" ? { lane: when.lifecycle, item: when.state } : { lane: "workflow", item: when.step };
  const move = rule.does.flatMap((one) => (one.what.act === "tracker_move" ? [{ lane: laneOfTarget(one.what.target, rule), item: one.what.to }] : []))[0];
  const deploy = rule.does.flatMap((one) => (one.what.act === "start_watch" && one.what.deploy !== undefined && one.what.deploy !== "" ? [one.what.deploy] : []))[0];
  const atStep = rule.conditions.flatMap((one) => (one.kind === "at_step" ? [one.step] : []))[0];
  const first = lifecycles.find((one) => one.id === DEPLOY)?.states[0]?.id;
  // A deploy's lane has no state to land on until the deploy is held, so the edge waits for it.
  const to =
    move?.lane !== undefined
      ? { lane: move.lane, item: move.item }
      : deploy !== undefined
        ? { lane: DEPLOY, item: first ?? "" }
        : { lane: "workflow", item: atStep ?? CURRENT_STEP };
  return {
    rule: {
      id: rule.id,
      label: rule.name,
      from,
      ...(when.kind === "enters" ? { at: "enters" as const } : { at: when.kind === "step_ends" ? ("ends" as const) : ("starts" as const) }),
      to,
      ifs: [],
      does: [],
      autonomy: autonomyOf(rule),
      kept: KEPT[kept.rank.source] ?? "repository",
      scope: "default",
    },
    ...(deploy === undefined || move?.lane !== undefined ? {} : { landsOn: instanceId(DEPLOY, deploy) }),
  };
}

/** A rule as a Job's lanes draw it, or none where it has no node to leave from. `lifecycles` are the Job's, which say where a deploy's lane begins. */
export const ruleOfWire = (applied: AppliedRule, lifecycles: readonly LifecycleDefinition[] = []): Rule | undefined => readRule(applied, lifecycles)?.rule;

/** How an act stands, in the words a person reads. Done reads as the act itself. */
const STANDING: Readonly<Record<string, string>> = { planned: "Planned", asked: "Asked", stuck: "Stuck", skipped: "Skipped", told: "Told the agent", not_carried: "Not carried" };

/** A time of day, 24-hour, as the lanes' chips read it. */
const clock = (at: string): string => new Date(at).toLocaleTimeString("en-GB", { hour: "2-digit", minute: "2-digit" });

/** How the acts of one rule on one instance end together: any that waits on the person asks, any still to be done waits, and all of them set aside is set aside. */
function outcomeOf(acts: readonly RuleActReading[]): Fired["outcome"] {
  const standing = acts.map((one) => one.standing);
  if (standing.some((one) => one === "asked" || one === "stuck")) return "asked";
  if (standing.some((one) => one === "planned" || one === "not_carried")) return "waiting";
  return standing.every((one) => one === "skipped") ? "skipped" : "acted";
}

const sayAct = (one: RuleActReading): string => {
  const word = STANDING[one.standing];
  return word === undefined ? one.text : `${one.text} · ${word}${one.said === undefined || one.said === "" ? "" : `: ${one.said}`}`;
};

/** The index of the step the Job was on at `at`: the last one begun by then. */
function stepWhen(at: string, visits: readonly { step: number; since: string }[]): number | undefined {
  const by = Date.parse(at);
  return visits.filter((one) => Date.parse(one.since) <= by).at(-1)?.step;
}

/** What the Job's rules did, a line for each rule and instance it did it to. Only the rules that are drawn have one. */
function firedOf(
  acts: readonly RuleActReading[],
  rules: readonly Rule[],
  landsOn: ReadonlyMap<string, string>,
  steps: readonly LaneStep[],
  visits: readonly { step: number; since: string }[],
): Fired[] {
  const groups = new Map<string, { rule: Rule; acts: RuleActReading[]; instance: string }>();
  for (const act of acts) {
    const rule = rules.find((one) => one.id === act.rule);
    if (rule === undefined) continue;
    // An edge that lands on a state is keyed by the instance it was done to, or the deploy a watch takes on; one that hands to a step, by the instance that set it off, or by none where a step set it off.
    const link = rule.to.lane === "workflow" ? (rule.from.lane === "workflow" ? undefined : (act.entered ?? act.on)) : (act.on ?? act.entered);
    const instance = rule.to.lane !== "workflow" && act.on === undefined && landsOn.has(rule.id) ? landsOn.get(rule.id)! : link === undefined ? "" : instanceId(link.lifecycle, link.key);
    const group = groups.get(`${act.rule}\n${instance}`);
    if (group === undefined) groups.set(`${act.rule}\n${instance}`, { rule, acts: [act], instance });
    else group.acts.push(act);
  }
  return [...groups.values()].map(({ rule, acts: mine, instance }): Fired => {
    const last = mine[mine.length - 1]!;
    const at = stepWhen(mine[0]!.at, visits);
    const step = at === undefined ? undefined : steps[at]?.id;
    return {
      rule: rule.id,
      instance,
      outcome: outcomeOf(mine),
      at: clock(last.settled_at ?? last.at),
      did: mine.map(sayAct),
      ...(step === undefined ? {} : { step }),
      acts: mine.map((one): FiredAct => ({ text: one.text, course: one.course === "asks" ? "asks" : "acts", standing: one.standing, at: clock(one.settled_at ?? one.at), ...(one.said === undefined || one.said === "" ? {} : { said: one.said }) })),
    };
  });
}

/** The lanes a Job draws, or none where it holds no instance. */
export function lanesOfJob(input: LaneInput): SessionLanes | undefined {
  const { held, workflow } = input;
  if (held.instances.length === 0) return undefined;
  const steps = (workflow?.steps ?? []).map((step, col): LaneStep => ({ id: step.id, label: step.label, col }));
  const waiting = input.waiting === undefined || input.waiting.length === 0 ? undefined : input.waiting;
  const read = (input.rules ?? []).flatMap((one) => readRule(one, held.lifecycles) ?? []);
  const rules = read.map((one) => one.rule);
  const landsOn = new Map(read.flatMap((one) => (one.landsOn === undefined ? [] : [[one.rule.id, one.landsOn] as const])));
  const job: JobLanes = {
    workspace: "",
    copy: { steps, lifecycles: held.lifecycles.map(lifecycleOf), rules },
    instances: held.instances.map((one) => instanceOf(one, held.moves)),
    fired: firedOf(held.acts ?? [], rules, landsOn, steps, workflow?.visits ?? []),
    changes: [],
    ...(waiting === undefined ? {} : { waiting }),
  };
  // `at` is -1 before a step starts and the step count once every one has ended; Fleet names no step in either case.
  const at = workflow?.at ?? (workflow?.began === true ? steps.length : -1);
  return {
    workflow: workflow?.name ?? "",
    at,
    working: input.working && workflow?.at !== undefined,
    ...(workflow?.skipped === undefined || workflow.skipped.length === 0 ? {} : { skipped: workflow.skipped }),
    job,
  };
}
