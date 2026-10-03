// A Job at its approval gate whose request linked an issue, as Fleet serves it
// at 23.8: two criteria read from the issue, which has been edited since Fleet
// read it, one from the prompt, and the repository's branches.
//
// **The wire's own shapes, field for field** (`crates/ipc/src/detail.rs`,
// `Criterion::of`): an issue's line carries `origin` and `origin_moved_at`, a
// prompt's carries its `origin` alone, and `landing`, `drone_cap` and
// `approved_at` are absent until the press.

import type { Branches, Criterion } from "@armada/protocol";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";
import { watchedRead } from "@armada/screens/src/fixtures/build/base";

import { refactorAtApproval } from "./job-detail-fixtures";

/** The issue the request linked, as `IssueAddress` reads it and the forge addresses it. */
const ISSUE = { kind: "issue", ref: "armada#1162", url: "https://github.com/NickMele/armada/issues/1162" };

/** When the issue was edited, after Fleet read it at the proposal. */
export const ISSUE_MOVED_AT = "2026-10-02T15:41:09Z";

const CRITERIA: Criterion[] = [
  {
    criterion_id: "c1",
    text: "Guide 8 is removed from the catalogue",
    source: "judge",
    origin: ISSUE,
    origin_moved_at: ISSUE_MOVED_AT,
  },
  {
    criterion_id: "c2",
    text: "A validation rule prevents guides without drawn pieces",
    source: "judge",
    origin: ISSUE,
    origin_moved_at: ISSUE_MOVED_AT,
  },
  {
    criterion_id: "c3",
    text: "The guide catalogue still opens on guide 1",
    source: "judge",
    origin: { kind: "prompt" },
  },
];

/** `GET /manifest/branches`: the base first, the rest by name. */
const BRANCHES: Branches = {
  branches: [
    { name: "main", base: true },
    { name: "armada/1-retire-guide-8-and-add-guide-validation-ru", base: false },
    { name: "release/2026-10", base: false },
  ],
};

export function proposalFromAnIssue(): JobFixture {
  const base = refactorAtApproval();
  if (base.watched.state !== "read") return base;
  // A person dispatched it, with the issue's address in the request.
  const job = { ...base.job, origin: "manual" };
  const detail = {
    ...base.watched.detail,
    job,
    facts:
      "Retire guide 8 and add a validation rule so a guide without drawn pieces cannot ship. " +
      "https://github.com/NickMele/armada/issues/1162",
    acceptance_criteria: CRITERIA,
  };
  return {
    ...base,
    name: "awaiting_approval — a proposal read from an issue that has moved since",
    job,
    watched: watchedRead(detail),
    branches: BRANCHES,
  };
}
