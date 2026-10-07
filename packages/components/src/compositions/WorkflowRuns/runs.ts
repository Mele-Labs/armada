// What a run is, as the mock draws it: a script or a skill that fires at a
// moment in a Job. Nothing here reaches Fleet — the runs are held by the
// Workflow creator while it is open and seeded from `MOCK_RUNS`. Delete with
// the mock.

import type { Source } from "../WorkflowCreator/def";

/** `pr_opened` is the delivering step starting. */
export type RunWhen = "starts" | "passes" | "pr_opened";

export type RunKind = "command" | "skill";

/** `every` is every workflow; any other value is one workflow's id. */
export const EVERY = "every";

export type Run = {
  id: string;
  /** Where it is set. A run set in a more specific place replaces the same run set in a less specific one. */
  place: Source;
  applies: string;
  when: RunWhen;
  /** The step a `starts` or `passes` run names. Empty for `pr_opened`. */
  step: string;
  kind: RunKind;
  name: string;
  /** If it fails: the Job waits. */
  block: boolean;
  /** If it fails: a repair Drone is dispatched, then the run goes again. */
  repair: boolean;
};

export type ResolvedRun = Run & { overriddenBy?: Source };

/** The Commands the repository's `armada.yml` names, and the Skills a Drone may run. */
export const MOCK_COMMANDS = ["deploy_qa", "lint_docs", "smoke"] as const;
export const MOCK_SKILLS = ["qa-notes", "release-checklist"] as const;

/** The step that delivers, in the carried Feature workflow. */
export const DELIVERING = "deliver";

export const PLACE_DIR: Record<Source, string> = {
  carried: "compiled in",
  kit: "~/.armada",
  repository: ".armada",
};

export const MOCK_RUNS: readonly Run[] = [
  { id: "r1", place: "kit", applies: EVERY, when: "pr_opened", step: "", kind: "command", name: "deploy_qa", block: false, repair: true },
  { id: "r2", place: "repository", applies: EVERY, when: "pr_opened", step: "", kind: "command", name: "deploy_qa", block: true, repair: false },
  { id: "r3", place: "repository", applies: EVERY, when: "passes", step: "implement", kind: "command", name: "lint_docs", block: false, repair: false },
  { id: "r4", place: "repository", applies: "feature", when: "starts", step: "review", kind: "skill", name: "qa-notes", block: false, repair: false },
];

/**
 * Most specific place wins. The repository is shared and the machine is only
 * the owner's, so the machine is the more specific of the two.
 */
const RANK: Record<Source, number> = { carried: 0, repository: 1, kit: 2 };

const sameRun = (one: Run) => `${one.when}|${one.step}|${one.name}`;

/** Each run, and the place that answers for it instead where a more specific one does. */
export function resolveRuns(runs: readonly Run[]): ResolvedRun[] {
  return runs.map((one) => {
    const winner = runs
      .filter((other) => sameRun(other) === sameRun(one) && RANK[other.place] > RANK[one.place])
      .sort((a, b) => RANK[b.place] - RANK[a.place])[0];
    return winner === undefined ? one : { ...one, overriddenBy: winner.place };
  });
}

/** The moment, as a short phrase. */
export function whenSaid(run: Pick<Run, "when" | "step">): string {
  if (run.when === "pr_opened") return "PR opened";
  return `${run.step === "" ? "a step" : run.step} ${run.when}`;
}

/** A run fired in a Job, as Job detail draws it. */
export type FiredRun = {
  name: string;
  when: string;
  place: Source;
  state: "passed" | "repairing";
};

/** The Jobs the mock fired runs on, by Job id. */
const MOCK_FIRED: Readonly<Record<string, readonly FiredRun[]>> = {
  "01M3WJ4CVF0021ZQB9G8PQMAHM": [
    { name: "lint_docs", when: "implement passes", place: "repository", state: "passed" },
    { name: "deploy_qa", when: "PR opened", place: "kit", state: "repairing" },
  ],
};

export function firedRunsOf(jobId: string): readonly FiredRun[] | undefined {
  return MOCK_FIRED[jobId];
}
