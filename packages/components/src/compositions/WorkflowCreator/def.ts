// What the Workflow creator draws, and the rules it refuses a definition on.
// **A mock.** Nothing here reads a file or asks Fleet: the list, the
// definitions and the refusals are written down so the screen can be walked
// before anything real is built. The shapes follow
// `crates/core-model/domain/workflowdef-fields.toml`, the refusals follow the
// load-time rules that file names, and the three gate settings follow
// `docs/concepts/workflow.md`, *A step's gate is three settings*.

/** Where a definition comes from, least specific first. The most specific wins by id. */
export type Source = "carried" | "kit" | "repository";

/** Where a saved definition is written: `kit`, or a Manifest by name. */
export type Scope = string;

export const KIT: Scope = "kit";

export const SOURCE_RANK: Record<Source, number> = { carried: 0, kit: 1, repository: 2 };

export const SOURCE_DIR: Record<Source, string> = {
  carried: "compiled in",
  kit: "~/.armada/workflows",
  repository: ".armada/workflows",
};

export const EVIDENCE = ["diff", "failing_test", "facts_note", "test_suite_run", "bundle", "document"] as const;
export type Evidence = (typeof EVIDENCE)[number];

export const CHECKS = [
  "none",
  "diff_nonempty",
  "artifact_exists",
  "manifest_check",
  "every_manifest_check",
  "test_run",
  "plan_recorded",
] as const;
export type Check = (typeof CHECKS)[number];

/** Three independent settings and a fourth state that stands in for all three. */
export type Gate = { checks: boolean; judge: boolean; you: boolean; repository: boolean };

export type Step = {
  id: string;
  evidence: Evidence;
  /** The one mechanical check this mock draws. */
  check: Check;
  /** The Judge's yes/no question. Empty is no Judge check. */
  judge: string;
  gate: Gate;
  retryLimit: number;
  /** `verdict_routing`: the earlier step this one sends the work back to. Empty is none. */
  returnsTo: string;
  /** How many passes the back edge may make. Read only where `returnsTo` is set. */
  iterationCap: number;
};

/** A workflow is its steps. Any step may send the work back to an earlier one. */
export type Definition = {
  id: string;
  scope: Scope;
  steps: Step[];
};

/** One row of the list: a file Fleet found, and what became of it. */
export type Entry = {
  key: string;
  id: string;
  source: Source;
  file: string;
  /** Set aside at start, and why. Such a file is never edited here. */
  leftOut?: string;
};

export type Resolved = Entry & {
  /** The place that answers for this id instead, where this one is shadowed. */
  overriddenBy?: Source;
};

/** The list as Fleet resolves it: by id, the most specific place wins, and a left-out file does not take part. */
export function resolve(entries: readonly Entry[]): Resolved[] {
  const winner = new Map<string, Source>();
  for (const one of entries) {
    if (one.leftOut !== undefined) continue;
    const held = winner.get(one.id);
    if (held === undefined || SOURCE_RANK[one.source] > SOURCE_RANK[held]) winner.set(one.id, one.source);
  }
  return entries.map((one) => {
    const won = winner.get(one.id);
    return one.leftOut === undefined && won !== undefined && won !== one.source
      ? { ...one, overriddenBy: won }
      : { ...one };
  });
}

export type Refusal = {
  /** The field the rule is about, as the definition spells it. */
  where: string;
  why: string;
  /** The step it is about, to mark in the frame. */
  step?: number;
};

const NAME = /^[a-z][a-z0-9_]*$/;

/**
 * The refusals the loader would give this definition. `taken` is the ids of the
 * other files in the place it would be written to: a repository's own is
 * strict about a shared id, and Kit sets both aside.
 */
export function refusalsOf(def: Definition, taken: readonly string[]): Refusal[] {
  const out: Refusal[] = [];
  if (def.id === "") out.push({ where: "id", why: "Empty" });
  else if (!NAME.test(def.id)) out.push({ where: "id", why: "Lowercase letters, digits and underscores, starting with a letter" });
  else if (taken.includes(def.id)) {
    out.push({ where: "id", why: def.scope === KIT ? "Another Kit file has this id" : "Another file in .armada/workflows has this id" });
  }
  if (def.steps.length === 0) out.push({ where: "steps", why: "At least one step" });

  const seen = new Set<string>();
  def.steps.forEach((step, at) => {
    const n = at + 1;
    if (step.id === "") out.push({ where: `steps[${n}].id`, why: "Empty", step: at });
    else if (!NAME.test(step.id)) out.push({ where: `steps[${n}].id`, why: "Lowercase letters, digits and underscores", step: at });
    else if (seen.has(step.id)) out.push({ where: `steps[${n}].id`, why: `Duplicate step name ${step.id}`, step: at });
    seen.add(step.id);

    if (step.gate.checks && step.check === "none") {
      out.push({ where: `steps[${n}].gate`, why: "Checks ticked and the step names no check", step: at });
    }
    if (step.gate.judge && step.judge.trim() === "") {
      out.push({ where: `steps[${n}].judge_checks`, why: "Judge ticked and the step names no question", step: at });
    }
    if (step.returnsTo !== "") {
      const to = def.steps.findIndex((one) => one.id === step.returnsTo);
      if (to === -1) {
        out.push({ where: `steps[${n}].verdict_routing`, why: `${step.returnsTo} is not a step`, step: at });
      } else if (to >= at) {
        out.push({ where: `steps[${n}].verdict_routing`, why: `${step.returnsTo} is not before ${displayId(step, n)}`, step: at });
      }
      if (!Number.isInteger(step.iterationCap) || step.iterationCap < 1) {
        out.push({ where: `steps[${n}].iteration_cap`, why: "At least 1", step: at });
      }
    }
  });
  return out;
}

function displayId(step: Step | { id: string }, n: number): string {
  return step.id === "" ? `step ${n}` : step.id;
}

/** The file a definition is written to, from the Manifest the window is in. */
export function fileOf(def: Pick<Definition, "id" | "scope">, current: string): string {
  const name = def.id === "" ? "…" : def.id;
  if (def.scope === KIT) return `${SOURCE_DIR.kit}/${name}.json`;
  return `${def.scope === current ? "" : `${def.scope}/`}${SOURCE_DIR.repository}/${name}.json`;
}

/** The draft as plain text, for Helm to read. */
export function textOf(def: Definition): string {
  return JSON.stringify(
    {
      workflow_id: def.id,
      steps: def.steps.map((step, at) => ({
        id: step.id,
        order: at + 1,
        evidence: { submitted: { type: step.evidence } },
        mechanical_checks: step.check === "none" ? [] : [{ type: step.check }],
        judge_checks: step.judge === "" ? [] : [{ enabled: true, question: step.judge }],
        gate: step.gate,
        retry_limit: step.retryLimit,
        ...(step.returnsTo === "" ? {} : { verdict_routing: step.returnsTo, iteration_cap: step.iterationCap }),
      })),
    },
    null,
    2,
  );
}

export function blankStep(): Step {
  return {
    id: "",
    evidence: "diff",
    check: "none",
    judge: "",
    gate: { checks: false, judge: false, you: false, repository: false },
    retryLimit: 3,
    returnsTo: "",
    iterationCap: 5,
  };
}

/** A new definition, written to the Manifest the window is in. */
export function blankDefinition(scope: Scope): Definition {
  return { id: "", scope, steps: [blankStep()] };
}
