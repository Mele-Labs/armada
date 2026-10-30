// A Job's lanes, read off what Fleet serves of it: the lifecycle definitions and instances `get_job_lifecycles` carries, the workflow the record names,
// and the sentences `workflow_waiting_on` words. One direction and no state: wire in, a `SessionLanes` out (`draft/sessions.ts`).
//
// **A Job that holds no instance has no lanes**, so its Workflow view is not offered: an empty place stays empty, and nothing is drawn for it.
// **The wire has no columns.** A step is placed by its order and a state by the longest run of expected moves before it, each at a column of its own in its
// lane, so a lane reads left to right and the layout does not move when an instance does.

import type { HeldInstance, InstanceMove, JobLifecycles, LifecycleDefinition } from "@armada/protocol";
import type { SessionLanes } from "@armada/components/src/compositions/LaneCanvas/build";
import type { Instance, JobLanes, LaneStep, Lifecycle, LifecycleKind, LifecycleState } from "@armada/components/src/compositions/LaneCanvas/model";

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
  };
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
  fleet: { kind: "job", said: "Armada" },
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
    ...(held.standing.kind === "unmapped" ? { title: `Unmapped: ${held.standing.said}` } : {}),
  };
}

/** The lanes a Job draws, or none where it holds no instance. */
export function lanesOfJob(input: LaneInput): SessionLanes | undefined {
  const { held, workflow } = input;
  if (held.instances.length === 0) return undefined;
  const steps = (workflow?.steps ?? []).map((step, col): LaneStep => ({ id: step.id, label: step.label, col }));
  const waiting = input.waiting === undefined || input.waiting.length === 0 ? undefined : input.waiting;
  const job: JobLanes = {
    workspace: "",
    copy: { steps, lifecycles: held.lifecycles.map(lifecycleOf), rules: [] },
    instances: held.instances.map((one) => instanceOf(one, held.moves)),
    fired: [],
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
