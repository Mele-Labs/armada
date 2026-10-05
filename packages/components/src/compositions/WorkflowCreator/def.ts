// What the Workflow creator draws, and the refusals it can give before Fleet
// does. Nothing here reads a file or asks Fleet: `json.ts` turns a definition's
// text into this model and back, and the surface supplies the text. The shapes
// follow `crates/core-model/domain/workflowdef-fields.toml`, and the three gate
// settings follow `docs/concepts/workflow.md`, *A step's gate is three
// settings*. **Fleet's loader is the authority on every other refusal**; a rule
// is written here only where it is the loader's own, so a save is never refused
// here for something Fleet would take.

/** Where a definition comes from, least specific first. The most specific wins by id. */
export type Source = "carried" | "kit" | "repository";

/** Where a saved definition is written: `kit`, or a Manifest by its id. */
export type Scope = string;

/** A Manifest a definition may be written to: the id Fleet names it by, and the repository's name a person reads. */
export type ManifestOption = { id: string; name: string };

export const KIT: Scope = "kit";

export const SOURCE_RANK: Record<Source, number> = { carried: 0, kit: 1, repository: 2 };

export const SOURCE_DIR: Record<Source, string> = {
  carried: "compiled in",
  kit: "~/.armada/workflows",
  repository: ".armada/workflows",
};

export const EVIDENCE = ["diff", "failing_test", "facts_note", "test_suite_run", "bundle", "document", "plan", "review"] as const;

/** The mechanical checks a step may name, and `none` for one that names none. */
export const CHECKS = [
  "none",
  "diff_nonempty",
  "artifact_exists",
  "manifest_check",
  "every_manifest_check",
  "plan_recorded",
  "test_run",
  "pr_merged",
] as const;

/** Three independent settings and a fourth state that stands in for all three. */
export type Gate = { checks: boolean; judge: boolean; you: boolean; repository: boolean };

export type Step = {
  id: string;
  /** What the step hands in. Empty is a step that hands in nothing. */
  evidence: string;
  /** The first mechanical check the step names, or `none`. */
  check: string;
  /** The Judge's yes/no question. Empty is no Judge check. */
  judge: string;
  gate: Gate;
  retryLimit: number;
  /** `verdict_routing`: the earlier step this one sends the work back to. Empty is none. */
  returnsTo: string;
  /** How many passes the back edge may make. Read only where `returnsTo` is set. */
  iterationCap: number;
  /** The step as its file wrote it. A key the editor does not own goes back out unchanged. */
  carried?: Record<string, unknown>;
};

/** A workflow is its steps. Any step may send the work back to an earlier one. */
export type Definition = {
  id: string;
  scope: Scope;
  steps: Step[];
  /** The file's other top-level keys, which go back out unchanged. */
  carried?: Record<string, unknown>;
};

/** What opening a file answers: its definition, or the reason it cannot be drawn. */
export type Read = { ok: true; def: Definition } | { ok: false; said: string };

/** What a save answers. `exists` is Fleet's refusal that a definition is already there, which a second press may replace. */
export type Saved = { ok: true } | { ok: false; said: string; exists?: true };

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
  /** The field that fixes it, as the panel labels it. */
  field: string;
};

/**
 * The refusals Fleet's loader gives that need no Fleet to know. `taken` is gone
 * and so is the lowercase rule on ids: whether an id is already a file in the
 * place it is written, and what a file may be called, are Fleet's to say.
 */
export function refusalsOf(def: Definition): Refusal[] {
  const out: Refusal[] = [];
  if (def.id === "") out.push({ where: "id", why: "Empty", field: "Workflow id" });
  if (def.steps.length === 0) out.push({ where: "steps", why: "At least one step", field: "Add step" });

  const seen = new Set<string>();
  def.steps.forEach((step, at) => {
    const n = at + 1;
    if (step.id === "") out.push({ where: `steps[${n}].id`, why: "Empty", step: at, field: "Step id" });
    else if (seen.has(step.id)) out.push({ where: `steps[${n}].id`, why: `Duplicate step name ${step.id}`, step: at, field: "Step id" });
    seen.add(step.id);

    // A Judge gate with no question reads as no Judge at all, and the loader refuses a gate that names one without the other.
    if (step.gate.judge && !step.gate.you && !step.gate.repository && step.judge.trim() === "") {
      out.push({ where: `steps[${n}].judge_checks`, why: "Judge ticked and the step names no question", step: at, field: "Judge question" });
    }
    if (!Number.isInteger(step.retryLimit) || step.retryLimit < 0) {
      out.push({ where: `steps[${n}].retry_limit`, why: "A whole number, 0 or more", step: at, field: "Retries" });
    }
    if (step.returnsTo !== "") {
      const to = def.steps.findIndex((one) => one.id === step.returnsTo);
      if (to === -1) {
        out.push({ where: `steps[${n}].verdict_routing`, why: `${step.returnsTo} is not a step`, step: at, field: "Sends work back to" });
      } else if (to >= at) {
        out.push({ where: `steps[${n}].verdict_routing`, why: `${step.returnsTo} is not before ${displayId(step, n)}`, step: at, field: "Sends work back to" });
      }
      if (!Number.isInteger(step.iterationCap) || step.iterationCap < 0) {
        out.push({ where: `steps[${n}].iteration_cap`, why: "A whole number, 0 or more", step: at, field: "Passes" });
      }
    }
  });
  return out;
}

function displayId(step: Step | { id: string }, n: number): string {
  return step.id === "" ? `step ${n}` : step.id;
}

/** The file a definition is written to, from the Manifest the window is in. `scopeName` is the repository's name for a Manifest. */
export function fileOf(def: Pick<Definition, "id" | "scope">, current: string, scopeName: (scope: Scope) => string): string {
  const name = def.id === "" ? "…" : def.id;
  if (def.scope === KIT) return `${SOURCE_DIR.kit}/${name}.json`;
  return `${def.scope === current ? "" : `${scopeName(def.scope)}/`}${SOURCE_DIR.repository}/${name}.json`;
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
