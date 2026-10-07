// What a trigger is, as the mock draws it: a script or a skill that fires at a
// moment in a Job. Nothing here reaches Fleet — the triggers are held by the
// Workflow creator and Job detail share, seeded from `MOCK_TRIGGERS`. Delete with
// the mock.

import { useSyncExternalStore } from "react";

import type { Source } from "../WorkflowCreator/def";

type Listener = () => void;

/** A mock's shared state: read by the creator and by Job detail, so a trigger kept in one is there in the other. */
function store<T>(initial: T) {
  let value = initial;
  const listeners = new Set<Listener>();
  return {
    get: () => value,
    set: (next: T | ((was: T) => T)) => {
      value = typeof next === "function" ? (next as (was: T) => T)(value) : next;
      listeners.forEach((one) => one());
    },
    subscribe: (one: Listener) => {
      listeners.add(one);
      return () => void listeners.delete(one);
    },
  };
}

/** `pr_opened` is the delivering step starting. */
export type TriggerWhen = "starts" | "passes" | "pr_opened";

export type TriggerKind = "command" | "skill";

/** `every` is every workflow; any other value is one workflow's id. */
export const EVERY = "every";

export type Trigger = {
  id: string;
  /** Where it is set. A trigger set in a more specific place replaces the same trigger set in a less specific one. */
  place: Source;
  applies: string;
  when: TriggerWhen;
  /** The step a `starts` or `passes` trigger names. Empty for `pr_opened`. */
  step: string;
  kind: TriggerKind;
  name: string;
  /** If it fails: the Job waits. */
  block: boolean;
  /** If it fails: a repair Drone is dispatched, then the trigger goes again. */
  repair: boolean;
};

export type ResolvedTrigger = Trigger & { overriddenBy?: Source };

/** The Commands the repository's `armada.yml` names, and the Skills a Drone runs. */
export const MOCK_COMMANDS = ["deploy_qa", "lint_docs", "smoke"] as const;
export const MOCK_SKILLS = ["qa-notes", "release-checklist"] as const;

/** The step that delivers, in the carried Feature workflow. */
export const DELIVERING = "handoff";

export const PLACE_DIR: Record<Source, string> = {
  carried: "compiled in",
  kit: "~/.armada",
  repository: ".armada",
};

export const MOCK_TRIGGERS: readonly Trigger[] = [
  { id: "r1", place: "kit", applies: EVERY, when: "pr_opened", step: "", kind: "command", name: "deploy_qa", block: false, repair: true },
  { id: "r2", place: "repository", applies: EVERY, when: "pr_opened", step: "", kind: "command", name: "deploy_qa", block: true, repair: false },
  { id: "r3", place: "repository", applies: EVERY, when: "passes", step: "implement", kind: "command", name: "lint_docs", block: false, repair: false },
  { id: "r4", place: "repository", applies: "feature", when: "starts", step: "tests", kind: "skill", name: "qa-notes", block: false, repair: false },
];

/**
 * Most specific place wins. The repository is shared and the machine is only
 * the owner's, so the machine is the more specific of the two.
 */
const RANK: Record<Source, number> = { carried: 0, repository: 1, kit: 2 };

const sameTrigger = (one: Trigger) => `${one.when}|${one.step}|${one.name}`;

/** Each trigger, and the place that answers for it instead where a more specific one does. */
export function resolveTriggers(triggers: readonly Trigger[]): ResolvedTrigger[] {
  return triggers.map((one) => {
    const winner = triggers
      .filter((other) => sameTrigger(other) === sameTrigger(one) && RANK[other.place] > RANK[one.place])
      .sort((a, b) => RANK[b.place] - RANK[a.place])[0];
    return winner === undefined ? one : { ...one, overriddenBy: winner.place };
  });
}

/** The moment, as a short phrase. */
export function whenSaid(trigger: Pick<Trigger, "when" | "step">): string {
  if (trigger.when === "pr_opened") return "PR opened";
  return `${trigger.step === "" ? "a step" : trigger.step} ${trigger.when}`;
}

/** A trigger fired in a Job, as Job detail draws it. */
export type FiredTrigger = {
  name: string;
  when: string;
  place: Source;
  state: "passed" | "repairing";
};

/** The Jobs the mock fired triggers on, by Job id. */
const MOCK_FIRED: Readonly<Record<string, readonly FiredTrigger[]>> = {
  "01M3WJ4CVF0021ZQB9G8PQMAHM": [
    { name: "lint_docs", when: "implement passes", place: "repository", state: "passed" },
    { name: "deploy_qa", when: "PR opened", place: "kit", state: "repairing" },
  ],
};

export function firedTriggersOf(jobId: string): readonly FiredTrigger[] | undefined {
  return MOCK_FIRED[jobId];
}

const triggerStore = store<readonly Trigger[]>(MOCK_TRIGGERS);

/** The saved triggers, and the way to change them. */
export function useTriggers() {
  return [useSyncExternalStore(triggerStore.subscribe, triggerStore.get), triggerStore.set] as const;
}

/** A step added to one Job only: a script Fleet runs, a skill a Drone runs, or a Drone with a short brief. */
export type InsertedKind = "script" | "skill" | "drone";

export type Inserted = {
  id: string;
  /** The step, or the inserted step, it follows. */
  after: string;
  kind: InsertedKind;
  /** The command or the skill. Empty for a Drone step. */
  name: string;
  /** A Drone step's brief. */
  brief: string;
  block: boolean;
  repair: boolean;
  state: "planned" | "firing" | "passed";
  /** Where it was kept for every Job, once it was. */
  kept?: Source;
};

const NONE: readonly Inserted[] = [];
const insertedStore = store<Readonly<Record<string, readonly Inserted[]>>>({});

/** The steps added to one Job, in the order they were added. */
export function useInserted(jobId: string): readonly Inserted[] {
  const all = useSyncExternalStore(insertedStore.subscribe, insertedStore.get);
  return all[jobId] ?? NONE;
}

let nextInserted = 0;

/** Add a step to a Job. In the mock it fires a moment after, and passes a moment after that. */
export function insertStep(jobId: string, after: string, kind: InsertedKind): string {
  const id = `added-${++nextInserted}`;
  const one: Inserted = {
    id,
    after,
    kind,
    name: kind === "script" ? "deploy_qa" : kind === "skill" ? MOCK_SKILLS[0] : "",
    brief: "",
    block: false,
    repair: false,
    state: "planned",
  };
  // A step added where one already is goes in front of it, so each step has at most one that follows.
  insertedStore.set((was) => ({
    ...was,
    [jobId]: [...(was[jobId] ?? []).map((other) => (other.after === after ? { ...other, after: id } : other)), one],
  }));
  window.setTimeout(() => changeInserted(jobId, id, { state: "firing" }), 2000);
  window.setTimeout(() => changeInserted(jobId, id, { state: "passed" }), 6000);
  return id;
}

export function changeInserted(jobId: string, id: string, next: Partial<Inserted>) {
  insertedStore.set((was) => ({ ...was, [jobId]: (was[jobId] ?? []).map((one) => (one.id === id ? { ...one, ...next } : one)) }));
}

export function removeInserted(jobId: string, id: string) {
  insertedStore.set((was) => ({
    ...was,
    [jobId]: (was[jobId] ?? []).filter((one) => one.id !== id && one.after !== id),
  }));
}

/** What keeping an added step for every Job starts from: a trigger that fires when the step it follows passes. */
export function triggerFromInserted(one: Inserted, workflow: string, after: string): Trigger {
  return {
    id: `h-${one.id}`,
    place: "kit",
    applies: workflow,
    when: "passes",
    step: after,
    kind: one.kind === "skill" ? "skill" : "command",
    name: one.name,
    block: one.block,
    repair: one.repair,
  };
}

export function keepTrigger(trigger: Trigger) {
  triggerStore.set((was) => [...was.filter((one) => one.id !== trigger.id), trigger]);
}
