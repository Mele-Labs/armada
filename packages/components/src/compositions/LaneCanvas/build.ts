// The lanes of a Workspace as the editor draws them, and of a Job as its Lanes view does. Both come to the same two lists, lanes
// and rule edges, which `LaneCanvas` places.

import { nodeId } from "./model";
import type { Instance, JobLanes, LaneNodeView, LaneStep, LaneView, Lifecycle, Rule, RuleView, Workspace } from "./model";

const laneOfLifecycle = (lifecycle: Lifecycle, id: string, title: string, note: string | undefined, nodes: readonly LaneNodeView[]): LaneView => ({
  id,
  kind: lifecycle.id,
  title,
  source: lifecycle.from,
  ...(note === undefined ? {} : { note }),
  nodes,
  moves: lifecycle.next.map(([from, to]) => [nodeId(id, from), nodeId(id, to)] as const),
});

/** A step's second line: the lifecycle state that ends it, when one does. */
function endsOnSaid(step: LaneStep, lifecycles: readonly Lifecycle[]): string | undefined {
  if (step.endsOn === undefined) return undefined;
  const lifecycle = lifecycles.find((one) => one.id === step.endsOn!.lane);
  const state = lifecycle?.states.find((one) => one.id === step.endsOn!.state);
  return `Ends on ${lifecycle?.label ?? step.endsOn.lane}: ${state?.label ?? step.endsOn.state}`;
}

/** The steps of a workflow, placed in columns where the Workspace says, and in order where it does not. */
export function stepsOf(workspace: Workspace, steps: readonly { id: string; label: string }[]): LaneStep[] {
  return steps.map((step, at) => {
    const given = workspace.steps[step.id];
    return { id: step.id, label: step.label, col: given?.col ?? at, ...(given?.endsOn === undefined ? {} : { endsOn: given.endsOn }) };
  });
}

/** Whether a rule applies in the scope the chip picked: the Workspace shows every rule, a repository the ones that apply there. */
export const appliesIn = (rule: Rule, scope: string): boolean => scope === "workspace" || rule.scope === "workspace" || rule.scope === scope;

/**
 * The editor's lanes: the workflow on top, then one lane for each lifecycle, and each rule in the scope as an edge. `proposed` are the
 * rules a change waiting on Approve would add, drawn as a diff.
 */
export function editorLanes(input: {
  workspace: Workspace;
  workflow: string;
  steps: readonly { id: string; label: string }[];
  scope: string;
  proposed?: readonly Rule[];
}): { lanes: LaneView[]; rules: RuleView[] } {
  const { workspace, workflow, scope } = input;
  const steps = stepsOf(workspace, input.steps);
  const flow: LaneView = {
    id: "workflow",
    kind: "workflow",
    title: workflow,
    source: "Workflow",
    nodes: steps.map((step) => {
      const sub = endsOnSaid(step, workspace.lifecycles);
      return { id: nodeId("workflow", step.id), label: step.label, ...(sub === undefined ? {} : { sub }), col: step.col, role: "step" as const };
    }),
    moves: steps.slice(1).map((step, at) => [nodeId("workflow", steps[at]!.id), nodeId("workflow", step.id)] as const),
  };
  const lifecycles = workspace.lifecycles.map((lifecycle) =>
    laneOfLifecycle(
      lifecycle,
      lifecycle.id,
      lifecycle.label,
      undefined,
      lifecycle.states.map((state) => ({ id: nodeId(lifecycle.id, state.id), label: state.label, col: state.col, role: "state" as const, ...(state.end === true ? { end: true as const } : {}) })),
    ),
  );
  const drawn = (rule: Rule, proposed: boolean): RuleView => ({
    id: rule.id,
    rule: rule.id,
    from: nodeId(rule.from.lane, rule.from.item),
    to: nodeId(rule.to.lane, rule.to.item),
    label: rule.label,
    autonomy: rule.autonomy,
    ...(proposed ? { proposed: true as const } : {}),
    ...(rule.labelAt === undefined ? {} : { labelAt: rule.labelAt }),
  });
  return {
    lanes: [flow, ...lifecycles],
    rules: [...workspace.rules.filter((rule) => appliesIn(rule, scope)).map((rule) => drawn(rule, false)), ...(input.proposed ?? []).map((rule) => drawn(rule, true))],
  };
}

// ---- A Job ---------------------------------------------------------------------------------------------------------------

/** Where a Job stands in its workflow, as the story reads it. */
export type JobStanding = { workflow: string; at: number; skipped?: readonly number[] };

/** What a Job's Lanes view draws: where the workflow stands (`at` is -1 before a step starts, the step count once all ended) and what the Job holds. */
export type SessionLanes = JobStanding & { job: JobLanes };

const instancesOf = (job: JobLanes, lane: string): readonly Instance[] => job.instances.filter((one) => one.lifecycle === lane);

/**
 * A Job's lanes: its workflow on top with the step it is at lit, then a lane for each instance it holds, the state it stands in lit and the
 * ones it went through marked. A rule that fired is an edge with its outcome and when; a rule still waiting is a dashed edge naming what it
 * waits for. A rule turned off is not drawn.
 */
export function jobLanes(job: JobLanes, standing: JobStanding): { lanes: LaneView[]; rules: RuleView[] } {
  const { copy } = job;
  const label = (lane: string, item: string): string => copy.lifecycles.find((one) => one.id === lane)?.states.find((one) => one.id === item)?.label ?? item;
  const flow: LaneView = {
    id: "workflow",
    kind: "workflow",
    title: standing.workflow,
    source: "Workflow",
    nodes: copy.steps.map((step, at) => {
      const sub = endsOnSaid(step, copy.lifecycles);
      const waiting = job.waitingOn?.step === step.id ? waitingSaid(job, copy.lifecycles) : undefined;
      return {
        id: nodeId("workflow", step.id),
        label: step.label,
        ...(sub === undefined ? {} : { sub }),
        col: step.col,
        role: "step" as const,
        ...(at === standing.at ? { lit: true as const } : {}),
        ...(at < standing.at ? { passed: true as const } : {}),
        ...(waiting === undefined ? {} : { waiting }),
        ...(standing.skipped?.includes(at) === true ? { skipped: true as const } : {}),
      };
    }),
    moves: copy.steps.slice(1).map((step, at) => [nodeId("workflow", copy.steps[at]!.id), nodeId("workflow", step.id)] as const),
  };
  const lanes: LaneView[] = [
    flow,
    ...job.instances.flatMap((instance): LaneView[] => {
      const lifecycle = copy.lifecycles.find((one) => one.id === instance.lifecycle);
      if (lifecycle === undefined) return [];
      const changed = job.changes.find((one) => one.lane === instance.id)?.text;
      const lane = laneOfLifecycle(
        lifecycle,
        instance.id,
        instance.label,
        instance.title,
        lifecycle.states.map((state) => ({
          id: nodeId(instance.id, state.id),
          label: state.label,
          col: state.col,
          role: "state" as const,
          ...(state.id === instance.state ? { lit: true as const } : {}),
          ...(state.id !== instance.state && instance.visited.includes(state.id) ? { passed: true as const } : {}),
          ...(state.end === true ? { end: true as const } : {}),
        })),
      );
      return [{ ...lane, ...(changed === undefined ? {} : { changed }) }];
    }),
  ];
  const stepAt = (id: string) => copy.steps.findIndex((one) => one.id === id);
  const rules: RuleView[] = [];
  for (const rule of copy.rules) {
    if (rule.autonomy === "off") continue;
    const sources = rule.from.lane === "workflow" ? [undefined] : instancesOf(job, rule.from.lane);
    const targets = rule.to.lane === "workflow" ? [undefined] : instancesOf(job, rule.to.lane);
    let first = true;
    for (const source of sources) {
      for (const target of targets) {
        const fired = job.fired.find((one) => one.rule === rule.id && one.instance === (target ?? source)?.id);
        const reached =
          source === undefined
            ? stepAt(rule.from.item) < standing.at || (rule.at !== "ends" && stepAt(rule.from.item) === standing.at)
            : source.visited.includes(rule.from.item);
        // A rule waits for its source only while it has not been reached: one reached and unfired has nothing left to show.
        if (fired === undefined && reached) continue;
        const status = fired === undefined ? ("waiting" as const) : fired.outcome;
        const name = source === undefined ? copy.steps.find((one) => one.id === rule.from.item)?.label ?? rule.from.item : label(rule.from.lane, rule.from.item);
        const waits = source === undefined ? `${name} starts` : `${source.label}: ${name}`;
        rules.push({
          id: `${rule.id}:${(source ?? target)?.id ?? "workflow"}${source !== undefined && target !== undefined ? `>${target.id}` : ""}`,
          rule: rule.id,
          from: nodeId(source?.id ?? "workflow", rule.from.item),
          to: nodeId(target?.id ?? "workflow", rule.to.item),
          label: rule.label,
          meta: fired === undefined ? waits : fired.at,
          autonomy: rule.autonomy,
          status,
          ...(first ? {} : { quiet: true as const }),
          ...(rule.labelAt === undefined ? {} : { labelAt: rule.labelAt }),
        });
        first = false;
      }
    }
  }
  return { lanes, rules };
}

/** What the rail says a step is waiting on: `Waiting on PR #812: Approved`. */
export function waitingSaid(job: JobLanes, lifecycles: readonly Lifecycle[]): string | undefined {
  const waiting = job.waitingOn;
  if (waiting === undefined) return undefined;
  const instance = job.instances.find((one) => one.id === waiting.instance);
  const state = lifecycles.find((one) => one.id === instance?.lifecycle)?.states.find((one) => one.id === waiting.state);
  return instance === undefined ? undefined : `Waiting on ${instance.label}: ${state?.label ?? waiting.state}`;
}

/** What is still in flight on a Job: each instance not in an end state, as `Deploy: rolling out`. */
export function inFlight(job: JobLanes): { id: string; label: string; state: string; lifecycle: Instance["lifecycle"] }[] {
  return job.instances.flatMap((instance) => {
    const state = job.copy.lifecycles.find((one) => one.id === instance.lifecycle)?.states.find((one) => one.id === instance.state);
    if (state === undefined || state.end === true) return [];
    return [{ id: instance.id, label: instance.label, state: state.label, lifecycle: instance.lifecycle }];
  });
}
