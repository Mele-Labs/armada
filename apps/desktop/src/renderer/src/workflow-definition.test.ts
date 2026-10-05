// A definition's text as the editor's model and back. The claim that matters:
// a definition opened and saved untouched goes out as it came in, so a save
// never drops what the editor does not draw; and what the editor does change
// is the only thing that moves.

import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import { refusalsOf } from "@armada/components/src/compositions/WorkflowCreator/def";
import { entriesOf, parse as load, readDefinition, writeDefinition } from "@armada/components/src/compositions/WorkflowCreator/json";

const CARRIED = join(import.meta.dirname, "../../../../../.armada/workflows");
const files = readdirSync(CARRIED).map((name) => [name, readFileSync(join(CARRIED, name), "utf8")] as const);

function opened(text: string, from: "carried" | "kit" | "repository" = "carried") {
  const read = readDefinition(text, from, "01M1CNPKTV0018H2M1CXDNBK06");
  if (!read.ok) throw new Error(read.said);
  return read.def;
}

const parsed = (text: string) => JSON.parse(text) as Record<string, unknown>;

describe("a carried definition opened and written untouched", () => {
  it.each(files)("%s comes back as it was read, comments aside", (_name, text) => {
    expect(parsed(writeDefinition(opened(text)))).toEqual(load(text));
  });
});

describe("what the editor changes is all that moves", () => {
  const feature = files.find(([name]) => name === "feature.json")![1];
  const original = load(feature) as { steps: Record<string, unknown>[] };

  it("changes one step's retries and nothing else", () => {
    const def = opened(feature);
    def.steps[0]!.retryLimit = 7;
    const out = parsed(writeDefinition(def)) as { steps: Record<string, unknown>[] };
    expect(out.steps[0]).toEqual({ ...original.steps[0], retry_limit: 7 });
    expect(out.steps.slice(1)).toEqual(original.steps.slice(1));
  });

  it("unticking the Judge drops its checks and moves the gate off the Judge", () => {
    const def = opened(feature);
    const at = def.steps.findIndex((one) => one.gate.judge && !one.gate.you && !one.gate.repository);
    expect(at).toBeGreaterThanOrEqual(0);
    def.steps[at]!.gate = { ...def.steps[at]!.gate, judge: false };
    const step = (parsed(writeDefinition(def)) as { steps: Record<string, unknown>[] }).steps[at]!;
    expect(step.judge_checks).toBeUndefined();
    expect(step.advance_gate).toBe("auto");
  });

  it("sends a step back through verdict_routing and the cap beside it, and takes both away together", () => {
    const def = opened(feature);
    const last = def.steps.length - 1;
    def.steps[last]!.returnsTo = def.steps[0]!.id;
    def.steps[last]!.iterationCap = 2;
    const out = parsed(writeDefinition(def)) as { steps: Record<string, unknown>[] };
    expect(out.steps[last]).toMatchObject({ verdict_routing: { request_changes: def.steps[0]!.id }, iteration_cap: 2 });

    def.steps[last]!.returnsTo = "";
    const gone = (parsed(writeDefinition(def)) as { steps: Record<string, unknown>[] }).steps[last]!;
    expect(gone).not.toHaveProperty("verdict_routing");
    expect(gone).not.toHaveProperty("iteration_cap");
  });

  it("never writes a structure key", () => {
    const out = parsed(writeDefinition(opened(feature))) as { steps: object[] };
    expect(out).not.toHaveProperty("structure");
    for (const step of out.steps) expect(step).not.toHaveProperty("structure");
  });

  it("keeps a rule the repository decides when the gate stays on it", () => {
    const bug = files.find(([name]) => name === "bug.json")![1];
    const def = opened(bug);
    const at = def.steps.findIndex((one) => one.gate.repository);
    if (at === -1) return;
    def.steps[at]!.retryLimit = 1;
    const step = (parsed(writeDefinition(def)) as { steps: Record<string, unknown>[] }).steps[at]!;
    expect(String(step.advance_gate)).toMatch(/^manifest_rule:/);
  });
});

describe("a new workflow", () => {
  it("carries every key the loader requires, and no cap without a back edge", () => {
    const def = opened(JSON.stringify({ workflow_id: "", steps: [{ id: "" }] }));
    def.id = "notes";
    def.steps[0]!.id = "write";
    def.steps[0]!.evidence = "document";
    const out = parsed(writeDefinition(def)) as { steps: Record<string, unknown>[] };
    expect(out).toMatchObject({ version: 1, workflow_id: "notes", name: "notes" });
    expect(out.steps[0]).toMatchObject({ id: "write", label: "write", delivers: false, advance_gate: "auto", evidence: { submitted: { type: "document" } } });
    expect(out.steps[0]).not.toHaveProperty("iteration_cap");
  });

  it("is refused here only for what the loader refuses on the shape alone", () => {
    const def = opened(JSON.stringify({ workflow_id: "", steps: [{ id: "a" }, { id: "a" }] }));
    expect(refusalsOf(def).map((one) => one.where)).toEqual(["id", "steps[2].id"]);
    def.id = "Not A Lowercase Name";
    expect(refusalsOf(def).map((one) => one.where)).toEqual(["steps[2].id"]);
  });
});

describe("the list from what Fleet answered", () => {
  it("draws a row for the one that runs, one for each it replaces, and the left-out files with their reason", () => {
    const rows = entriesOf(
      [
        { id: "bug", source: "repository", file: "/r/.armada/workflows/bug.json", overrides: [{ source: "armada", file: "[bug]" }, { source: "kit", file: "/k/bug.json" }] },
        { id: "feature", source: "armada", file: "[feature]" },
      ],
      [{ source: "kit", file: "/k/hotfix.json", said: "Names the check smoke" }],
    );
    expect(rows.map((one) => [one.key, one.source])).toEqual([
      ["repository/bug", "repository"],
      ["carried/bug", "carried"],
      ["kit/bug", "kit"],
      ["carried/feature", "carried"],
      ["left/kit//k/hotfix.json", "kit"],
    ]);
    expect(rows.at(-1)).toMatchObject({ id: "hotfix.json", leftOut: "Names the check smoke" });
  });

  it("draws nothing for a Fleet that does not say where a workflow came from", () => {
    expect(entriesOf([{ id: "bug" }, { id: "feature", source: "armada" }], [])).toEqual([]);
  });
});
