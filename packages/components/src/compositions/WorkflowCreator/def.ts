// What the Workflow creator draws, and the rules it refuses a definition on.
// **A mock.** Nothing here reads a file or asks Fleet: the list, the
// definitions and the refusals are written down so the screen can be walked
// before anything real is built. The shapes follow
// `crates/core-model/domain/workflowdef-fields.toml`, the refusals follow the
// load-time rules that file names, and the three gate settings follow
// `docs/concepts/workflow.md`, *A step's gate is three settings*.

/** Where a definition comes from, least specific first. The most specific wins by id. */
export type Source = "carried" | "kit" | "repository";

/** Where a saved definition is written: Kit, or this repository's `.armada/workflows/`. */
export type Scope = "kit" | "repository";

export const SOURCE_RANK: Record<Source, number> = { carried: 0, kit: 1, repository: 2 };

export const SOURCE_WORD: Record<Source, string> = {
  carried: "Carried",
  kit: "Kit",
  repository: "Repository",
};

export const SOURCE_DIR: Record<Source, string> = {
  carried: "compiled in",
  kit: "~/.armada/workflows",
  repository: ".armada/workflows",
};

export type Structure = "linear" | "loop";

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
  /** `verdict_routing`: the step a loop returns to. Empty is none. */
  returnsTo: string;
};

export type Definition = {
  id: string;
  structure: Structure;
  scope: Scope;
  iterationCap: number;
  steps: Step[];
};

/** One row of the list: a file Fleet found, and what became of it. */
export type Entry = {
  key: string;
  id: string;
  source: Source;
  file: string;
  structure: Structure;
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
    out.push({ where: "id", why: def.scope === "kit" ? "Another Kit file has this id" : "Another file in .armada/workflows has this id" });
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
      if (def.structure === "linear") {
        out.push({ where: `steps[${n}].verdict_routing`, why: `linear carries verdict_routing on ${displayId(step, n)}`, step: at });
      } else if (to === -1) {
        out.push({ where: `steps[${n}].verdict_routing`, why: `${step.returnsTo} is not a step`, step: at });
      } else if (to > at) {
        out.push({ where: `steps[${n}].verdict_routing`, why: `${step.returnsTo} comes after ${displayId(step, n)}`, step: at });
      }
    }
  });
  if (def.structure === "loop" && def.steps.every((one) => one.returnsTo === "")) {
    out.push({ where: "structure", why: "loop carries no verdict_routing" });
  }
  if (def.structure === "loop" && (!Number.isInteger(def.iterationCap) || def.iterationCap < 1)) {
    out.push({ where: "iteration_cap", why: "At least 1" });
  }
  return out;
}

function displayId(step: Step | { id: string }, n: number): string {
  return step.id === "" ? `step ${n}` : step.id;
}

/** The file a definition is written to. */
export function fileOf(def: Pick<Definition, "id" | "scope">): string {
  const dir = def.scope === "kit" ? SOURCE_DIR.kit : SOURCE_DIR.repository;
  return `${dir}/${def.id === "" ? "…" : def.id}.json`;
}

/** The draft as plain text, for Helm to read. */
export function textOf(def: Definition): string {
  return JSON.stringify(
    {
      workflow_id: def.id,
      structure: def.structure,
      ...(def.structure === "loop" ? { iteration_cap: def.iterationCap } : {}),
      steps: def.steps.map((step, at) => ({
        id: step.id,
        order: at + 1,
        evidence: { submitted: { type: step.evidence } },
        mechanical_checks: step.check === "none" ? [] : [{ type: step.check }],
        judge_checks: step.judge === "" ? [] : [{ enabled: true, question: step.judge }],
        gate: step.gate,
        retry_limit: step.retryLimit,
        ...(step.returnsTo === "" ? {} : { verdict_routing: step.returnsTo }),
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
  };
}

export function blankDefinition(): Definition {
  return { id: "", structure: "linear", scope: "kit", iterationCap: 5, steps: [blankStep()] };
}
