/// <reference path="./js-yaml.d.ts" />
// A definition's text, as the editor's model and back.
//
// **The file is Fleet's, and the editor owns a few of its keys.** A step is read
// into `Step`, and the step as written is kept beside it as `carried`. Writing
// puts a key back only where the editor's value differs from what the carried
// step reads as, so a definition that is opened and saved untouched goes out as
// it came in: `on_complete`, `evidence_scope`, `model`, a Judge's other
// criteria, a check's own parameters. **There is no `structure` field and the
// editor never emits one**: a step loops back by naming an earlier step in
// `verdict_routing`, and `iteration_cap` goes out only beside it.
//
// A definition's text may be YAML, since Fleet reads one with a YAML parser and
// the carried set is written that way, so JSON is tried first and YAML second.
// What goes back is JSON, which is the same thing to the loader.

import { load } from "js-yaml";

import { KIT, SOURCE_RANK, type Definition, type Entry, type Gate, type Read, type Source, type Step } from "./def";

type Table = Record<string, unknown>;

const table = (value: unknown): Table =>
  typeof value === "object" && value !== null && !Array.isArray(value) ? (value as Table) : {};
const list = (value: unknown): unknown[] => (Array.isArray(value) ? value : []);
const text = (value: unknown): string | undefined => (typeof value === "string" ? value : undefined);

/** The place Fleet spells `armada` is `carried` here, where the editor says what ships with Armada. */
export const SOURCE_OF_WIRE: Readonly<Record<string, Source>> = { armada: "carried", kit: "kit", repository: "repository" };
export const WIRE_OF_SOURCE: Readonly<Record<Source, string>> = { carried: "armada", kit: "kit", repository: "repository" };

/** A definition file's text as a tree: JSON, or YAML where it is not JSON. */
export function parse(source: string): unknown {
  try {
    return JSON.parse(source);
  } catch {
    return load(source);
  }
}

/** What the editor shows of a step as its file wrote it. */
function stepOf(raw: Table): Step {
  const word = text(raw.advance_gate) ?? "auto";
  const repository = word.startsWith("manifest_rule:");
  const judges = list(raw.judge_checks).map(table);
  const criteria = judges.flatMap((one) => list(one.criteria).map(table));
  const fires = judges.some((one) => one.enabled !== false && list(one.criteria).length > 0);
  const checks = list(raw.mechanical_checks).map(table);
  return {
    id: text(raw.id) ?? "",
    evidence: text(table(table(raw.evidence).submitted).type) ?? "",
    check: text(checks[0]?.type) ?? "none",
    judge: criteria.map((one) => text(one.question)).find((one) => one !== undefined) ?? "",
    gate: {
      checks: !repository && checks.length > 0,
      judge: !repository && fires,
      you: word === "human_always",
      repository,
    },
    retryLimit: typeof raw.retry_limit === "number" ? raw.retry_limit : 0,
    returnsTo: text(table(raw.verdict_routing).request_changes) ?? "",
    iterationCap: typeof raw.iteration_cap === "number" ? raw.iteration_cap : 5,
    carried: raw,
  };
}

/**
 * A file's text as a definition, drawn for the Manifest the window is in. A
 * Kit file is written back to Kit, and any other to that Manifest. Anything that
 * is not a workflow — no `steps` list — is an `ok: false` with the reason.
 */
export function readDefinition(source: string, from: Source, current: string): Read {
  let value: unknown;
  try {
    value = parse(source);
  } catch (cause) {
    return { ok: false, said: cause instanceof Error ? cause.message : "Not readable" };
  }
  const top = table(value);
  if (!Array.isArray(top.steps)) return { ok: false, said: "No steps" };
  const { workflow_id, steps, ...rest } = top;
  return {
    ok: true,
    def: {
      id: text(workflow_id) ?? "",
      scope: from === "kit" ? KIT : current,
      steps: steps.map((one) => stepOf(table(one))),
      carried: rest,
    },
  };
}

/** The word `advance_gate` takes for what is ticked, keeping the repository's own rule where the file named one. */
function gateWord(gate: Gate, was: unknown): string {
  if (gate.repository) return text(was)?.startsWith("manifest_rule:") ? (was as string) : "manifest_rule:review_gate";
  if (gate.you) return "human_always";
  return gate.judge ? "auto_if_judge_passes" : "auto";
}

function stepJson(step: Step): Table {
  const raw = step.carried ?? {};
  const was = stepOf(raw);
  const out: Table = { ...raw, id: step.id };
  if (out.label === undefined) out.label = step.id;
  if (out.delivers === undefined) out.delivers = false;

  if (step.evidence !== was.evidence) {
    const evidence = { ...table(raw.evidence) };
    if (step.evidence === "") delete evidence.submitted;
    else evidence.submitted = { ...table(evidence.submitted), type: step.evidence };
    if (Object.keys(evidence).length === 0) delete out.evidence;
    else out.evidence = evidence;
  }

  // A rule the repository decides holds its own checks and Judge, and the editor does not touch them.
  if (!step.gate.repository) {
    if (step.gate.checks !== was.gate.checks || step.check !== was.check) {
      const kept = list(raw.mechanical_checks);
      if (!step.gate.checks || step.check === "none") delete out.mechanical_checks;
      else out.mechanical_checks = [text(table(kept[0]).type) === step.check ? kept[0] : { type: step.check }, ...kept.slice(1)];
    }
    if (step.gate.judge !== was.gate.judge || step.judge !== was.judge) {
      const kept = list(raw.judge_checks).map(table);
      if (!step.gate.judge || step.judge.trim() === "") delete out.judge_checks;
      else {
        const first = kept[0] ?? {};
        const criteria = list(first.criteria).map(table);
        const lead = criteria[0] ?? {};
        criteria[0] = { criterion_id: `${step.id}_question`, ...lead, question: step.judge };
        out.judge_checks = [{ ...first, enabled: true, criteria }, ...kept.slice(1)];
      }
    }
  }
  const gate = step.gate;
  const gateChanged = gate.repository !== was.gate.repository || gate.you !== was.gate.you || gate.judge !== was.gate.judge;
  if (gateChanged || out.advance_gate === undefined) out.advance_gate = gateWord(gate, raw.advance_gate);

  if (step.retryLimit !== was.retryLimit) out.retry_limit = step.retryLimit;

  if (step.returnsTo !== was.returnsTo || step.iterationCap !== was.iterationCap) {
    if (step.returnsTo === "") {
      delete out.verdict_routing;
      delete out.iteration_cap;
    } else {
      out.verdict_routing = { ...table(raw.verdict_routing), request_changes: step.returnsTo };
      out.iteration_cap = step.iterationCap;
    }
  }
  return out;
}

/**
 * The definition as the text `save_workflow` takes. Version and name are the
 * loader's required keys, so a new workflow gets version 1 and its id for a name.
 */
export function writeDefinition(def: Definition): string {
  const { version, name, ...rest } = def.carried ?? {};
  return JSON.stringify(
    { version: version ?? 1, workflow_id: def.id, name: name ?? def.id, ...rest, steps: def.steps.map(stepJson) },
    null,
    2,
  );
}

/** What Fleet answered for one workflow it holds: the list row and the shadowed definitions beneath it. */
export type WorkflowRow = { id: string; source?: string; file?: string; overrides?: { source: string; file: string }[] };
/** A definition Fleet left out. */
export type LeftOutRow = { id?: string; source: string; file: string; said: string };

/**
 * The rows of the list from what Fleet answered: each workflow it runs, a row
 * for each definition it replaces, and each file it left out with its reason.
 * **A workflow with no `source` or `file` is from a Fleet older than the
 * fields, and draws nothing** rather than a row that cannot be opened.
 */
export function entriesOf(workflows: readonly WorkflowRow[], leftOut: readonly LeftOutRow[]): Entry[] {
  const out: Entry[] = [];
  for (const one of workflows) {
    const source = one.source === undefined ? undefined : SOURCE_OF_WIRE[one.source];
    if (source === undefined || one.file === undefined) continue;
    out.push({ key: `${source}/${one.id}`, id: one.id, source, file: one.file });
    for (const under of one.overrides ?? []) {
      const lower = SOURCE_OF_WIRE[under.source];
      if (lower !== undefined && SOURCE_RANK[lower] < SOURCE_RANK[source]) {
        out.push({ key: `${lower}/${one.id}`, id: one.id, source: lower, file: under.file });
      }
    }
  }
  for (const one of leftOut) {
    const source = SOURCE_OF_WIRE[one.source];
    if (source === undefined) continue;
    const named = one.id ?? one.file.split("/").pop() ?? one.file;
    out.push({ key: `left/${source}/${one.file}`, id: named, source, file: one.file, leftOut: one.said });
  }
  return out;
}
