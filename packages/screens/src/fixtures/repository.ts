// The repository every mock window is a moment of, and the workflow it holds.
//
// **A made-up home**, since a real one would name a person. Fleet keeps a repository's records in a
// folder of its own, off the checkout.

import type { DeclaredCheck, ManifestSummary, RepositorySummary, WorkflowSummary } from "@armada/protocol";

export const WORKFLOW_ID = "bug";
export const MANIFEST_ID = "01M1CNPKTV0018H2M1CXDNBK06";
/** What that Manifest is *called* — the repository Fleet read it from, and what a surface draws. */
export const MANIFEST_NAME = "armada";

/** The instant the mock's clock is moved from. */
export const NOW = Date.parse("2026-09-10T14:31:00Z");

/** The Manifest Check the Fix and consumer steps declare. */
const BUILD_CHECK: DeclaredCheck = {
  kind: "manifest_check",
  name: "cargo_build",
  run: "cargo build --workspace --locked",
  expect_exit_code: 0,
};

/** The regression suite Regression check declares, and Fix does not. */
const NEXTEST_CHECK: DeclaredCheck = {
  kind: "manifest_check",
  name: "cargo_nextest",
  run: "cargo nextest run --workspace",
  expect_exit_code: 0,
};

/** `WorkflowSummary` for `bug`, six steps. */
export function workflow(): WorkflowSummary {
  return {
    id: WORKFLOW_ID,
    name: "Bug",
    version: 3,
    manifest_id: MANIFEST_ID,
    steps: [
      {
          step_id: "repro",
          label: "Reproduction",
          checks: [],
          judge_checks: [],
          advance_gate: "auto",
          delivers: false,
          phase: "work",
        },
      {
          step_id: "root_cause",
          label: "Root cause",
          checks: [],
          judge_checks: [],
          advance_gate: "auto",
          delivers: false,
          phase: "work",
        },
      {
          step_id: "fix",
          label: "Fix",
          checks: [BUILD_CHECK],
          judge_checks: [],
          advance_gate: "auto_if_judge_passes",
          delivers: false,
          phase: "work",
        },
      {
          step_id: "regression_verify",
          label: "Regression check",
          checks: [NEXTEST_CHECK],
          judge_checks: [{ criteria: 2 }],
          advance_gate: "human_always",
          delivers: false,
          phase: "work",
        },
      {
          step_id: "consumers",
          label: "Check the consumers still compile",
          checks: [BUILD_CHECK],
          judge_checks: [],
          advance_gate: "auto",
          delivers: false,
          phase: "work",
        },
      {
          step_id: "land",
          label: "Land",
          checks: [],
          judge_checks: [],
          advance_gate: "auto",
          delivers: true,
          phase: "delivery",
        },
    ],
  };
}

/** The repository `manifest()` was read from, as `GET /repositories` lists it. */
export function repository(): RepositorySummary {
  return { root: "/Users/user/armada", records_root: manifest().records_root, manifest: manifest() };
}

export function manifest(): ManifestSummary {
  return {
    id: MANIFEST_ID,
    repository: MANIFEST_NAME,
    // A bare `armada.yml` has no directory component, and `repoOf` cuts at the
    // manifest's own last `/`. A workspace-relative path is what it is built to read.
    path: "armada/armada.yml",
    records_root: "/Users/user/Library/Application Support/Armada/records/armada",
    version: 3,
    checks: ["cargo_build", "cargo_nextest"],
    // What this repository's own `armada.yml` declares, which is what the mock Fleet's Manifest
    // surface serves for it too — `DECLARED` in `apps/desktop/.../mock/manifest-fleet.ts`.
    auto_merge: "never",
    review_gate: "human_always",
  };
}
