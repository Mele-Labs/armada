// What the Workflows page makes of Fleet's answers: a step's words changed in a definition's text and nothing else, the
// list's rows with the definitions a more specific place replaced, and the box and cards read off the changes.

import { forRequestsOf, keyOf, stepsOfDefinition, withWords } from "@armada/components";
import type { ManagedStep } from "@armada/components";
import type { WorkflowChange } from "@armada/protocol";
import { describe, expect, test } from "vitest";

import { agentOf, driftOf, listedOf, stepOf, suggestionsOf } from "./workflow-manager-wire";

const FILE = {
  version: 1,
  workflow_id: "feature",
  name: "feature",
  for_requests: "adding something",
  lands: "pull_request",
  steps: [
    { id: "propose", label: "Propose", instructions: "Say it.", on_complete: { notify: true }, retry_limit: 2 },
    { id: "deliver", label: "Deliver", delivers: true, mechanical_checks: [{ type: "pr_merged" }] },
  ],
};

test("changing a step's words changes those words and no other field of the file", () => {
  const out = withWords(JSON.stringify(FILE), "propose", "Say it twice.");
  expect(JSON.parse(out!)).toEqual({ ...FILE, steps: [{ ...FILE.steps[0], instructions: "Say it twice." }, FILE.steps[1]] });
});

test("a step that had no words gets them, and the key order of the file is kept", () => {
  const out = withWords(JSON.stringify(FILE), "deliver", "Merge it.")!;
  expect(Object.keys(JSON.parse(out))).toEqual(Object.keys(FILE));
  expect((JSON.parse(out) as typeof FILE).steps[1]).toMatchObject({ instructions: "Merge it.", delivers: true });
});

test("a step the file lacks, or text that is not a workflow, changes nothing", () => {
  expect(withWords(JSON.stringify(FILE), "nope", "x")).toBeUndefined();
  expect(withWords("{ not json", "propose", "x")).toBeUndefined();
  expect(withWords(JSON.stringify({ workflow_id: "x" }), "propose", "x")).toBeUndefined();
});

test("a definition written as YAML is read, and goes back as JSON with its other fields", () => {
  const yaml = "version: 1\nworkflow_id: bug\nname: bug\nfor_requests: a break\nsteps:\n  - id: fix\n    label: Fix\n    instructions: Fix it.\n";
  expect(forRequestsOf(yaml)).toBe("a break");
  expect(stepsOfDefinition(yaml)).toEqual([{ id: "fix", label: "Fix", instructions: "Fix it." }]);
  expect(JSON.parse(withWords(yaml, "fix", "Fix it well.")!)).toMatchObject({ workflow_id: "bug", for_requests: "a break", steps: [{ id: "fix", instructions: "Fix it well." }] });
});

test("a step is drawn with the glyph its id carries, what ends it and when it is passed over", () => {
  expect(stepOf({ id: "walk", label: "Walk", instructions: "Show it.", ends_when: "you approve", passed_over_when: "nothing is seen" })).toEqual({
    id: "walk",
    label: "Walk",
    glyph: "walk",
    ends: "you approve",
    passedOver: "nothing is seen",
    words: "Show it.",
  });
  // An id with no glyph and no end draws neither, and no words are empty ones.
  expect(stepOf({ id: "odd", label: "Odd" })).toEqual({ id: "odd", label: "Odd", ends: "", words: "" });
});

test("nudges become drifted Jobs under their step, with how long ago", () => {
  const now = Date.parse("2026-10-27T12:00:00Z");
  const drift = driftOf(
    [
      { id: "walk", label: "Walk", nudges: [{ id: 4, job_id: "job-a", job_title: "Timeline", fact: "a test too early", at: "2026-10-27T10:00:00Z" }, { id: 5, job_id: "job-b", fact: "an edit too early", at: "2026-10-24T12:00:00Z" }] },
      { id: "deliver", label: "Deliver" },
    ],
    now,
  );
  expect(Object.keys(drift)).toEqual(["walk"]);
  expect(drift.walk).toEqual([
    { id: "4", jobId: "job-a", job: "Timeline", what: "a test too early", when: "2h ago" },
    { id: "5", jobId: "job-b", job: "job-b", what: "an edit too early", when: "3d ago" },
  ]);
});

test("the list has each workflow Fleet runs, the ones a more specific place replaced under it, and each file left out", () => {
  const summary = (id: string, source: string, overrides: { source: string; file: string }[] = []) => ({ id, name: id, version: 1, steps: [], manifest_id: "m", source, file: `${source}/${id}`, overrides });
  const rows = listedOf(
    [summary("feature", "kit", [{ source: "repository", file: "r" }, { source: "armada", file: "a" }]), summary("bug", "armada"), { ...summary("old", ""), source: undefined }],
    [{ id: "hotfix", source: "kit", file: "~/hotfix.json", said: "Names a check this repository does not declare" }, { source: "repository", file: ".armada/workflows/odd.json", said: "Not a workflow" }],
  );
  expect(rows).toEqual([
    { id: "feature", source: "yours", manifestId: "m" },
    { id: "feature", source: "repository", manifestId: "m" },
    { id: "feature", source: "armada", manifestId: "m" },
    { id: "bug", source: "armada", manifestId: "m" },
    { id: "hotfix", source: "yours", leftOut: { said: "Names a check this repository does not declare", file: "~/hotfix.json" } },
    { id: "odd.json", source: "repository", leftOut: { said: "Not a workflow", file: ".armada/workflows/odd.json" } },
  ]);
});

describe("the list across repositories", () => {
  const copy = (id: string, source: string, manifest: string) => ({ id, name: id, version: 1, steps: [], manifest_id: manifest, source, file: `${source}/${id}`, overrides: [] });
  const fleet = [
    copy("bug", "repository", "m-1"),
    copy("bug", "armada", "m-1"),
    copy("bug", "repository", "m-2"),
    copy("bug", "armada", "m-2"),
    copy("docs", "kit", "m-1"),
    copy("docs", "kit", "m-2"),
  ];

  test("on All, Armada's and Yours appear once and each repository's copy of one id stays apart", () => {
    const rows = listedOf(fleet, [], null);
    expect(rows.filter((one) => one.source === "armada")).toHaveLength(1);
    expect(rows.filter((one) => one.source === "yours")).toHaveLength(1);
    const repositories = rows.filter((one) => one.source === "repository");
    expect(repositories.map((one) => one.manifestId)).toEqual(["m-1", "m-2"]);
    expect(new Set(rows.map(keyOf)).size).toBe(rows.length);
    expect(keyOf(repositories[0]!)).not.toBe(keyOf(repositories[1]!));
  });

  test("with a repository picked, only its own copies are listed", () => {
    const rows = listedOf(fleet, [], "m-1");
    expect(rows.map((one) => `${one.source}:${one.id}:${one.manifestId}`)).toEqual(["repository:bug:m-1", "armada:bug:m-1", "yours:docs:m-1"]);
  });
});

const before = JSON.stringify(FILE);
const change = (over: Partial<WorkflowChange>): WorkflowChange => ({
  id: 1,
  workflow_id: "feature",
  source: "repository",
  origin: { kind: "asked", text: "add a step" },
  before,
  state: "drafting",
  at: "2026-10-27T12:00:00Z",
  ...over,
});
const KNOWN: ManagedStep[] = [{ id: "propose", label: "Propose", ends: "you reply", words: "Say it." }];

test("a change he asked for is the box: working, a draft with the steps before and after, or why none fit", () => {
  expect(agentOf([], KNOWN)).toEqual({ phase: "idle" });
  expect(agentOf([change({})], KNOWN)).toEqual({ phase: "working", asked: "add a step" });
  expect(agentOf([change({ state: "failed", reason: "Names a step twice" })], KNOWN)).toEqual({ phase: "failed", reason: "Names a step twice", asked: "add a step" });
  const after = JSON.stringify({ ...FILE, steps: [...FILE.steps, { id: "extra", label: "Extra", instructions: "More." }] });
  const proposed = agentOf([change({ state: "ready", after })], KNOWN);
  expect(proposed.phase).toBe("proposed");
  if (proposed.phase !== "proposed") return;
  expect(proposed.before.map((one) => one.id)).toEqual(["propose", "deliver"]);
  expect(proposed.after.map((one) => one.id)).toEqual(["propose", "deliver", "extra"]);
  // A step the workflow has keeps what Fleet read of it; one it lacks has no end to say.
  expect(proposed.after[0]).toMatchObject({ ends: "you reply", words: "Say it." });
  expect(proposed.after[2]).toMatchObject({ ends: "", words: "More." });
});

test("a change Armada can't make is the box saying so, with what is missing, and no steps to approve", () => {
  const cannot = change({ state: "cannot", missing: ["A step that ends on any of several approvals"] });
  expect(agentOf([cannot], KNOWN)).toEqual({ phase: "cannot", missing: ["A step that ends on any of several approvals"], asked: "add a step" });
  // Fleet may say nothing of what is missing; the box still says it cannot.
  expect(agentOf([change({ state: "cannot" })], KNOWN)).toEqual({ phase: "cannot", missing: [], asked: "add a step" });
  // Whatever drafted text it carries is not offered as a change.
  const after = JSON.stringify({ ...FILE, steps: [...FILE.steps, { id: "extra", label: "Extra" }] });
  expect(agentOf([change({ state: "cannot", missing: ["x"], after })], KNOWN).phase).toBe("cannot");
});

test("a draft's steps are Fleet's own reading of each side where it gives one, with what ends them and when they are passed over", () => {
  const after = JSON.stringify({ ...FILE, steps: [...FILE.steps, { id: "extra", label: "Extra", instructions: "More." }] });
  const proposed = agentOf(
    [
      change({
        state: "ready",
        after,
        before_steps: [{ id: "propose", label: "Propose", instructions: "Say it.", ends_when: "you reply" }],
        after_steps: [
          { id: "propose", label: "Propose", instructions: "Say it.", ends_when: "you reply" },
          { id: "extra", label: "Extra", instructions: "More.", ends_when: "any of two approvals", passed_over_when: "nothing is seen" },
        ],
      }),
    ],
    KNOWN,
  );
  expect(proposed).toEqual({
    phase: "proposed",
    asked: "add a step",
    before: [{ id: "propose", label: "Propose", glyph: "propose", ends: "you reply", words: "Say it." }],
    after: [
      { id: "propose", label: "Propose", glyph: "propose", ends: "you reply", words: "Say it." },
      { id: "extra", label: "Extra", ends: "any of two approvals", passedOver: "nothing is seen", words: "More." },
    ],
  });
});

test("a draft with no steps from Fleet is read from its text, and each side is read on its own", () => {
  const after = JSON.stringify({ ...FILE, steps: [...FILE.steps, { id: "extra", label: "Extra", instructions: "More." }] });
  const proposed = agentOf([change({ state: "ready", after, after_steps: [{ id: "only", label: "Only", ends_when: "done" }] })], KNOWN);
  if (proposed.phase !== "proposed") throw new Error("not proposed");
  // Fleet's reading of the after side, and the text for the before side it did not give.
  expect(proposed.after).toEqual([{ id: "only", label: "Only", ends: "done", words: "" }]);
  expect(proposed.before.map((one) => one.id)).toEqual(["propose", "deliver"]);
  expect(proposed.before[0]).toMatchObject({ ends: "you reply", words: "Say it." });
});

test("a change drawn from drift is a card on its step, or the step an agent is drawing one for, and is not the box", () => {
  const after = JSON.stringify({ ...FILE, steps: [{ ...FILE.steps[0], instructions: "Say it twice." }, FILE.steps[1]] });
  const drift: Partial<WorkflowChange> = { origin: { kind: "drift", nudges: [7, 8] }, step_id: "propose" };
  expect(agentOf([change({ ...drift, state: "ready", after })], KNOWN)).toEqual({ phase: "idle" });
  expect(suggestionsOf([change({ ...drift, state: "ready", after })], "feature")).toEqual({
    suggestions: { propose: { words: "Say it twice.", from: ["7", "8"] } },
    suggesting: null,
  });
  expect(suggestionsOf([change({ ...drift })], "feature")).toEqual({ suggestions: {}, suggesting: "feature/propose" });
  expect(suggestionsOf([change({ ...drift, state: "failed", reason: "Unchanged" })], "feature").suggestions.propose).toEqual({ words: "", from: ["7", "8"], failed: "Unchanged" });
});

test("a workflow a Job proposed is a fresh draft: approved or dismissed, never sent back", () => {
  const out = agentOf([change({ origin: { kind: "new" }, before: "", state: "ready", after: JSON.stringify(FILE) })], []);
  expect(out).toMatchObject({ phase: "proposed", fresh: true });
  expect(out).not.toHaveProperty("asked");
});
