// What the mock Fleet answers the Workflow creator with. Invented to show every
// state the list has: a carried definition, one shadowed by Kit, one shadowed by
// the repository, a Kit file and a repository file of their own, and a Kit file
// set aside. The fixtures are written as definitions and handed over the way
// Fleet hands them: a row each, the left-out files, and each definition's text.
// Delete with the mock.

import { resolve, type Definition, type Entry, type Gate, type Step } from "./def";
import { writeDefinition, type LeftOutRow, type WorkflowRow, WIRE_OF_SOURCE } from "./json";

export const MOCK_REPOSITORY = "armada";

/** The Manifests a definition may be written to, beside Kit. */
export const MOCK_MANIFESTS: readonly string[] = [MOCK_REPOSITORY, "ledger", "site"];

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
      step("review", { evidence: "bundle", gate: { ...none, you: true } }),
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

/** Fleet's own spelling of a file: a bracketed name where Armada carries it. */
const fileOf = (one: Entry) => (one.source === "carried" ? `[${one.file}]` : one.file);

/** `GET /workflows`: the definition that runs for each id, and the ones it replaces. */
export const MOCK_WORKFLOWS: readonly WorkflowRow[] = (() => {
  const resolved = resolve(MOCK_ENTRIES);
  return resolved
    .filter((one) => one.leftOut === undefined && one.overriddenBy === undefined)
    .map((one) => {
      const under = resolved.filter((other) => other.id === one.id && other.overriddenBy !== undefined);
      return {
        id: one.id,
        source: WIRE_OF_SOURCE[one.source],
        file: fileOf(one),
        ...(under.length === 0 ? {} : { overrides: under.map((other) => ({ source: WIRE_OF_SOURCE[other.source], file: fileOf(other) })) }),
      };
    });
})();

/** `GET /workflows/left_out`. */
export const MOCK_LEFT_OUT: readonly LeftOutRow[] = MOCK_ENTRIES.filter((one) => one.leftOut !== undefined).map((one) => ({
  id: one.id,
  source: WIRE_OF_SOURCE[one.source],
  file: one.file,
  said: one.leftOut ?? "",
}));

/** `GET /workflows/definition`: each definition's text, by `<source>/<id>` with the source as Fleet spells it. */
export const MOCK_DEFINITION_TEXT: Readonly<Record<string, string>> = Object.fromEntries(
  MOCK_ENTRIES.filter((one) => one.leftOut === undefined).map((one) => [
    `${WIRE_OF_SOURCE[one.source]}/${one.id}`,
    writeDefinition(MOCK_DEFINITIONS[one.key]!),
  ]),
);
