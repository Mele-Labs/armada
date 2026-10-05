// The list and the definitions the Workflow creator mock opens on. Invented to
// show every state the list has: a carried definition, one shadowed by Kit, one
// shadowed by the repository, a Kit file and a repository file of their own,
// and a Kit file set aside. Delete with the mock.

import type { Definition, Entry, Gate, Step } from "./def";

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
    ...over,
  };
}

export const MOCK_REPOSITORY = "armada";

export const MOCK_ENTRIES: readonly Entry[] = [
  { key: "carried/feature", id: "feature", source: "carried", file: "feature", structure: "linear" },
  { key: "carried/bug", id: "bug", source: "carried", file: "bug", structure: "loop" },
  { key: "carried/design_plan", id: "design_plan", source: "carried", file: "design_plan", structure: "loop" },
  { key: "kit/design_plan", id: "design_plan", source: "kit", file: "design_plan.json", structure: "loop" },
  { key: "kit/release_notes", id: "release_notes", source: "kit", file: "release_notes.json", structure: "linear" },
  { key: "repository/bug", id: "bug", source: "repository", file: "bug.json", structure: "loop" },
  { key: "repository/migration", id: "migration", source: "repository", file: "migration.json", structure: "linear" },
  {
    key: "kit/hotfix",
    id: "hotfix",
    source: "kit",
    file: "hotfix.json",
    structure: "linear",
    leftOut: "Names the check smoke, which this repository does not declare",
  },
];

/** The definitions behind each editable row, by key. A carried one is edited as a copy. */
export const MOCK_DEFINITIONS: Readonly<Record<string, Definition>> = {
  "carried/feature": {
    id: "feature",
    structure: "linear",
    scope: "kit",
    iterationCap: 5,
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
    structure: "loop",
    scope: "kit",
    iterationCap: 5,
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
      step("review", { evidence: "bundle", gate: { ...none, repository: true }, returnsTo: "fix" }),
    ],
  },
  "carried/design_plan": {
    id: "design_plan",
    structure: "loop",
    scope: "kit",
    iterationCap: 5,
    steps: [
      step("draft", { evidence: "document", check: "artifact_exists" }),
      step("feedback", { evidence: "document", gate: { ...none, you: true }, returnsTo: "draft" }),
    ],
  },
  "kit/design_plan": {
    id: "design_plan",
    structure: "loop",
    scope: "kit",
    iterationCap: 3,
    steps: [
      step("draft", { evidence: "document", check: "artifact_exists" }),
      step("feedback", { evidence: "document", gate: { ...none, you: true }, returnsTo: "draft" }),
    ],
  },
  "kit/release_notes": {
    id: "release_notes",
    structure: "linear",
    scope: "kit",
    iterationCap: 5,
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
    structure: "loop",
    scope: "repository",
    iterationCap: 4,
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
      step("review", { evidence: "bundle", gate: { ...none, you: true }, returnsTo: "fix" }),
    ],
  },
  "repository/migration": {
    id: "migration",
    structure: "linear",
    scope: "repository",
    iterationCap: 5,
    steps: [
      step("schema", { check: "manifest_check", gate: { ...none, checks: true } }),
      step("backfill", { evidence: "test_suite_run", check: "manifest_check", gate: { ...none, checks: true, you: true } }),
    ],
  },
};
