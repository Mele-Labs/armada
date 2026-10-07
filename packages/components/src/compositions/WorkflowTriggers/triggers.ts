// What a trigger is, as the mock draws it: a script or a skill that fires at a
// moment in a Job. Nothing here reaches Fleet — the triggers are held by the
// Workflow creator and Job detail share, seeded from `MOCK_TRIGGERS`. Delete with
// the mock.

import { useEffect, useSyncExternalStore } from "react";

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
  /** The Job it fired in, so a repair under way can change what the row says. */
  job?: string;
  name: string;
  when: string;
  place: Source;
  state: "passed" | "repairing" | "failed";
};

/** The Jobs the mock fired triggers on, by Job id. */
const FIRED: readonly FiredTrigger[] = [
  { name: "lint_docs", when: "implement passes", place: "repository", state: "passed" },
  { name: "deploy_qa", when: "PR opened", place: "kit", state: "repairing" },
];
const MOCK_FIRED: Readonly<Record<string, readonly FiredTrigger[]>> = {
  "01M3WJ4CVF0021ZQB9G8PQMAHM": FIRED,
  "01M3WJ4CVF0021ZQB9G8PQMAHN": FIRED,
};

export function firedTriggersOf(jobId: string): readonly FiredTrigger[] | undefined {
  return MOCK_FIRED[jobId]?.map((one) => ({ ...one, job: jobId }));
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

/**
 * What keeping an added step for every Job starts from. One added after a step
 * fires when that step passes; one in the delivery lane fires when the PR
 * opens; one ahead of the first step fires when that step starts.
 */
export function triggerFromInserted(one: Inserted, workflow: string, after: string): Trigger {
  const delivery = after.startsWith("at:");
  const ahead = after.startsWith("before:");
  return {
    id: `h-${one.id}`,
    place: "kit",
    applies: workflow,
    when: delivery ? "pr_opened" : ahead ? "starts" : "passes",
    step: delivery ? "" : ahead ? after.slice("before:".length) : after,
    kind: one.kind === "skill" ? "skill" : "command",
    name: one.name,
    block: one.block,
    repair: one.repair,
  };
}

export function keepTrigger(trigger: Trigger) {
  triggerStore.set((was) => [...was.filter((one) => one.id !== trigger.id), trigger]);
}

/**
 * A failed trigger with Self repair on. A repair Drone works on a fix, then the
 * branch holds and asks where the fix goes: on the Job's open PR, or a PR of its
 * own. The trigger then runs again. Two tries, and the second failing ends it.
 */
export type RepairPhase = "working" | "asking" | "rerunning" | "done" | "failed";

export type Repair = {
  trigger: string;
  /** The step the trigger fired at: the one that opens the PR. */
  at: string;
  phase: RepairPhase;
  /** Which of the two tries the Drone is on. */
  tries: number;
  choice?: "branch" | "newpr";
  /** What the fix changes, which is what he is approving. */
  fix: readonly { path: string; change: "added" | "modified" }[];
};

/** The Jobs a repair is mocked on, and whether the Drone finds a fix. */
const REPAIRS: Readonly<Record<string, "works" | "fails">> = {
  "01M3WJ4CVF0021ZQB9G8PQMAHM": "works",
  "01M3WJ4CVF0021ZQB9G8PQMAHN": "fails",
};

const FIX = [
  { path: "deploy/qa.sh", change: "modified" },
  { path: ".armada/qa.env", change: "added" },
] as const;

const repairStore = store<Readonly<Record<string, Repair>>>({});
const begun = new Set<string>();
/** Bumped when a repair is let go, so the timers of the old one do nothing. */
const generation = new Map<string, number>();
const watching = new Map<string, number>();

const generationOf = (jobId: string) => generation.get(jobId) ?? 0;

/** Run `then` later, unless the repair it belongs to has been let go. */
function later(jobId: string, ms: number, then: () => void) {
  const mine = generationOf(jobId);
  window.setTimeout(() => {
    if (mine === generationOf(jobId)) then();
  }, ms);
}

/** Start the mocked repair once, when a Job that has one is first looked at. */
function beginRepair(jobId: string) {
  const plan = REPAIRS[jobId];
  if (plan === undefined || begun.has(jobId)) return;
  begun.add(jobId);
  const set = (next: Partial<Repair>) =>
    repairStore.set((was) => ({ ...was, [jobId]: { ...was[jobId]!, ...next } }));
  repairStore.set((was) => ({
    ...was,
    [jobId]: { trigger: "deploy_qa", at: "handoff", phase: "working", tries: 1, fix: FIX },
  }));
  if (plan === "works") {
    later(jobId, 3500, () => set({ phase: "asking" }));
  } else {
    later(jobId, 2000, () => set({ tries: 2 }));
    later(jobId, 4000, () => set({ phase: "failed" }));
  }
}

/** Let a repair go once nothing is looking at its Job, so the next look starts it again. */
function releaseRepair(jobId: string) {
  window.setTimeout(() => {
    if ((watching.get(jobId) ?? 0) > 0) return;
    generation.set(jobId, generationOf(jobId) + 1);
    begun.delete(jobId);
    repairStore.set((was) => Object.fromEntries(Object.entries(was).filter(([id]) => id !== jobId)));
  }, 100);
}

/** The repair on a Job, if it has one. Looking starts it. */
export function useRepair(jobId: string): Repair | undefined {
  const all = useSyncExternalStore(repairStore.subscribe, repairStore.get);
  useEffect(() => {
    watching.set(jobId, (watching.get(jobId) ?? 0) + 1);
    beginRepair(jobId);
    return () => {
      watching.set(jobId, (watching.get(jobId) ?? 1) - 1);
      releaseRepair(jobId);
    };
  }, [jobId]);
  return all[jobId];
}

/** The repair as it stands, without starting it. */
export function useRepairSnapshot(jobId: string | undefined): Repair | undefined {
  const all = useSyncExternalStore(repairStore.subscribe, repairStore.get);
  return jobId === undefined ? undefined : all[jobId];
}

/** Where the fix goes. The trigger runs again, and passes. */
export function chooseRepair(jobId: string, choice: "branch" | "newpr") {
  repairStore.set((was) => (was[jobId] === undefined ? was : { ...was, [jobId]: { ...was[jobId]!, phase: "rerunning", choice } }));
  later(jobId, 2500, () =>
    repairStore.set((was) => (was[jobId] === undefined ? was : { ...was, [jobId]: { ...was[jobId]!, phase: "done" } })),
  );
}
