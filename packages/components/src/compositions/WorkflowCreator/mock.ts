// What the mock Fleet answers the Workflow creator with. Invented to show every
// state the list has: a carried definition, one shadowed by Kit, one shadowed by
// the repository, a Kit file and a repository file of their own, and a Kit file
// set aside. The fixtures are written as definitions and handed over the way
// Fleet hands them: a row each, the left-out files, and each definition's text.
// Delete with the mock.

import { SOURCE_RANK, type Definition, type Entry, type Gate, type Step } from "./def";
import { SOURCE_OF_WIRE, WIRE_OF_SOURCE, writeDefinition, type LeftOutRow, type WorkflowRow } from "./json";

export const MOCK_REPOSITORY = "armada";

const none: Gate = { checks: false, judge: false, you: false, repository: false };

function step(id: string, over: Partial<Omit<Step, "id">> = {}): Step {
  return {
    id,
    evidence: "diff",
    check: "none",
    judge: "",
    gate: none,
    retryLimit: 3,
    returnsTo: "",
    iterationCap: 5,
    ...over,
  };
}


const MOCK_ENTRIES: readonly Entry[] = [
  { key: "carried/feature", id: "feature", source: "carried", file: "feature" },
  { key: "carried/bug", id: "bug", source: "carried", file: "bug" },
  { key: "carried/design_plan", id: "design_plan", source: "carried", file: "design_plan" },
  { key: "kit/design_plan", id: "design_plan", source: "kit", file: "design_plan.json" },
  { key: "kit/release_notes", id: "release_notes", source: "kit", file: "release_notes.json" },
  { key: "repository/bug", id: "bug", source: "repository", file: "bug.json" },
  { key: "repository/migration", id: "migration", source: "repository", file: "migration.json" },
  {
    key: "kit/hotfix",
    id: "hotfix",
    source: "kit",
    file: "hotfix.json",
    leftOut: "Names the check smoke, which this repository does not declare",
  },
];

/** The definitions behind each editable row, by key. A carried one is edited as a copy. */
const MOCK_DEFINITIONS: Readonly<Record<string, Definition>> = {
  "carried/feature": {
    id: "feature",
    scope: "kit",
    steps: [
      step("plan", { evidence: "document", check: "plan_recorded", gate: { ...none, checks: true } }),
      step("implement", {
        check: "every_manifest_check",
        judge: "Does this diff do what the plan says, and nothing else?",
        gate: { ...none, checks: true, judge: true },
      }),
      step("tests", { evidence: "test_suite_run", check: "every_manifest_check", gate: { ...none, checks: true } }),
      step("handoff", { evidence: "bundle", gate: { ...none, you: true } }),
    ],
  },
  "carried/bug": {
    id: "bug",
    scope: "kit",
    steps: [
      step("repro", {
        evidence: "failing_test",
        check: "test_run",
        judge: "Does this failing test actually represent the reported bug?",
        gate: { ...none, checks: true, judge: true },
      }),
      step("fix", {
        check: "diff_nonempty",
        judge: "Does this diff address the stated root cause, not just the symptom?",
        gate: { ...none, checks: true, judge: true },
      }),
      step("review", { evidence: "bundle", gate: { ...none, repository: true }, returnsTo: "fix", iterationCap: 4 }),
    ],
  },
  "carried/design_plan": {
    id: "design_plan",
    scope: "kit",
    steps: [
      step("draft", { evidence: "document", check: "artifact_exists" }),
      step("feedback", { evidence: "document", gate: { ...none, you: true }, returnsTo: "draft", iterationCap: 3 }),
    ],
  },
  "kit/design_plan": {
    id: "design_plan",
    scope: "kit",
    steps: [
      step("draft", { evidence: "document", check: "artifact_exists" }),
      step("feedback", { evidence: "document", gate: { ...none, you: true }, returnsTo: "draft", iterationCap: 3 }),
    ],
  },
  "kit/release_notes": {
    id: "release_notes",
    scope: "kit",
    steps: [
      step("gather", { evidence: "facts_note", check: "artifact_exists", gate: { ...none, checks: true } }),
      step("write", {
        evidence: "document",
        judge: "Does every change in the notes appear in the merged pull requests?",
        gate: { ...none, judge: true },
      }),
      step("publish", { evidence: "bundle", gate: { ...none, you: true } }),
    ],
  },
  "repository/bug": {
    id: "bug",
    scope: MOCK_REPOSITORY,
    steps: [
      step("repro", {
        evidence: "failing_test",
        check: "test_run",
        judge: "Does this failing test actually represent the reported bug?",
        gate: { ...none, checks: true, judge: true },
      }),
      step("fix", {
        check: "every_manifest_check",
        judge: "Does this diff address the stated root cause, not just the symptom?",
        gate: { ...none, checks: true, judge: true },
        retryLimit: 2,
      }),
      step("review", { evidence: "bundle", gate: { ...none, you: true }, returnsTo: "fix", iterationCap: 4 }),
    ],
  },
  "repository/migration": {
    id: "migration",
    scope: MOCK_REPOSITORY,
    steps: [
      step("schema", { check: "manifest_check", gate: { ...none, checks: true } }),
      step("backfill", { evidence: "test_suite_run", check: "manifest_check", gate: { ...none, checks: true, you: true } }),
    ],
  },
};

/** One file the mock Fleet holds: the place as Fleet spells it, its id, its path, its text, and why it was left out. */
export type MockFile = { source: string; id: string; file: string; text: string; leftOut?: string };

/** Fleet's own spelling of a file: a bracketed name where Armada carries it. */
const pathOf = (one: Entry) => (one.source === "carried" ? `[${one.file}]` : one.file);

export const MOCK_FILES: readonly MockFile[] = MOCK_ENTRIES.map((one) => ({
  source: WIRE_OF_SOURCE[one.source],
  id: one.id,
  file: pathOf(one),
  text: one.leftOut === undefined ? writeDefinition(MOCK_DEFINITIONS[one.key]!) : "",
  ...(one.leftOut === undefined ? {} : { leftOut: one.leftOut }),
}));

/** `GET /workflows`: the file that runs for each id, and the ones it replaces. */
export function workflowRowsOf(files: readonly MockFile[]): WorkflowRow[] {
  const rank = (one: MockFile) => SOURCE_RANK[SOURCE_OF_WIRE[one.source] ?? "carried"];
  const runs = files.filter((one) => one.leftOut === undefined);
  return runs
    .filter((one) => runs.every((other) => other.id !== one.id || rank(other) <= rank(one)))
    .map((one) => {
      const under = runs.filter((other) => other.id === one.id && rank(other) < rank(one));
      return {
        id: one.id,
        source: one.source,
        file: one.file,
        ...(under.length === 0 ? {} : { overrides: under.map((other) => ({ source: other.source, file: other.file })) }),
      };
    });
}

/** `GET /workflows/left_out`. */
export function leftOutRowsOf(files: readonly MockFile[]): LeftOutRow[] {
  return files.flatMap((one) => (one.leftOut === undefined ? [] : [{ id: one.id, source: one.source, file: one.file, said: one.leftOut }]));
}
