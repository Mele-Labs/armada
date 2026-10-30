// What a step waits on and what a rule fires on, as the two dropdowns that set them list it: a thing, then its state or its moment. **A mock's options**: the things Fleet watches come
// from the Workspace (`observes`), and every lifecycle it does not already list is a thing too. Pure, so the editor and a story draw the same lists.

import { nodeId } from "./model";
import type { LaneStep, Observed, Rule, Waits, Workspace } from "./model";

/** A thing a step can wait on: the agent, the pull request, the Jobs this Job starts, a lifecycle such as Ticket or Deploy. */
export type WaitSubject = {
  id: string;
  label: string;
  states: readonly { id: string; label: string; /** The words Fleet reads the step's end as, where it reads it. */ says?: string; /** The lifecycle state this is, so the step waits on it. */ endsOn?: { lane: string; state: string } }[];
};

/** Everything a step can wait on or be passed over for: what Fleet watches, then each lifecycle that is not one of those already. */
export function subjectsOf(workspace: Workspace): WaitSubject[] {
  const observed: readonly Observed[] = workspace.observes ?? [];
  const listed = new Set(observed.flatMap((one) => (one.lifecycle === undefined ? [] : [one.lifecycle])));
  return [
    ...observed.map((one) => ({ id: one.id, label: one.label, states: one.states })),
    ...workspace.lifecycles
      .filter((one) => !listed.has(one.id))
      .map((one) => ({ id: one.id, label: one.label, states: one.states.map((state) => ({ id: state.id, label: state.label, endsOn: { lane: one.id, state: state.id } })) })),
  ];
}

/**
 * The pair a step shows: the one the person chose, else the one its words say (`ends` as Fleet read them), else the lifecycle state it waits on. Absent when none of them is a choice
 * in the lists, so the dropdowns open on nothing rather than on a guess.
 */
export function waitsOf(step: LaneStep, which: "ends" | "passedOver", words: string | undefined, subjects: readonly WaitSubject[]): Waits | undefined {
  const chosen = step[which];
  if (chosen !== undefined && subjects.some((one) => one.id === chosen.on && one.states.some((state) => state.id === chosen.state))) return chosen;
  const said = words?.trim().toLowerCase();
  if (said !== undefined && said !== "") {
    for (const one of subjects) {
      const state = one.states.find((each) => each.says?.toLowerCase() === said);
      if (state !== undefined) return { on: one.id, state: state.id };
    }
  }
  if (which === "ends" && step.endsOn !== undefined) {
    const { lane, state } = step.endsOn;
    const found = subjects.find((one) => one.id === lane) ?? subjects.find((one) => one.states.some((each) => each.endsOn?.lane === lane && each.endsOn.state === state));
    const at = found?.states.find((each) => each.id === state || (each.endsOn?.lane === lane && each.endsOn.state === state));
    if (found !== undefined && at !== undefined) return { on: found.id, state: at.id };
  }
  return undefined;
}

/** The lifecycle state a pair makes the step wait on, where it is one. */
export const endsOnOf = (waits: Waits, subjects: readonly WaitSubject[]): { lane: string; state: string } | undefined => subjects.find((one) => one.id === waits.on)?.states.find((state) => state.id === waits.state)?.endsOn;

// ---- When a rule fires ---------------------------------------------------------------------------------------------------------

/** What a rule can fire on, in two lists: the workflow's steps, and the states of each lifecycle it draws. Each value is a node's id. */
export type FireGroup = { label: string; items: readonly { value: string; label: string }[] };

export function fireGroupsOf(steps: readonly { id: string; label: string }[], lifecycles: readonly { id: string; label: string; states: readonly { id: string; label: string }[] }[]): FireGroup[] {
  return [
    { label: "Steps", items: steps.map((step) => ({ value: nodeId("workflow", step.id), label: step.label })) },
    ...lifecycles.map((lifecycle) => ({ label: lifecycle.label, items: lifecycle.states.map((state) => ({ value: nodeId(lifecycle.id, state.id), label: `${lifecycle.label}: ${state.label}` })) })),
  ];
}

/** The moments a rule can fire at on a step and on a state. */
export const MOMENTS = {
  step: [
    { value: "starts", label: "starts" },
    { value: "ends", label: "ends" },
  ],
  state: [
    { value: "enters", label: "is entered" },
    { value: "leaves", label: "is left" },
  ],
} as const;

/** The moment a rule fires at, as it stands: the one it names, or the first its node has. */
export function momentOf(rule: Pick<Rule, "from" | "at">): NonNullable<Rule["at"]> {
  const own = rule.from.lane === "workflow" ? MOMENTS.step : MOMENTS.state;
  return own.some((one) => one.value === rule.at) ? rule.at! : own[0].value;
}
