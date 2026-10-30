// The lanes of a Workspace as the editor draws them, and of a Job as its Lanes view does. Both come to the same two lists, lanes
// and rule edges, which `LaneCanvas` places.

import { DEFAULT, END_ITEM, START_ITEM, layersAt, lifecyclesAt, lifecyclesFor, nodeId, overviews, ruleAt, ruleDiffers, stateDiffers, stepDiffers } from "./model";
import type { Instance, JobLanes, LaneNodeView, LaneStep, LaneView, Lifecycle, LifecycleKind, NodeKind, Rule, RuleView, Workspace } from "./model";

/** The node a state of a lifecycle is drawn as, by what reads the lifecycle. */
const STATE_KIND: Readonly<Record<LifecycleKind, NodeKind>> = { tracker: "tracker", forge: "forge", watch: "watch", document: "document", job: "job" };
const kindOfState = (lifecycle: { kind?: LifecycleKind }): NodeKind => STATE_KIND[lifecycle.kind ?? "tracker"];

/** A step is a box, and one that ends on a state, so waits for it, is a hexagon. */
const kindOfStep = (step: LaneStep): NodeKind => (step.endsOn === undefined ? "step" : "wait");

/**
 * The workflow's lane with a small circle at each end: where it starts and where it ends. They sit in the columns of the first and the last step, and are no steps, so nothing picks them. `began` and
 * `ended` are a Job's: it has started its first step, and every step is over.
 */
function withEnds(steps: readonly LaneNodeView[], pairs: readonly (readonly [string, string])[], standing?: { began: boolean; ended: boolean }): { nodes: LaneNodeView[]; moves: (readonly [string, string])[] } {
  const first = steps[0];
  const last = steps[steps.length - 1];
  if (first === undefined || last === undefined) return { nodes: [...steps], moves: [...pairs] };
  const start: LaneNodeView = { id: nodeId("workflow", START_ITEM), label: "Start", col: first.col, role: "step", kind: "start", ...(standing?.began === true ? { passed: true as const } : {}) };
  const end: LaneNodeView = { id: nodeId("workflow", END_ITEM), label: "End", col: last.col, role: "step", kind: "end", ...(standing?.ended === true ? { here: true as const } : {}) };
  return { nodes: [start, ...steps, end], moves: [[start.id, first.id], ...pairs, [last.id, end.id]] };
}

const laneOfLifecycle = (lifecycle: Lifecycle, id: string, title: string, note: string | undefined, nodes: readonly LaneNodeView[]): LaneView => ({
  id,
  kind: "lifecycle",
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

/** The steps of a workflow, placed in columns where the Workspace says, and in order where it does not. At a level below the default each takes what that level changes of it. */
export function stepsOf(workspace: Workspace, workflow: string, steps: readonly { id: string; label: string }[], scope: string = DEFAULT): LaneStep[] {
  return steps.map((step, at) => {
    const given = workspace.workflows[workflow]?.steps[step.id];
    let label = step.label;
    let ends = given?.ends;
    let endsOn = given?.endsOn;
    let passedOver = given?.passedOver;
    for (const layer of layersAt(workspace.workflows[workflow], step.id, scope)) {
      if (layer.label !== undefined) label = layer.label;
      if (layer.ends !== undefined) {
        ends = layer.ends;
        endsOn = layer.endsOn;
      }
      if (layer.passedOver !== undefined) passedOver = layer.passedOver;
    }
    return { id: step.id, label, col: given?.col ?? at, ...(endsOn === undefined ? {} : { endsOn }), ...(ends === undefined ? {} : { ends }), ...(passedOver === undefined ? {} : { passedOver }) };
  });
}

/** The rules a workflow draws as they read at a level: its own, and the Workspace's wherever both ends are on its lanes. */
export function rulesFor(workspace: Workspace, workflow: string, steps: readonly { id: string; label: string }[], scope: string = DEFAULT): Rule[] {
  const drawn = new Set<string>([...steps.map((step) => nodeId("workflow", step.id)), ...lifecyclesFor(workspace, workflow, scope).flatMap((lifecycle) => lifecycle.states.map((state) => nodeId(lifecycle.id, state.id)))]);
  return workspace.rules
    .map((rule) => ruleAt(rule, scope))
    .filter((rule) => (rule.workflow === undefined || rule.workflow === workflow) && drawn.has(nodeId(rule.from.lane, rule.from.item)) && drawn.has(nodeId(rule.to.lane, rule.to.item)));
}

/** Whether a tab changes anything in this workflow of what it takes from the level above, or adds to it. The default has no level above. */
export function changedAt(workspace: Workspace, workflow: string, steps: readonly { id: string; label: string }[], scope: string): boolean {
  if (scope === DEFAULT) return false;
  return (
    lifecyclesFor(workspace, workflow).some((lifecycle) => lifecycle.states.some((state) => stateDiffers(state).includes(scope))) ||
    rulesFor(workspace, workflow, steps).some((rule) => ruleDiffers(rule).includes(scope)) ||
    steps.some((step) => stepDiffers(workspace.workflows[workflow], step.id).includes(scope)) ||
    (workspace.workflows[workflow]?.added ?? []).some((one) => one.scope === scope)
  );
}

/**
 * The editor's lanes: the workflow on top, then one lane for each lifecycle it uses, and each rule it holds as an edge. **The lanes show the Workspace**: where a repository
 * differs, the node or the rule says so and names it. `proposed` are the rules a change waiting on Approve would add, drawn as a diff.
 */
export function editorLanes(input: {
  workspace: Workspace;
  workflow: string;
  steps: readonly { id: string; label: string }[];
  proposed?: readonly Rule[];
  /** `default`, `workspace`, or the id of one repository: the lanes as the workflow runs there, with that level's names, words and autonomy, and the marks for what differs from it down. */
  scope?: string;
}): { lanes: LaneView[]; rules: RuleView[] } {
  const { workspace, workflow } = input;
  const scope = input.scope ?? DEFAULT;
  const named = (ids: readonly string[]): string[] => ids.map((id) => (id === "workspace" ? workspace.name : (workspace.repositories.find((one) => one.id === id)?.name ?? id)));
  /** The levels a mark names: on the default's tab and the Workspace's every one under it that differs, and on a repository's own tab only that one. */
  const marked = (ids: readonly string[]): string[] => named(overviews(scope) ? ids : ids.filter((id) => id === scope));
  /** A rule is drawn where it applies: everywhere on the default's tab and the Workspace's, and on a repository's tab where it is the Workspace's or that repository's own. */
  const appliesHere = (rule: Rule): boolean => overviews(scope) || rule.scope === DEFAULT || rule.scope === "workspace" || rule.scope === scope;
  const steps = stepsOf(workspace, workflow, input.steps, scope);
  const lifecycles = lifecyclesAt(workspace, workflow, scope);
  const placeOf = workspace.workflows[workflow];
  const drawnSteps = withEnds(
    steps.map((step): LaneNodeView => {
      const sub = endsOnSaid(step, lifecycles);
      const differs = marked(stepDiffers(placeOf, step.id));
      return { id: nodeId("workflow", step.id), label: step.label, ...(sub === undefined ? {} : { sub }), col: step.col, role: "step", kind: kindOfStep(step), ...(differs.length === 0 ? {} : { differs }) };
    }),
    steps.slice(1).map((step, at) => [nodeId("workflow", steps[at]!.id), nodeId("workflow", step.id)] as const),
  );
  const flow: LaneView = { id: "workflow", kind: "workflow", title: workflow, source: "Workflow", nodes: drawnSteps.nodes, moves: drawnSteps.moves };
  const lanes = lifecycles.map((lifecycle) =>
    laneOfLifecycle(
      lifecycle,
      lifecycle.id,
      lifecycle.label,
      undefined,
      lifecycle.states.map((state) => {
        const differs = marked(stateDiffers(state));
        return { id: nodeId(lifecycle.id, state.id), label: state.label, col: state.col, role: "state" as const, kind: kindOfState(lifecycle), ...(state.end === true ? { end: true as const } : {}), ...(differs.length === 0 ? {} : { differs }) };
      }),
    ),
  );
  const drawn = (rule: Rule, proposed: boolean): RuleView => {
    const differs = marked(ruleDiffers(rule));
    return {
      id: rule.id,
      rule: rule.id,
      from: nodeId(rule.from.lane, rule.from.item),
      to: nodeId(rule.to.lane, rule.to.item),
      label: rule.label,
      autonomy: rule.autonomy,
      ...(differs.length === 0 ? {} : { differs }),
      ...(proposed ? { proposed: true as const } : {}),
    };
  };
  return {
    lanes: [flow, ...lanes],
    rules: [...rulesFor(workspace, workflow, input.steps, scope).filter(appliesHere).map((rule) => drawn(rule, false)), ...(input.proposed ?? []).map((rule) => ruleAt(rule, scope)).filter(appliesHere).map((rule) => drawn(rule, true))],
  };
}

// ---- A Job ---------------------------------------------------------------------------------------------------------------

/** Where a Job stands in its workflow, as the story reads it. `working` is the agent being on that step now: only then is the step lit. */
export type JobStanding = { workflow: string; at: number; skipped?: readonly number[]; working?: boolean };

/** What a Job's Lanes view draws: where the workflow stands (`at` is -1 before a step starts, the step count once all ended) and what the Job holds. */
export type SessionLanes = JobStanding & { job: JobLanes };

const instancesOf = (job: JobLanes, lane: string): readonly Instance[] => job.instances.filter((one) => one.lifecycle === lane);

/**
 * A Job's lanes: its workflow on top with the step it is at (lit while the agent works on it), then a lane for each instance it holds, the state it
 * stands in marked as where it is, and the ones it went through marked. A rule that fired is an edge with its outcome and when; a rule still waiting is a dashed edge naming what it
 * waits for. A rule turned off is not drawn.
 */
export function jobLanes(job: JobLanes, standing: JobStanding): { lanes: LaneView[]; rules: RuleView[] } {
  const { copy } = job;
  const label = (lane: string, item: string): string => copy.lifecycles.find((one) => one.id === lane)?.states.find((one) => one.id === item)?.label ?? item;
  const drawnSteps = withEnds(
    copy.steps.map((step, at): LaneNodeView => {
      const sub = endsOnSaid(step, copy.lifecycles);
      // What the Job's step waits on is the step it is at.
      const waiting = at === standing.at ? job.waiting : undefined;
      const changed = job.changes.find((one) => one.lane === "workflow" && one.item === step.id)?.text;
      return {
        id: nodeId("workflow", step.id),
        label: step.label,
        ...(sub === undefined ? {} : { sub }),
        col: step.col,
        role: "step",
        kind: kindOfStep(step),
        ...(at === standing.at ? (standing.working === true ? { lit: true as const } : { here: true as const }) : {}),
        ...(at < standing.at ? { passed: true as const } : {}),
        ...(waiting === undefined ? {} : { waiting }),
        ...(standing.skipped?.includes(at) === true ? { skipped: true as const } : {}),
        ...(changed === undefined ? {} : { changed }),
      };
    }),
    copy.steps.slice(1).map((step, at) => [nodeId("workflow", copy.steps[at]!.id), nodeId("workflow", step.id)] as const),
    { began: standing.at >= 0, ended: standing.at >= copy.steps.length },
  );
  const flow: LaneView = { id: "workflow", kind: "workflow", title: standing.workflow, source: "Workflow", nodes: drawnSteps.nodes, moves: drawnSteps.moves };
  const lanes: LaneView[] = [
    // A Job with no workflow has no top lane.
    ...(copy.steps.length === 0 ? [] : [flow]),
    ...job.instances.flatMap((instance): LaneView[] => {
      const lifecycle = copy.lifecycles.find((one) => one.id === instance.lifecycle);
      if (lifecycle === undefined) return [];
      const lane = laneOfLifecycle(
        lifecycle,
        instance.id,
        instance.label,
        instance.title,
        lifecycle.states.map((state) => {
          const says = instance.says?.[state.id];
          const changed = job.changes.find((one) => one.lane === instance.id && one.item === state.id)?.text;
          return {
            id: nodeId(instance.id, state.id),
            label: state.label,
            col: state.col,
            role: "state" as const,
            kind: kindOfState(lifecycle),
            ...(state.id === instance.state ? { here: true as const } : {}),
            ...(state.id !== instance.state && instance.visited.includes(state.id) ? { passed: true as const } : {}),
            ...(state.end === true ? { end: true as const } : {}),
            ...(state.id === instance.state && instance.assignee !== undefined ? { who: instance.assignee } : {}),
            ...(says === undefined ? {} : { facts: [says] }),
            ...(changed === undefined ? {} : { changed }),
          };
        }),
      );
      return [lane];
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
        const waits = source === undefined ? `${name} ${rule.at === "ends" ? "ends" : "starts"}` : `${source.label}: ${name}`;
        rules.push({
          id: `${rule.id}:${(source ?? target)?.id ?? "workflow"}${source !== undefined && target !== undefined ? `>${target.id}` : ""}`,
          rule: rule.id,
          from: nodeId(source?.id ?? "workflow", rule.from.item),
          to: nodeId(target?.id ?? "workflow", rule.to.item),
          label: rule.label,
          meta: fired === undefined ? waits : fired.at,
          autonomy: rule.autonomy,
          status,
          ...(fired?.did === undefined ? {} : { did: fired.did }),
          ...(fired?.needs === undefined ? {} : { needs: fired.needs }),
          ...(first ? {} : { quiet: true as const }),
        });
        first = false;
      }
    }
  }
  return { lanes, rules };
}

/** What is still in flight on a Job: each instance not in an end state, as `Deploy: rolling out`. */
export function inFlight(job: JobLanes): { id: string; label: string; state: string; lifecycle: Instance["lifecycle"] }[] {
  return job.instances.flatMap((instance) => {
    const state = job.copy.lifecycles.find((one) => one.id === instance.lifecycle)?.states.find((one) => one.id === instance.state);
    if (state === undefined || state.end === true) return [];
    return [{ id: instance.id, label: instance.label, state: state.label, lifecycle: instance.lifecycle }];
  });
}
